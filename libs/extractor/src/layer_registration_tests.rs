use super::*;
use crate::extract_style::{
    extract_layer_order::ExtractLayerOrder, extract_static_style::ExtractStaticStyle,
};
use crate::extractor::style_order_projection::AuthoredStyles;
use crate::style_collector::StyleCollector;
use oxc_allocator::Allocator;

fn registration(file: &str, layers: &[&str]) -> ExtractStyleValue {
    ExtractStyleValue::LayerOrder(ExtractLayerOrder {
        file: file.to_string(),
        layers: layers.iter().map(|layer| (*layer).to_string()).collect(),
    })
}

fn padding(layer: &str, value: &str) -> ExtractStyleProp<'static> {
    ExtractStyleProp::Static(ExtractStyleValue::Static(
        ExtractStaticStyle::new_with_layer("padding", value, 0, None, Some(layer.to_string())),
    ))
}

#[test]
fn registrations_survive_when_override_and_cover_remove_first_atoms() {
    // Given: authored a precedes b, and a is later replaced then covered.
    let arena = Allocator::default();
    let ast = AstBuilder::new(&arena);
    let mut composition = Composition::default();
    AuthoredStyles(vec![padding("a", "4px"), padding("b", "8px")])
        .registered("actual.tsx")
        .apply(&ast, &mut composition);
    composition.apply(&ast, vec![padding("a", "12px")]);
    // When: a known class covers a's remaining atom.
    composition.cover(&padding("a", "16px").into_extract());
    let values = composition
        .unconditional()
        .unwrap_or_else(|| panic!("unconditional fixture"));
    // Then: the first-use record remains ahead of the sole b declaration.
    assert_eq!(
        values,
        vec![
            registration("actual.tsx", &["a", "b"]),
            ExtractStyleValue::Static(ExtractStaticStyle::new_with_layer(
                "padding",
                "8px",
                0,
                None,
                Some("b".to_string())
            ))
        ]
    );
}

#[test]
fn metadata_is_transparent_when_typography_is_composed_and_covered() {
    // Given: a preset and registration share no cascade key.
    let arena = Allocator::default();
    let ast = AstBuilder::new(&arena);
    let mut composition = Composition::default();
    let order = registration("actual.tsx", &["a"]);
    composition.apply(
        &ast,
        vec![ExtractStyleProp::Static(ExtractStyleValue::Typography(
            "body".to_string(),
        ))],
    );
    // When: metadata is applied and used as a known covering value.
    composition.apply(&ast, vec![ExtractStyleProp::Static(order.clone())]);
    composition.cover(std::slice::from_ref(&order));
    // Then: typography and the classless record both survive.
    assert_eq!(
        composition.unconditional(),
        Some(vec![
            order,
            ExtractStyleValue::Typography("body".to_string())
        ])
    );
}

#[test]
fn registration_has_no_overlap_when_other_part_sets_typography() {
    // Given: registration-only props and a typography declaration.
    let metadata = [ExtractStyleProp::Static(registration("actual.tsx", &["a"]))];
    let typography = [ExtractStyleProp::Static(ExtractStyleValue::Typography(
        "body".to_string(),
    ))];
    // When: admission compares their keys.
    let overlaps = overlaps(&metadata, &typography);
    // Then: registration creates no declaration competition.
    assert!(!overlaps);
}

#[test]
fn conditional_registration_is_unconditional_when_it_has_no_declaration() {
    // Given: only one branch carries a classless registration.
    let arena = Allocator::default();
    let ast = AstBuilder::new(&arena);
    let mut composition = Composition::default();
    let order = registration("actual.tsx", &["a", "a.inner"]);
    // When: a conditional is folded.
    composition.apply_conditional(
        &ast,
        &Expression::new_identifier(oxc_span::SPAN, "on", &ast),
        vec![ExtractStyleProp::Static(order.clone())],
        vec![],
    );
    // Then: metadata does not introduce an unkeyed or conditional class.
    assert_eq!(composition.unconditional(), Some(vec![order.clone()]));
    assert!(order.extract(None).is_none());
    assert!(matches!(
        composition.into_props().as_slice(),
        [ExtractStyleProp::Static(ExtractStyleValue::LayerOrder(_))]
    ));
}

#[test]
fn collector_consolidates_when_partial_imported_records_precede_fallback_atoms() {
    // Given: declarations are reordered, but metadata records authored a before b.
    let mut collector = StyleCollector::default();
    let props = vec![
        padding("b", "8px"),
        ExtractStyleProp::Static(registration("import.ts", &["a", "b"])),
    ];
    // When: metadata is consumed before fallback and duplicate records are inserted.
    collector.use_layers(&props);
    collector.insert(registration("import.ts", &["a", "b"]));
    collector.insert(registration("other.ts", &["b", "c.inner"]));
    assert!(!collector.is_empty());
    let styles = collector.into_styles("actual.tsx");
    // Then: the consuming file owns exactly one consolidated record.
    assert_eq!(
        styles,
        std::iter::once(registration("actual.tsx", &["a", "b", "c", "c.inner"])).collect()
    );
}
