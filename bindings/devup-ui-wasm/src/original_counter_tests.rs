use super::*;
use rstest::rstest;
use serial_test::serial;
mod cache;
mod controls;
mod environments;

fn setup() {
    reset_build_state_internal();
    css::debug::set_debug(false);
    seed_file_map(vec!["a.tsx".into(), "child.tsx".into()]);
    set_canonical_map(HashMap::from([("child.tsx".into(), "a.tsx".into())]));
}

fn compile(file: &str, source: &str) -> Output {
    code_extract_internal(
        file,
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

fn delivered() -> String {
    with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false))
}

#[rstest]
#[case("import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;")]
#[case("import {css} from '@devup-ui/react';export const x=css({color:'red'});")]
#[case(
    "import * as stylex from '@stylexjs/stylex';export const x=stylex.create({red:{color:'red'}});"
)]
#[serial]
fn single_atom_uses_each_original_first_slot(#[case] source: &str) {
    // Given
    setup();
    // When
    let outputs: Vec<_> = ["child.tsx", "a.tsx"]
        .into_iter()
        .map(|file| compile(file, source))
        .collect();
    // Then
    for (output, name) in outputs.iter().zip(["b-a", "a-a"]) {
        assert!(output.code().contains(name), "{}", output.code());
        assert!(delivered().contains(&format!(".{name}{{color:red}}")));
    }
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 2);
        assert!(map.values().all(|slots| slots.len() == 1));
    });
    with_style_sheet(|sheet| assert_eq!(sheet.names.len(), 0));
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn collapsed_literals_keep_original_first_slots_when_source_order_reverses(#[case] reverse: bool) {
    // Given: two numbered originals deliver into the same canonical sheet.
    setup();
    let mut inputs = [("a.tsx", "red", "a"), ("child.tsx", "blue", "b")];
    if reverse {
        inputs.reverse();
    }
    // When: each literal is compiled through the real binding pipeline.
    for (file, color, owner) in inputs {
        let source = format!(
            "import {{Box}} from '@devup-ui/react';export const x=<Box color='{color}' p={{4}}/>;"
        );
        let output = compile(file, &source);
        // Then: names, canonical delivery, allocator and registry agree.
        assert!(
            output
                .code()
                .contains(&format!("className=\"{owner}-a {owner}-b\"")),
            "{}",
            output.code()
        );
        assert!(output.code().contains("df/devup-ui-0.css"));
        assert_eq!(output.css_file(), Some("df/devup-ui-0.css".into()));
        let css = delivered();
        assert!(
            css.contains(&format!(".{owner}-a{{color:{color}}}")),
            "{css}"
        );
        assert!(
            css.contains(&format!(".{owner}-b{{padding:16px}}")),
            "{css}"
        );
    }
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 2);
        for id in ["D9-0", "D9-1"] {
            let slots = map
                .get(id)
                .unwrap_or_else(|| panic!("missing {id}: {map:?}"));
            assert_eq!(slots.len(), 2);
            assert_eq!(
                slots
                    .values()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>(),
                [0, 1].into_iter().collect()
            );
        }
    });
    with_style_sheet(|sheet| {
        assert_eq!(sheet.names.len(), 0);
        assert_eq!(sheet.properties.len(), 1);
    });
    reset_build_state_internal();
}

#[test]
#[serial]
fn equal_content_survives_deferred_union_when_original_scopes_have_ended() {
    // Given: equal declarations captured in independent raw-source scopes.
    setup();
    let source = "import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;";
    let outputs: Vec<_> = ["a.tsx", "child.tsx"]
        .into_iter()
        .map(|file| {
            extract_without_source_map(
                file,
                source,
                ExtractOption {
                    css_dir: "df".into(),
                    ..Default::default()
                },
            )
            .unwrap_or_else(|error| panic!("{error}"))
        })
        .collect();
    let union: FxHashSet<_> = outputs
        .iter()
        .flat_map(|output| output.styles.iter().cloned())
        .collect();
    // When: the union is emitted only after both extraction scopes have ended.
    let mut sheet = StyleSheet::default();
    sheet
        .update_styles(&union, "a.tsx", false)
        .unwrap_or_else(|error| panic!("{error}"));
    // Then: neither owner's reference was discarded and emission uses existing slots.
    assert_eq!(union.len(), 2);
    let css = sheet.create_css(Some("a.tsx"), false);
    for (output, name) in outputs.iter().zip(["a-a", "b-a"]) {
        assert!(output.code.contains(name), "{}", output.code);
        assert!(css.contains(&format!(".{name}{{color:red}}")), "{css}");
    }
    assert_eq!(sheet.names.len(), 0);
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 2);
        assert!(map.values().all(|slots| slots.len() == 1));
    });
    reset_build_state_internal();
}

#[test]
#[serial]
fn dynamic_site_owner_names_the_class_and_delivered_reset_without_registry_claims() {
    // Given
    setup();
    let source = "import {Box} from '@devup-ui/react';export const x=(tone)=><Box color={tone}/>;";
    // When
    let output = compile("child.tsx", source);
    // Then
    assert!(output.code().contains("b-a"), "{}", output.code());
    let css = delivered();
    let variable = css::Site {
        file: css::sparse_site::SourceFile::D9(1),
        at: source
            .find("tone}/>")
            .unwrap_or_else(|| panic!("missing assignment")),
        role: 0,
    }
    .variable_name("");
    assert!(output.code().contains(&variable), "{}", output.code());
    assert!(
        css.contains(&format!(".b-a{{{variable}:initial;color:var({variable})}}")),
        "{css}"
    );
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 1);
        assert_eq!(map["D9-1"].len(), 1);
    });
    with_style_sheet(|sheet| assert_eq!(sheet.names.len(), 0));
    reset_build_state_internal();
}

#[test]
#[serial]
fn conditional_typography_constructor_keeps_each_original_counter() {
    // Given
    setup();
    register_theme_internal(
        serde_json::from_value(serde_json::json!({"typography":{"body":{"fontSize":"14px"}}}))
            .unwrap_or_else(|error| panic!("{error}")),
    );
    let source =
        "import {Box} from '@devup-ui/react';export const x=<Box _hover={{typography:'body'}}/>;";
    // When
    let outputs: Vec<_> = ["a.tsx", "child.tsx"]
        .into_iter()
        .map(|file| compile(file, source))
        .collect();
    // Then
    for (output, name) in outputs.iter().zip(["a-a", "b-a"]) {
        assert!(output.code().contains(name), "{}", output.code());
        assert!(
            delivered().contains(&format!(".{name}:hover{{font-size:14px}}")),
            "{}",
            delivered()
        );
    }
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 2);
        assert!(map.values().all(|slots| slots.len() == 1));
    });
    with_style_sheet(|sheet| assert_eq!(sheet.names.len(), 0));
    reset_build_state_internal();
    register_theme_internal(sheet::theme::Theme::default());
}
