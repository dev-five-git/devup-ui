use crate::{
    ExtractStyleProp,
    extract_style::{extract_keyframes::ExtractKeyframes, extract_style_value::ExtractStyleValue},
    extractor::{
        KeyframesExtractResult, extract_style_from_expression::extract_style_from_expression,
    },
};
use oxc_ast::{
    AstBuilder,
    ast::{Expression, ObjectPropertyKind, PropertyKey},
};

pub fn extract_keyframes_from_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
) -> KeyframesExtractResult {
    let mut keyframes = ExtractKeyframes::default();

    if let Expression::ObjectExpression(obj) = expression {
        for p in obj.properties.iter_mut() {
            if let ObjectPropertyKind::ObjectProperty(o) = p {
                let mut name = if let PropertyKey::StaticIdentifier(ident) = &o.key {
                    ident.name.to_string()
                } else if let PropertyKey::StringLiteral(s) = &o.key {
                    s.value.to_string()
                } else if let PropertyKey::TemplateLiteral(t) = &o.key {
                    t.quasis
                        .iter()
                        .map(|q| q.value.raw.as_str())
                        .collect::<Vec<_>>()
                        .join("")
                } else if let PropertyKey::NumericLiteral(n) = &o.key {
                    n.value.to_string()
                } else {
                    continue;
                };
                // convert number
                if let Ok(num) = name.parse::<f64>() {
                    name = format!("{num}%");
                }
                let mut styles =
                    extract_style_from_expression(ast_builder, None, &mut o.value, 0, None)
                        .styles
                        .into_iter()
                        .filter_map(|s| match s {
                            ExtractStyleProp::Static(ExtractStyleValue::Static(s)) => Some(s),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                styles.sort_by_key(|a| a.property().to_string());
                keyframes.keyframes.insert(name, styles);
            }
        }
    }
    KeyframesExtractResult { keyframes }
}
