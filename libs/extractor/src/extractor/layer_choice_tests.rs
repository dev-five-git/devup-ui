use super::*;
use crate::ExtractStyleValue;
use crate::extract_style::extract_layer_order::ExtractLayerOrder;

#[test]
fn known_class_is_falsy_when_metadata_is_its_only_entry() {
    // Given: an exact existing inline carrier with registration-only metadata.
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let value = Expression::new_string_literal(SPAN, "", None, &ast);
    let values = crate::style_values::StyleValues::default();
    let imports = rustc_hash::FxHashMap::default();
    let inline_css = rustc_hash::FxHashMap::from_iter([(
        SPAN.start,
        vec![ExtractStyleValue::LayerOrder(ExtractLayerOrder {
            file: "actual.tsx".to_string(),
            layers: vec!["a".to_string()],
        })],
    )]);
    let bindings = StyledBindings {
        imports: &imports,
        values: &values,
        inline_css: &inline_css,
    };
    // When: whole-rule choice normalization reads the known class.
    let choice = read(&ast, &value, &bindings);
    // Then: registration does not turn the empty class into a truthy rule.
    assert!(matches!(choice, Ok(Choice::Rules { truthy: false, .. })));
}
