use super::literal_w38m_support::{class_tokens, static_color};
use crate::ExtractStyleValue;
use std::collections::BTreeMap;

pub(super) struct Expected {
    pub(super) tokens: Vec<String>,
    pub(super) variables: BTreeMap<String, i32>,
}

pub(super) fn inventory(
    output: &crate::ExtractOutput,
    root: &str,
    values: &[(&str, i32)],
) -> Expected {
    let mut expected_static = vec![
        static_color(root, None, None),
        static_color("green", Some("&:focus"), None),
        static_color("yellow", Some("&:active"), None),
        static_color("gray", Some("&:disabled"), None),
        static_color("blue", Some("&:hover"), None),
    ];
    expected_static.sort_unstable();
    let mut actual_static = Vec::new();
    let mut actual_dynamic = Vec::new();
    for style in &output.styles {
        match style {
            ExtractStyleValue::Dynamic(style) => {
                actual_dynamic.push(style.property());
                assert_eq!(style.level(), 0);
                assert_eq!(style.selector(), None);
                assert_eq!(style.style_order(), None);
                assert_eq!(style.layer(), None);
                assert!(!style.important());
                assert_eq!(style.fallback(), None);
            }
            ExtractStyleValue::Static(_)
            | ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_)
            | ExtractStyleValue::Keyframes(_) => actual_static.push(style.clone()),
        }
    }
    actual_static.sort_unstable();
    assert_eq!(
        actual_static,
        expected_static
            .iter()
            .cloned()
            .map(ExtractStyleValue::Static)
            .collect::<Vec<_>>()
    );
    actual_dynamic.sort_unstable();
    let mut expected_dynamic = values
        .iter()
        .map(|(property, _)| *property)
        .collect::<Vec<_>>();
    expected_dynamic.sort_unstable();
    assert_eq!(actual_dynamic, expected_dynamic);
    let mut tokens = class_tokens(&expected_static);
    css::debug::set_debug(true);
    tokens.extend(values.iter().map(|(property, _)| {
        css::sheet_to_classname(property, 0, None, None, None, Some("a.tsx"))
    }));
    let variables = values
        .iter()
        .map(|(property, value)| (css::sheet_to_variable_name(property, 0, None), *value))
        .collect();
    css::debug::set_debug(false);
    tokens.sort_unstable();
    Expected { tokens, variables }
}
