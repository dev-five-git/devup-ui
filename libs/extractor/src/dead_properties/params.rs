use crate::{
    extractor::{
        selector_params::{ArgumentList, parse, parse_expression},
        style_inputs::absent,
    },
    own_css_context::{Contexts, Frame, under},
    utils::{
        get_string_by_literal_expression, get_string_by_property_key, readable_code,
        unwrap_syntax_only,
    },
};
use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectPropertyKind};
use oxc_span::GetSpan;

pub(super) fn errors(
    expression: &Expression<'_>,
    scoping: &oxc_semantic::Scoping,
) -> Vec<(u32, String)> {
    #[derive(Clone, Copy)]
    enum Kind {
        Styles,
        Selectors,
        At(css::style_selector::AtRuleKind),
        Layers,
    }
    let mut pending = vec![(expression, Kind::Styles, None)];
    let mut contexts = Contexts::default();
    let mut errors = Vec::new();
    while let Some((expression, kind, mut path)) = pending.pop() {
        match unwrap_syntax_only(expression) {
            Expression::ArrayExpression(array) => {
                for item in array.elements.iter().rev() {
                    if let Some(value) = item.as_expression() {
                        pending.push((value, kind, path));
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                let mut keys = rustc_hash::FxHashSet::default();
                let mut properties = Vec::new();
                let mut complete = true;
                let mut candidates: Vec<_> = object.properties.iter().collect();
                while let Some(property) = candidates.pop() {
                    let property = match property {
                        ObjectPropertyKind::ObjectProperty(property) => property,
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            if let Expression::ObjectExpression(inner) =
                                unwrap_syntax_only(&spread.argument)
                            {
                                candidates.extend(inner.properties.iter());
                                continue;
                            }
                            complete = false;
                            break;
                        }
                    };
                    let Some(name) = get_string_by_property_key(&property.key) else {
                        complete = false;
                        break;
                    };
                    if keys.insert(name.clone()) {
                        properties.push((name, property));
                    }
                }
                if matches!(kind, Kind::Styles)
                    && contexts.selected(path)
                    && !complete
                    && !keys.contains("params")
                {
                    path = Some(contexts.uncertain(path));
                }
                if matches!(kind, Kind::Styles)
                    && contexts.selected(path)
                    && let Some((_, property)) =
                        properties.iter().find(|(name, _)| name == "params")
                {
                    let value = &property.value;
                    let mut partial = false;
                    let parsed = if property.kind != oxc_ast::ast::PropertyKind::Init {
                        partial = true;
                        Ok(None)
                    } else if let Expression::ArrayExpression(array) = unwrap_syntax_only(value) {
                        let entries: Vec<_> = array
                            .elements
                            .iter()
                            .map(|element| {
                                let leaf = match element {
                                    ArrayExpressionElement::Elision(_) => return Ok(None),
                                    ArrayExpressionElement::SpreadElement(_) => {
                                        partial = true;
                                        return Ok(None);
                                    }
                                    _ => element.to_expression(),
                                };
                                if !known(leaf, scoping) {
                                    partial = true;
                                    return Ok(None);
                                }
                                get_string_by_literal_expression(leaf)
                                    .map(|text| Some((text.into_owned(), leaf)))
                                    .ok_or(leaf)
                            })
                            .collect();
                        parse(ArgumentList::List(entries, value), || {
                            !partial && contexts.requires_argument(path)
                        })
                    } else if known(value, scoping) {
                        parse_expression(
                            value,
                            absent(value, None) || unbound_undefined(value, scoping),
                            || contexts.requires_argument(path),
                        )
                    } else {
                        partial = true;
                        Ok(None)
                    };
                    match parsed {
                        Ok(_) if partial => path = Some(contexts.uncertain(path)),
                        Ok(Some(params)) => path = Some(contexts.push(path, Frame::Params(params))),
                        Ok(None) => {}
                        Err(leaf) => {
                            errors.push((leaf.span().start, readable_code(leaf)));
                            continue;
                        }
                    }
                }
                for (name, property) in properties {
                    let mut child_path = path;
                    let child = match kind {
                        Kind::Styles => {
                            if matches!(
                                name.as_str(),
                                "props" | "vars" | "styleVars" | "params" | "as"
                            ) || css::is_special_property::is_special_property(&name)
                            {
                                continue;
                            }
                            if name == "selectors" {
                                Kind::Selectors
                            } else if name == "@layer" {
                                Kind::Layers
                            } else if let Some(kind) =
                                crate::extractor::extract_style_from_expression::at_rule_record_kind(
                                    &name,
                                )
                            {
                                Kind::At(kind)
                            } else {
                                if (name.starts_with([':', '.', '#', '[', '*', '>', '+', '~', '_'])
                                    || name.contains('&')
                                    || css::at_rule::split_at_rule_key(&name).is_some())
                                    && let Some(frame) = under(&name)
                                {
                                    child_path = Some(contexts.push(path, frame));
                                }
                                Kind::Styles
                            }
                        }
                        Kind::Selectors | Kind::At(_) | Kind::Layers => {
                            let frame = match kind {
                                Kind::Selectors => under(&name),
                                Kind::At(kind) => Some(Frame::At(kind, name)),
                                Kind::Layers => Some(Frame::Layer(name)),
                                Kind::Styles => None,
                            };
                            let Some(frame) = frame else { continue };
                            child_path = Some(contexts.push(path, frame));
                            Kind::Styles
                        }
                    };
                    pending.push((&property.value, child, child_path));
                }
            }
            _ => {}
        }
    }
    errors
}

fn unbound_undefined(value: &Expression<'_>, scoping: &oxc_semantic::Scoping) -> bool {
    matches!(unwrap_syntax_only(value), Expression::Identifier(id)
    if id.name == "undefined"
        && id.reference_id.get().is_some_and(|reference| {
            scoping.get_reference(reference).symbol_id().is_none()
        }))
}

fn known(value: &Expression<'_>, scoping: &oxc_semantic::Scoping) -> bool {
    matches!(
        unwrap_syntax_only(value),
        Expression::ArrayExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::NullLiteral(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::RegExpLiteral(_)
            | Expression::BigIntLiteral(_)
    ) || get_string_by_literal_expression(value).is_some()
        || absent(value, None)
        || unbound_undefined(value, scoping)
}
