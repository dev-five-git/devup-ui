use crate::{
    ExtractOption, ImportAlias, extract, extract_style::extract_style_value::ExtractStyleValue,
};
use css::{class_map::reset_class_map, file_map::reset_file_map, style_selector::StyleSelector};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

#[rstest]
#[case::selector_array("css({ _hover: [{ padding: A }] })", ("4px", true))]
#[case::selector_constant_array("css({ _hover: selectorValues })", ("4px", true))]
#[case::singleton("css({ padding: [A] })", ("4px", false))]
#[case::fallback("css({ padding: [A, B] })", ("1px;padding:2px", false))]
#[serial]
fn units_when_library_arrays_have_distinct_rule_contexts(
    #[case] expression: &str,
    #[case] expected: (&str, bool),
    #[values(false, true)] single_css: bool,
) {
    // Given: selector arrays are native responsive rules, not scalar fallbacks.
    let _state = crate::test_state::TestStateGuard::new();
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ css }} from '@emotion/react';\nconst A=1, B=2; const selectorValues=[{{ padding: A }}];\nexport const a = {expression};"
    );
    // When: the complete library alias and constant-inlining pipeline extracts it.
    let output = extract(
        "selector-arrays.tsx",
        &source,
        ExtractOption {
            single_css,
            import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{expression}: {error}"));
    // Then: units and selector/breakpoint identity remain specific to that context.
    assert_eq!(output.styles.len(), 1, "{expression}");
    let Some(ExtractStyleValue::Static(style)) = output.styles.iter().next() else {
        panic!("expected one static declaration: {expression}");
    };
    assert_eq!(
        (style.property.as_str(), style.value.as_str(), style.level),
        ("padding", expected.0, 0)
    );
    assert_eq!(
        style.selector,
        expected
            .1
            .then(|| StyleSelector::Selector("&:hover".into()))
    );
}
