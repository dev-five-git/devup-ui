use super::*;
use css::style_selector::{AtRule, AtRuleKind};
use rstest::rstest;

#[test]
fn at_chain_when_nested_retains_outer_rules_and_child_selector()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let child = StyleSelector::At {
        outer: vec![AtRule {
            kind: AtRuleKind::Supports,
            query: "(display: grid)".into(),
        }],
        kind: AtRuleKind::Media,
        query: "print".into(),
        selector: Some("&:focus".into()),
        file: None,
    };
    let parent = StyleSelector::Selector("&:hover".into());
    // When
    let result = nest(Some(&parent), Some(&child))
        .map_err(|_| "failed to nest readable selector")?
        .ok_or("nested selector was missing")?;
    // Then
    assert_eq!(
        result.to_string(),
        "@supports(display:grid) @media print &:hover:focus"
    );
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
fn conflicting_media_when_contextualized_drops_static_or_dynamic_atom(
    #[case] dynamic: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let selector = StyleSelector::At {
        kind: AtRuleKind::Media,
        query: "screen".into(),
        selector: None,
        outer: vec![],
        file: None,
    };
    let value = if dynamic {
        ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "width",
            0,
            "rest.width",
            Some(selector),
        ))
    } else {
        ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, Some(selector)))
    };
    // When
    let result = contextualize(
        vec![ExtractStyleProp::Static(value)],
        &["@media print {".into()],
    )
    .map_err(|_| "never matching styles must be empty, not opaque")?;
    // Then
    assert!(
        matches!(result.as_slice(), [ExtractStyleProp::StaticArray(values)] if values.is_empty())
    );
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
fn global_atom_when_contextualized_is_rejected_as_opaque(#[case] dynamic: bool) {
    // Given
    let selector = StyleSelector::Global("body".into(), "global.tsx".into());
    let value = if dynamic {
        ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "width",
            0,
            "rest.width",
            Some(selector),
        ))
    } else {
        ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, Some(selector)))
    };
    // When
    let result = contextualize(vec![ExtractStyleProp::Static(value)], &["&:hover {".into()]);
    // Then
    assert!(matches!(result, Err(ContextError::Opaque)));
}

#[test]
fn dynamic_array_when_contextualized_preserves_important_order_layer_and_value()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let mut value = ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
        "width",
        2,
        "rest.width !important",
        None,
    ));
    value.set_style_order(7);
    let props = vec![ExtractStyleProp::StaticArray(vec![
        ExtractStyleProp::Static(value),
    ])];
    // When
    let result = contextualize(props, &["@layer outer {".into(), "&:hover {".into()])
        .map_err(|_| "failed to contextualize readable dynamic array")?;
    // Then
    let [ExtractStyleProp::StaticArray(array)] = result.as_slice() else {
        panic!("array must be preserved")
    };
    let [ExtractStyleProp::Static(ExtractStyleValue::Dynamic(style))] = array.as_slice() else {
        panic!("dynamic atom must be preserved")
    };
    assert_eq!(
        (
            style.identifier(),
            style.important(),
            style.style_order(),
            style.level(),
            style.layer()
        ),
        ("rest.width", true, Some(7), 2, Some("outer"))
    );
    assert_eq!(
        style.selector().map(ToString::to_string),
        Some("&:hover".into())
    );
    Ok(())
}

#[test]
fn opaque_array_entry_when_contextualized_fails_instead_of_filtering_it() {
    // Given
    let props = vec![ExtractStyleProp::StaticArray(vec![
        ExtractStyleProp::Static(ExtractStyleValue::Typography("body".into())),
    ])];
    // When
    let result = contextualize(props, &["&:hover {".into()]);
    // Then
    assert!(matches!(result, Err(ContextError::Opaque)));
}

#[test]
fn impossible_parent_when_contextualized_emits_no_atoms() -> Result<(), Box<dyn std::error::Error>>
{
    // Given
    let props = vec![ExtractStyleProp::Static(ExtractStyleValue::Static(
        ExtractStaticStyle::new("color", "red", 0, None),
    ))];
    // When
    let result = contextualize(props, &["@media print {".into(), "@media screen {".into()])
        .map_err(|_| "impossible context must be empty, not an error")?;
    // Then
    assert_eq!(result.len(), 0);
    Ok(())
}
