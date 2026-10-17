use crate::{
    ExtractStyleProp,
    extract_style::{extract_keyframes::ExtractKeyframes, extract_style_value::ExtractStyleValue},
    extractor::{
        ExtractResult, KeyframesExtractResult,
        extract_style_from_expression::{
            LiteralHandling, extract_style_from_expression, unreadable,
        },
    },
    utils::{get_string_by_property_key, readable_code, runtime_value, unwrap_syntax_only_mut},
};
use oxc_ast::{
    ast::{Expression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::GetSpan;

pub(crate) fn extract_keyframes_with_location<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
) -> (KeyframesExtractResult, Option<u32>) {
    let mut keyframes = ExtractKeyframes::default();
    let invalid = crate::dead_properties::expression_errors(expression);
    if let Some((offset, value)) =
        crate::dead_properties::located_value(&invalid, expression.span().start, true)
    {
        return (
            KeyframesExtractResult {
                keyframes,
                runtime_value: Some(value),
            },
            Some(offset),
        );
    }
    let Expression::ObjectExpression(obj) = unwrap_syntax_only_mut(expression) else {
        return (
            KeyframesExtractResult {
                keyframes,
                runtime_value: runtime_value(&unreadable(expression).styles),
            },
            Some(expression.span().start),
        );
    };
    let mut runtime = None;
    let mut runtime_offset = None;
    for p in &mut obj.properties {
        let o = match p {
            ObjectPropertyKind::ObjectProperty(o) => o,
            ObjectPropertyKind::SpreadProperty(spread) => {
                runtime = runtime
                    .or_else(|| Some((format!("...{}", readable_code(&spread.argument)), None)));
                continue;
            }
        };
        let Some(name) = get_string_by_property_key(&o.key) else {
            runtime = runtime.or_else(|| {
                Some((
                    format!(
                        "[{}]",
                        o.key
                            .as_expression()
                            .map_or_else(String::new, readable_code)
                    ),
                    None,
                ))
            });
            continue;
        };
        let ExtractResult { styles, .. } = extract_style_from_expression(
            ast_builder,
            None,
            &mut o.value,
            0,
            &None,
            LiteralHandling::ExpandResponsiveThemeToken,
        );
        if runtime.is_none()
            && let Some((offset, value)) =
                crate::dead_properties::located_value(&styles, o.value.span().start, true)
        {
            runtime = Some(value);
            runtime_offset = Some(offset);
        }

        let mut styles = styles
            .into_iter()
            .filter_map(|s| match s {
                ExtractStyleProp::Static(ExtractStyleValue::Static(s)) => Some(s),
                _ => None,
            })
            .collect::<Vec<_>>();
        styles.sort_by(|a, b| a.property().cmp(b.property()));
        keyframes.keyframes.insert(
            name.parse::<f64>().map(|v| format!("{v}%")).unwrap_or(name),
            styles,
        );
    }
    (
        KeyframesExtractResult {
            keyframes,
            runtime_value: runtime,
        },
        runtime_offset,
    )
}
