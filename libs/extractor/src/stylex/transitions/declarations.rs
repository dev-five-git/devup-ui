use oxc_ast::ast::{
    Expression, ObjectExpression, ObjectProperty, ObjectPropertyKind, PropertyKind,
};
use oxc_span::GetSpan;

use super::{DeclarationValue, Slot, TransitionApi};
use crate::stylex::normalize_stylex_property;
use crate::utils::{build_time_error, get_str_by_property_key, key_error, spread_error};

pub(crate) type DeclarationReader<'r> =
    dyn Fn(&str, &Expression<'_>) -> Result<DeclarationValue, String> + 'r;
type Declarations = Vec<(String, String)>;

pub(super) struct DeclarationContext {
    pub api: TransitionApi,
    pub slot: Option<Slot>,
}

/// Read only plain, statically named assignments; unresolved shapes retain offsets.
pub(super) fn property<'o, 'a>(
    api: TransitionApi,
    entry: &'o ObjectPropertyKind<'a>,
) -> Result<(String, &'o ObjectProperty<'a>), (u32, String)> {
    let property = match entry {
        ObjectPropertyKind::ObjectProperty(property) => property,
        ObjectPropertyKind::SpreadProperty(spread) => return Err(spread_error(api.name(), spread)),
    };
    if property.method || property.kind != PropertyKind::Init {
        return Err((
            property.span.start,
            build_time_error(
                api.name(),
                "method/getter/setter",
                "write a plain property assignment",
            ),
        ));
    }
    let name = get_str_by_property_key(&property.key)
        .ok_or_else(|| key_error(api.name(), &property.key))?;
    Ok((name.into_owned(), property))
}

/// Preserve first insertion order while replacing a repeated property's value.
pub(super) fn declarations(
    context: DeclarationContext,
    object: &ObjectExpression<'_>,
    reader: &DeclarationReader<'_>,
) -> Result<Declarations, Vec<(u32, String)>> {
    let api = context.api;
    let mut declarations: Vec<(String, Option<String>)> =
        Vec::with_capacity(object.properties.len());
    let mut errors = vec![];
    for entry in &object.properties {
        let (key, property) = match property(api, entry) {
            Ok(property) => property,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        match api {
            TransitionApi::Position if !position_key(&key) => {
                errors.push((
                    property.key.span().start,
                    build_time_error(
                        api.name(),
                        &key,
                        "use an anchor, inset, margin, size or self-alignment position-try property",
                    ),
                ));
                continue;
            }
            TransitionApi::Position | TransitionApi::View => {}
        }
        let css_key = normalize_stylex_property(&key);
        let value = match reader(&css_key, &property.value) {
            Ok(DeclarationValue::Omitted) => None,
            Ok(DeclarationValue::Scalar(value)) => {
                Some(super::css_value(&css_key, &value).into_owned())
            }
            Err(message) => {
                let slot = context.slot.map_or("position", Slot::name);
                errors.push((property.value.span().start, format!(
                    "{message}; slot `{slot}`, declaration `{key}` has {}: accepted values are static strings/numbers or omitted null/undefined/false, not nested styles or fallback arrays",
                    value_kind(&property.value),
                )));
                continue;
            }
        };
        match declarations.iter_mut().find(|(name, _)| *name == css_key) {
            Some((_, previous)) => *previous = value,
            None => declarations.push((css_key, value)),
        }
    }
    if errors.is_empty() {
        Ok(declarations
            .into_iter()
            .filter_map(|(key, value)| value.map(|value| (key, value)))
            .collect())
    } else {
        Err(errors)
    }
}

fn value_kind(value: &Expression<'_>) -> &'static str {
    match crate::utils::unwrap_syntax_only(value) {
        Expression::BooleanLiteral(boolean) if boolean.value => "boolean true",
        Expression::ArrayExpression(_) => "an array",
        Expression::ObjectExpression(_) => "an object",
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => "a function",
        _ => "an unknown-at-build-time value",
    }
}

fn position_key(key: &str) -> bool {
    matches!(
        key,
        "anchorName"
            | "positionAnchor"
            | "positionArea"
            | "top"
            | "right"
            | "bottom"
            | "left"
            | "inset"
            | "insetBlock"
            | "insetBlockEnd"
            | "insetBlockStart"
            | "insetInline"
            | "insetInlineEnd"
            | "insetInlineStart"
            | "margin"
            | "marginBlock"
            | "marginBlockEnd"
            | "marginBlockStart"
            | "marginInline"
            | "marginInlineEnd"
            | "marginInlineStart"
            | "marginTop"
            | "marginBottom"
            | "marginLeft"
            | "marginRight"
            | "width"
            | "height"
            | "minWidth"
            | "minHeight"
            | "maxWidth"
            | "maxHeight"
            | "blockSize"
            | "inlineSize"
            | "minBlockSize"
            | "minInlineSize"
            | "maxBlockSize"
            | "maxInlineSize"
            | "alignSelf"
            | "justifySelf"
            | "placeSelf"
    )
}

pub(super) fn serialize(declarations: &[(String, String)]) -> String {
    let mut css = String::new();
    for (key, value) in declarations {
        css.push_str(key);
        css.push(':');
        css.push_str(value);
        css.push(';');
    }
    css
}
