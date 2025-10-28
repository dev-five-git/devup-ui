use crate::{
    ExtractStyleProp,
    extract_style::{extract_keyframes::ExtractKeyframes, extract_style_value::ExtractStyleValue},
    extractor::{
        ExtractResult, KeyframesExtractResult,
        extract_style_from_expression::extract_style_from_expression,
    },
    utils::get_string_by_property_key,
};
use oxc_ast::{
    AstBuilder,
    ast::{Expression, ObjectPropertyKind},
};

pub fn extract_keyframes_from_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
) -> KeyframesExtractResult {
    let mut keyframes = ExtractKeyframes::default();

    if let Expression::ObjectExpression(obj) = expression {
        for p in obj.properties.iter_mut() {
            if let ObjectPropertyKind::ObjectProperty(o) = p
                && let Some(name) = get_string_by_property_key(&o.key)
            {
                let ExtractResult { styles, .. } =
                    extract_style_from_expression(ast_builder, None, &mut o.value, 0, &None);

                let mut styles = styles
                    .into_iter()
                    .filter_map(|s| match s {
                        ExtractStyleProp::Static(ExtractStyleValue::Static(s)) => Some(s),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                styles.sort_by_key(|a| a.property().to_string());
                keyframes.keyframes.insert(
                    name.parse::<f64>().map(|v| format!("{v}%")).unwrap_or(name),
                    styles,
                );
            }
        }
    }
    KeyframesExtractResult { keyframes }
}
