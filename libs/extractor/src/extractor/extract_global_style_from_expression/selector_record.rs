use super::{
    AstBuilder, AtRule, Expression, ExtractStyleProp, ExtractStyleValue, LiteralHandling,
    ObjectPropertyKind, StyleSelector, extract_style_from_expression, get_str_by_property_key,
    get_string_by_literal_expression, get_string_by_property_key, place_in_layer, unreadable_key,
    yield_typography,
};
use oxc_ast::ast::ObjectProperty;

pub(super) struct Context<'s> {
    pub file: &'s str,
    pub at_rules: &'s [AtRule],
}

/// Extract a selector-record entry without interpreting its key as metadata.
pub(super) fn extract<'a>(
    ast: &AstBuilder<'a>,
    property: &mut ObjectProperty<'a>,
    context: Context<'_>,
) -> Vec<ExtractStyleProp<'a>> {
    let Some(name) = get_string_by_property_key(&property.key) else {
        return vec![unreadable_key(&property.key, false)];
    };
    let layer_name = if let Expression::ObjectExpression(object) = &property.value
        && let Some(ObjectPropertyKind::ObjectProperty(layer)) = object.properties.iter().find(|entry| matches!(entry, ObjectPropertyKind::ObjectProperty(entry) if get_str_by_property_key(&entry.key).as_deref() == Some("@layer")))
    {
        get_string_by_literal_expression(&layer.value)
    } else {
        None
    };
    let global = StyleSelector::Global(
        if let Some(name) = name.strip_prefix('_') {
            StyleSelector::from(name).to_string().replace('&', "*")
        } else {
            name
        },
        context.file.to_string(),
    );
    let selector = context.at_rules.iter().try_fold(global, |selector, rule| {
        StyleSelector::nest_at_rule(Some(&selector), rule.kind, &rule.query)
    });
    let mut styles = selector
        .map(|selector| {
            extract_style_from_expression(
                ast,
                None,
                &mut property.value,
                0,
                &Some(selector),
                LiteralHandling::ExpandResponsiveThemeToken,
            )
            .styles
        })
        .unwrap_or_default();
    styles.retain(|style| {
        !matches!(style, ExtractStyleProp::Static(ExtractStyleValue::Static(style)) if style.property() == "@layer")
    });
    if let Some(layer) = layer_name {
        place_in_layer(&mut styles, &layer);
    }
    yield_typography(&mut styles);
    styles
}
