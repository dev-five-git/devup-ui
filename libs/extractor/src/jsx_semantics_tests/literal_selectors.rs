use super::*;
use rstest::rstest;
use serial_test::serial;

fn atoms(result: &ExtractOutput) -> Vec<(String, String, Option<String>, Option<u8>)> {
    let mut atoms: Vec<_> = result
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((
                style.property().to_string(),
                style.value().to_string(),
                style.selector().as_ref().map(ToString::to_string),
                style.style_order(),
            )),
            _ => None,
        })
        .collect();
    atoms.sort();
    atoms
}

#[rstest]
#[case("styleOrder")]
#[case("style-order")]
#[serial]
fn literal_selectors_when_class_block_names_reserved_word_keeps_selector_and_root_order(
    #[case] name: &str,
) {
    let source = format!(
        "import {{css}} from '@devup-ui/react'; const a=css`{name}{{color:red}}style-order:2;background:blue`;"
    );
    let result = output(&source);
    assert_eq!(
        atoms(&result),
        vec![
            ("background".into(), "blue".into(), None, Some(2)),
            (
                "color".into(),
                "red".into(),
                Some(format!("& {name}")),
                Some(2)
            ),
        ]
    );
}

#[rstest]
#[case("styleOrder")]
#[case("style-order")]
#[serial]
fn literal_selectors_when_global_block_names_reserved_word_keeps_selector_and_sibling_order(
    #[case] name: &str,
) {
    let source = format!(
        "import {{globalCss}} from '@devup-ui/react'; globalCss`{name}{{color:red}}body{{style-order:2;color:blue}}`;"
    );
    let result = output(&source);
    assert_eq!(
        atoms(&result),
        vec![
            ("color".into(), "blue".into(), Some("body".into()), Some(2)),
            ("color".into(), "red".into(), Some(name.into()), Some(0)),
        ]
    );
}

#[test]
#[serial]
fn literal_selectors_when_no_directive_keeps_both_spellings_without_metadata() {
    for name in ["styleOrder", "style-order"] {
        let result = output(&format!(
            "import {{css}} from '@devup-ui/react'; const a=css`{name}{{color:red}}`;"
        ));
        assert_eq!(
            atoms(&result),
            vec![(
                "color".into(),
                "red".into(),
                Some(format!("& {name}")),
                None
            )]
        );
        let result = output(&format!(
            "import {{globalCss}} from '@devup-ui/react'; globalCss`{name}{{color:red}}`;"
        ));
        assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Css(css) if css.css == format!("{name}{{color:red}}"))));
    }
}

#[test]
#[serial]
fn literal_selectors_when_nested_orders_override_keeps_root_inheritance_and_sibling_isolation() {
    for name in ["styleOrder", "style-order"] {
        let source = format!(
            "import {{css}} from '@devup-ui/react'; const a=css`{name}{{color:red;style-order:3;span{{color:green}}}}aside{{color:blue}}style-order:2;`;"
        );
        let result = output(&source);
        assert_eq!(
            atoms(&result),
            vec![
                (
                    "color".into(),
                    "blue".into(),
                    Some("& aside".into()),
                    Some(2)
                ),
                (
                    "color".into(),
                    "green".into(),
                    Some(format!("& {name} span")),
                    Some(3)
                ),
                (
                    "color".into(),
                    "red".into(),
                    Some(format!("& {name}")),
                    Some(3)
                ),
            ]
        );
    }
}

#[test]
#[serial]
fn literal_selectors_when_global_root_and_inner_orders_differ_preserves_original_record_names() {
    for name in ["styleOrder", "style-order"] {
        let source = format!(
            "import {{globalCss}} from '@devup-ui/react'; globalCss`style-order:2;{name}{{color:red;style-order:3;span{{color:green}}}}aside{{color:blue}}`;"
        );
        let result = output(&source);
        assert_eq!(
            atoms(&result),
            vec![
                ("color".into(), "blue".into(), Some("aside".into()), Some(2)),
                (
                    "color".into(),
                    "green".into(),
                    Some(format!("{name} span")),
                    Some(3)
                ),
                ("color".into(), "red".into(), Some(name.into()), Some(3)),
            ]
        );
    }
}

#[test]
#[serial]
fn literal_selectors_when_actual_declaration_value_is_object_still_reports_original_location() {
    for api in ["css", "globalCss"] {
        for name in ["styleOrder", "style-order"] {
            for text in [
                format!("{name}:${{{{color:'red'}}}};color:blue"),
                format!("body{{{name}:${{{{color:'red'}}}};color:blue}}"),
            ] {
                let source =
                    format!("import {{{api}}} from '@devup-ui/react'; const a={api}`{text}`;");
                let message =
                    compile(&source).required_err("object-valued metadata is not a selector block");
                let column = source
                    .find("{color:'red'}")
                    .required("fixture has metadata expression")
                    + 1;
                assert!(
                    message.starts_with(&format!("a.tsx:1:{column}:")),
                    "{message}"
                );
                assert!(
                    message.contains("an explicit styleOrder must be"),
                    "{message}"
                );
            }
        }
    }
}

#[test]
#[serial]
fn literal_selectors_when_mixin_splits_root_keeps_envelope_and_source_effects() {
    let source = "import {css} from '@devup-ui/react'; const mark=(name)=>(trace.push(name),true); const base=css({borderColor:'green'}); const a=css`styleOrder{color:${mark('color')?'red':'blue'}}${base};style-order:${mark('order')?2:3};background:blue`;";
    let result = output(source);
    assert_eq!(
        atoms(&result)
            .into_iter()
            .filter(|atom| atom.3 == Some(2))
            .map(|atom| (atom.0, atom.2))
            .collect::<Vec<_>>(),
        vec![
            ("background".into(), None),
            ("border-color".into(), None),
            ("color".into(), Some("& styleOrder".into())),
            ("color".into(), Some("& styleOrder".into())),
        ]
    );
    let evaluated = whole::evaluate_code(&result.code, "a");
    assert_eq!(evaluated.trace, serde_json::json!(["color", "order"]));
    assert!(
        evaluated
            .element
            .as_str()
            .required("mixin emits classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn literal_selectors_when_global_grouping_places_named_layers_keeps_numeric_axis_and_selector() {
    let source = concat!(
        "import {globalCss} from '@devup-ui/react'; globalCss`@layer named{@media print{style-order:2;style-order{",
        "color:red}styleOrder{style-order:3;color:blue}}}`;",
    );
    let result = output(source);
    let mut styles: Vec<_> = result
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((
                style.value().to_string(),
                style.selector().as_ref().map(ToString::to_string),
                style.style_order(),
                style.layer().map(str::to_string),
            )),
            _ => None,
        })
        .collect();
    styles.sort();
    assert_eq!(
        styles,
        vec![
            (
                "blue".into(),
                Some("@media print styleOrder".into()),
                Some(3),
                Some("named".into())
            ),
            (
                "red".into(),
                Some("@media print style-order".into()),
                Some(2),
                Some("named".into())
            ),
        ]
    );
}

#[test]
#[serial]
fn literal_selectors_when_opaque_global_block_is_serialized_keeps_reserved_selector_records() {
    let source = "import {globalCss} from '@devup-ui/react'; globalCss`@unknown{styleOrder{color:red}style-order{color:blue}}body{style-order:2;color:green}`;";
    let result = output(source);
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Css(css) if css.css == "@unknown{styleOrder{color:red;}style-order{color:blue;}}")));
    assert_eq!(
        atoms(&result),
        vec![("color".into(), "green".into(), Some("body".into()), Some(2))]
    );
}
