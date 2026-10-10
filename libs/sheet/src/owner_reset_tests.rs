use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p={pad}")]
#[case("_before={{p:pad}}")]
#[case(concat!("selectors={{'& > span':{", "p:pad", "}}}"))]
#[case(concat!("selectors={{'& + span':{", "p:pad", "}}}"))]
#[case(concat!("_media={{'(min-width:800px)':{", "p:pad", "}}}"))]
#[case("p={[null,pad]}")]
#[case("_hover={{p:pad}}")]
#[case("_themeDark={{p:pad}}")]
#[case("_groupHover={{p:pad}}")]
#[case(concat!("_hover={enabled ? {", "p:pad", "} : {p:'8px'}}"))]
#[case(concat!("p=\"8px\" {...{...{", "p:pad", "}}}"))]
#[case("p={pad ?? '8px'}")]
#[serial]
fn owner_reset_is_plain_when_consumer_is_conditional_or_another_subject(#[case] props: &str) {
    for atom_mode in [false, true] {
        for single_css in [false, true] {
            for shared in [false, true] {
                // Given
                css::atom_hoist::restore_atom_plan(None);
                css::atom_hoist::set_atom_hoist(atom_mode.then_some(2));
                css::file_map::reset_file_map();
                css::file_map::reset_canonical_map();
                css::class_map::reset_class_map();
                css::set_prefix(None);
                css::file_routes::set_file_routes(std::collections::HashMap::from([(
                    "owner.tsx".into(),
                    if shared {
                        std::collections::HashSet::from([0, 1])
                    } else {
                        std::collections::HashSet::from([0])
                    },
                )]));
                let source = format!(
                    "import {{Box}} from '@devup-ui/react'; export const View=({{pad,enabled}})=><Box {props}/>;"
                );
                let output = extractor::extract(
                    "owner.tsx",
                    &source,
                    extractor::ExtractOption {
                        single_css,
                        ..extractor::ExtractOption::default()
                    },
                )
                .unwrap_or_else(|error| panic!("{error}"));
                let mut sheet = StyleSheet::default();
                // When
                let first = sheet.update_styles(&output.styles, "owner.tsx", single_css);
                let repeated = sheet.update_styles(&output.styles, "owner.tsx", single_css);
                // Then
                let mut variables = 0;
                for style in &output.styles {
                    if let ExtractStyleValue::Dynamic(dynamic) = style {
                        variables += 1;
                        let StyleProperty::Variable {
                            class_name,
                            variable_name,
                            ..
                        } = dynamic.extract((!single_css).then_some("owner.tsx"))
                        else {
                            panic!("{dynamic:?}")
                        };
                        assert!(output.code.contains(&class_name), "{}", output.code);
                        assert!(output.code.contains(&variable_name), "{}", output.code);
                        if props == "p={pad ?? '8px'}" {
                            assert!(
                                output.styles.iter().any(|style| matches!(style,
                                    ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px")),
                                "{}\n{:?}",
                                output.code,
                                output.styles
                            );
                        }
                        let resets: Vec<_> = sheet
                            .properties
                            .values()
                            .flat_map(|orders| orders.iter())
                            .flat_map(|(_, levels)| levels.iter())
                            .flat_map(|(level, properties)| {
                                properties.iter().map(move |prop| (*level, prop))
                            })
                            .filter(|(_, prop)| prop.property == variable_name)
                            .collect();
                        assert_eq!(resets.len(), 1, "{props}: {resets:?}");
                        let (level, reset) = resets[0];
                        assert_eq!(level, 0);
                        assert_eq!(reset.selector, None);
                        assert_eq!(reset.layer, None);
                        assert_eq!(reset.class_name, class_name);
                        assert_eq!(reset.value, "initial");
                        let filename = if single_css || (atom_mode && shared) {
                            None
                        } else {
                            Some("owner.tsx")
                        };
                        let emitted = sheet.create_css(filename, false);
                        let serialized = serde_json::to_string(&sheet.export_snapshot())
                            .unwrap_or_else(|error| panic!("{error}"));
                        let restored: StyleSheet = serde_json::from_str(&serialized)
                            .unwrap_or_else(|error| panic!("{error}"));
                        assert_eq!(restored.create_css(filename, false), emitted);
                        assert!(emitted.contains(&format!(".{class_name}{{")), "{emitted}");
                        assert_eq!(
                            emitted.matches(&format!("{variable_name}:initial")).count(),
                            1,
                            "{emitted}"
                        );
                        assert!(
                            emitted.contains(&format!("var({variable_name})")),
                            "{emitted}"
                        );
                        if dynamic.selector().is_none() && dynamic.level() == 0 {
                            assert_eq!(
                                emitted.matches(&format!(".{class_name}{{")).count(),
                                1,
                                "{emitted}"
                            );
                        }
                    }
                }
                assert!(variables > 0, "{}\n{:?}", output.code, output.styles);
                assert_eq!(first, Ok((true, atom_mode && (single_css || shared))));
                assert_eq!(repeated, Ok((false, false)));
            }
        }
    }
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    css::file_routes::reset_file_routes();
}

#[test]
#[serial]
fn owner_reset_leaves_theme_and_manual_variables_inherited() {
    // Given
    css::atom_hoist::restore_atom_plan(None);
    css::atom_hoist::set_atom_hoist(None);
    css::file_map::reset_canonical_map();
    let source = "import {Box} from '@devup-ui/react'; export const View=({pad})=><Box p={pad} color=\"$brand\" bg=\"var(--manual)\"/>;";
    let output = extractor::extract("owner.tsx", source, extractor::ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    let mut sheet = StyleSheet::default();
    // When
    sheet
        .update_styles(&output.styles, "owner.tsx", true)
        .unwrap_or_else(|error| panic!("{error}"));
    // Then
    let emitted = sheet.create_css(None, false);
    assert!(emitted.contains("color:var(--brand)"), "{emitted}");
    assert!(emitted.contains("background:var(--manual)"), "{emitted}");
    assert_eq!(emitted.matches(":initial").count(), 1, "{emitted}");
    assert!(!emitted.contains("--brand:initial"));
    assert!(!emitted.contains("--manual:initial"));
}
