use crate::extract_style::style_property::StyleProperty;
use crate::{
    ExtractStyleProp,
    assignment_test_support::evaluate,
    coverage_tests::{code, expression},
};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn residual_rebuilt_tailwind_logical_class_keeps_padding_associations_and_falsy_guard(
    #[case] enabled: bool,
) {
    // Given: a valid conditional padding consumer plus an unextracted class operand.
    // The full visitor normally pre-extracts this class; this is its typed property seam.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let padding: Vec<_> = crate::tailwind::parse_class("p-2")
        .unwrap_or_else(|| panic!("valid padding utility"))
        .styles()
        .map(crate::ExtractStyleValue::Static)
        .collect();
    assert_eq!(padding.len(), 1);
    let mut styles = [ExtractStyleProp::Conditional {
        condition: expression(&allocator, "state.enabled"),
        consequent: Some(Box::new(ExtractStyleProp::Static(padding[0].clone()))),
        alternate: None,
    }];
    let input = Some(expression(&allocator, "state.enabled && 'p-4'"));
    // When
    let (class, records) =
        super::get_class_name_expression(&ast, &input, &mut styles, None, &[], None);
    let Some(class) = class else {
        panic!("conditional class")
    };
    let actual = evaluate(&format!(
        "let reads=0;const state={{get enabled(){{reads++;return {enabled}}}}};const value={};JSON.stringify([value.trim().split(/\\s+/).filter(Boolean),reads]);",
        code(&class),
    ));
    // Then: derive expected class membership from independently authored padding values.
    assert_eq!(records.len(), 1);
    let mut classes = Vec::new();
    for record in records.iter().chain(&padding) {
        let crate::ExtractStyleValue::Static(style) = record else {
            panic!("padding atom")
        };
        assert_eq!(style.property, "padding");
        let expected = if classes.is_empty() { "1rem" } else { ".5rem" };
        assert_eq!(style.value, expected);
        let Some(StyleProperty::ClassName(class)) = record.extract(None) else {
            panic!("static class")
        };
        classes.push(class);
    }
    let expected = if enabled {
        format!("[[\"{}\",\"{}\"],2]", classes[0], classes[1])
    } else {
        "[[],2]".into()
    };
    assert_eq!(actual, expected);
}
