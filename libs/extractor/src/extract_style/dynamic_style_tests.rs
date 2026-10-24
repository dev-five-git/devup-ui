use super::*;

#[test]
fn test_extract_dynamic_style() {
    let style = ExtractDynamicStyle::new("color", 0, "primary", None);
    assert_eq!(style.property(), "color");
    assert_eq!(style.level(), 0);
    assert_eq!(style.selector(), None);
    assert_eq!(style.identifier(), "primary");
    assert_eq!(style.style_order(), None);
    assert!(!style.important());
}

#[test]
fn test_debug_dynamic_style() {
    let style = ExtractDynamicStyle::new("color", 2, "value", None);
    assert_eq!(
        format!("{style:?}"),
        "ExtractDynamicStyle { property: \"color\", level: 2, identifier: \"value\", selector: None, style_order: None }"
    );
}

#[rstest::rstest]
#[case("color", "color", false)]
#[case("color !important fallback", "color !important fallback", false)]
#[case("`${color} !important`", "`${color}`", true)]
#[case("\"red !important\"", "\"red\"", true)]
#[case("'red !important'", "'red'", true)]
#[case("something !important", "something", true)]
fn important_suffix_when_present_is_stripped(
    #[case] input: &str,
    #[case] expected: &str,
    #[case] important: bool,
) {
    assert_eq!(
        strip_important(input.to_string()),
        (expected.to_string(), important)
    );
}

#[test]
fn test_dynamic_style_with_important() {
    let style = ExtractDynamicStyle::new("background", 0, "`${color} !important`", None);
    assert_eq!(style.property(), "background");
    assert_eq!(style.identifier(), "`${color}`");
    assert!(style.important());
}

#[test]
fn runtime_template_when_css_ends_in_a_semicolon_keeps_valid_javascript() {
    let style = ExtractDynamicStyle::new("color", 0, " `${color};`; ", None);
    assert_eq!(style.identifier(), "`${color}`");
    assert!(!style.important());
}
