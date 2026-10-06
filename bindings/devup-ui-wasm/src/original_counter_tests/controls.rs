use super::*;
use extractor::extract_style::{ExtractStyleProperty, extract_static_style::ExtractStaticStyle};

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn a_numbered_root_never_lends_its_counter_when_the_member_is_unnumbered(#[case] reverse: bool) {
    // Given
    reset_build_state_internal();
    seed_file_map(vec!["a.tsx".into()]);
    set_canonical_map(HashMap::from([("child.tsx".into(), "a.tsx".into())]));
    let mut files = ["a.tsx", "child.tsx"];
    if reverse {
        files.reverse();
    }
    // When
    for file in files {
        let output = compile(
            file,
            "import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;",
        );
        let name = if file == "a.tsx" {
            "a-a"
        } else {
            "a-OLcolor-vred"
        };
        // Then
        assert!(output.code().contains(name), "{}", output.code());
        assert!(delivered().contains(&format!(".{name}{{color:red}}")));
    }
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 1);
        assert_eq!(map["D9-0"].len(), 1);
    });
    with_style_sheet(|sheet| assert_eq!(sheet.names.len(), 1));
    reset_build_state_internal();
}

#[rstest]
#[case(false, false)]
#[case(true, false)]
#[case(false, true)]
#[serial]
fn shared_delivery_stays_content_named_when_single_or_hoisted(
    #[case] single: bool,
    #[case] hoisted: bool,
) {
    // Given
    setup();
    if hoisted {
        css::atom_hoist::set_atom_hoist(Some(1));
        css::atom_hoist::restore_atom_plan(Some(std::iter::once("a.tsx".into()).collect()));
    }
    let order = if single || hoisted { 255 } else { 0 };
    let source = format!(
        "import {{Box,globalCss}} from '@devup-ui/react';globalCss({{body:{{color:'blue'}}}});export const x=<Box color='red' styleOrder={{{order}}}/>;"
    );
    // When
    let output = code_extract_internal(
        "child.tsx",
        &source,
        "@devup-ui/react",
        "df".into(),
        single,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert!(output.code().contains("OLcolor-vred"), "{}", output.code());
    let css = with_style_sheet(|sheet| sheet.create_css(None, false));
    assert!(css.contains("color:red"), "{css}");
    assert!(css.contains(concat!("body{", "color:blue", "}")), "{css}");
    css::class_map::with_class_map(|map| assert_eq!(map.len(), 0));
    reset_build_state_internal();
}

#[test]
#[serial]
fn diagnostics_and_naming_transitions_preserve_semantic_identity() {
    // Given
    setup();
    let source = "import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;";
    let mut styles: Vec<_> = ["a.tsx", "child.tsx"]
        .into_iter()
        .map(|file| {
            extract_without_source_map(file, source, ExtractOption::default())
                .unwrap_or_else(|error| panic!("{error}"))
                .styles
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("missing style"))
        })
        .collect();
    // When
    for style in &mut styles {
        match style {
            ExtractStyleValue::Static(style) => {
                style.origin = css::style_origin::Origin::from_location(
                    css::style_origin::RealLocation::ModuleExport {
                        file: "unrelated-diagnostic.css.ts".into(),
                        binding: None,
                    },
                );
            }
            other => panic!("expected literal: {other:?}"),
        }
    }
    // Then
    for (style, name) in styles.iter().zip(["a-a", "b-a"]) {
        assert_eq!(
            style
                .extract(Some("a.tsx"))
                .unwrap_or_else(|| panic!("missing class"))
                .to_string(),
            name
        );
    }
    assert_ne!(styles[0], styles[1]);
    for style in &mut styles {
        style.join_naming(css::Naming::Risky);
    }
    assert_eq!(styles[0], styles[1]);
    assert_eq!(styles.iter().cloned().collect::<FxHashSet<_>>().len(), 1);
    let literal = ExtractStaticStyle::new("color", "red", 0, None);
    assert_eq!(literal.extract(Some("child.tsx")).to_string(), "b-a");
    reset_build_state_internal();
}

#[test]
#[serial]
fn counter_only_atoms_do_not_claim_an_unused_unnumbered_canonical_scope() {
    // Given
    reset_build_state_internal();
    seed_file_map(vec!["child.tsx".into()]);
    set_canonical_map(HashMap::from([(
        "child.tsx".into(),
        "unseeded-root.tsx".into(),
    )]));
    // When
    let output = compile(
        "child.tsx",
        "import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;",
    );
    // Then
    assert!(output.code().contains("a-a"));
    with_style_sheet(|sheet| {
        assert_eq!(sheet.names.len(), 0);
        assert!(
            sheet
                .create_css(Some("unseeded-root.tsx"), false)
                .contains(concat!(".a-a{", "color:red", "}"))
        );
    });
    css::class_map::with_class_map(|map| assert_eq!(map["D9-0"].len(), 1));
    reset_build_state_internal();
}
