use super::literal_w38m_support::color;
use crate::ExtractStyleValue;
use crate::extract_style::style_property::StyleProperty;
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
        color(root, None, None),
        color("green", Some("&:focus"), None),
        color("yellow", Some("&:active"), None),
        color("gray", Some("&:disabled"), None),
        color("blue", Some("&:hover"), None),
    ];
    expected_static.sort_unstable();
    let mut actual_static = Vec::new();
    let mut actual_dynamic = Vec::new();
    for style in &output.styles {
        match style {
            ExtractStyleValue::Static(_) => actual_static.push(style.clone()),
            ExtractStyleValue::Dynamic(style) => {
                actual_dynamic.push(style.property());
                assert_eq!(style.level(), 0);
                assert_eq!(style.selector(), None);
                assert_eq!(style.style_order(), None);
                assert_eq!(style.layer(), None);
                assert!(!style.important());
                assert_eq!(style.fallback(), None);
            }
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_)
            | ExtractStyleValue::Keyframes(_) => {
                panic!("unexpected scalar fixture rule: {style:?}")
            }
        }
    }
    actual_static.sort_unstable();
    assert_eq!(actual_static, expected_static);
    actual_dynamic.sort_unstable();
    let mut expected_dynamic = values
        .iter()
        .map(|(property, _)| *property)
        .collect::<Vec<_>>();
    expected_dynamic.sort_unstable();
    assert_eq!(actual_dynamic, expected_dynamic);
    css::debug::set_debug(true);
    let mut tokens = expected_static
        .iter()
        .map(|style| match style.extract(Some("a.tsx")) {
            Some(StyleProperty::ClassName(token)) => token,
            Some(StyleProperty::Variable { .. }) | None => panic!("static input token"),
        })
        .collect::<Vec<_>>();
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
