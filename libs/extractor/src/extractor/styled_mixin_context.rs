use crate::extract_style::{
    extract_dynamic_style::ExtractDynamicStyle, extract_static_style::ExtractStaticStyle,
};
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::style_selector::StyleSelector;

pub(super) enum ContextError {
    Never,
    Opaque,
}

#[cfg(test)]
#[path = "w27_styled_context_coverage.rs"]
mod coverage_tests;

pub(super) fn contextualize<'a>(
    props: Vec<ExtractStyleProp<'a>>,
    context: &[String],
) -> Result<Vec<ExtractStyleProp<'a>>, ContextError> {
    if context.is_empty() {
        return Ok(props);
    }
    let text = format!(
        "{}--devupContext:0;{}",
        context.concat(),
        "}".repeat(context.len())
    );
    let parsed = crate::css_utils::css_to_style(&text, 0, &None);
    let Some(parent) = parsed.first() else {
        return Ok(Vec::new());
    };
    props.into_iter().map(|prop| place(prop, parent)).collect()
}

fn nest(
    parent: Option<&StyleSelector>,
    child: Option<&StyleSelector>,
) -> Result<Option<StyleSelector>, ContextError> {
    let mut selector = parent.cloned();
    match child {
        None => {}
        Some(StyleSelector::Selector(child)) => {
            selector = Some(StyleSelector::nest_selector(selector.as_ref(), child));
        }
        Some(StyleSelector::At {
            outer,
            kind,
            query,
            selector: child,
            ..
        }) => {
            for rule in outer {
                selector = Some(
                    StyleSelector::nest_at_rule(selector.as_ref(), rule.kind, &rule.query)
                        .ok_or(ContextError::Never)?,
                );
            }
            selector = Some(
                StyleSelector::nest_at_rule(selector.as_ref(), *kind, query)
                    .ok_or(ContextError::Never)?,
            );
            if let Some(child) = child {
                selector = Some(StyleSelector::nest_selector(selector.as_ref(), child));
            }
        }
        Some(StyleSelector::Global(_, _)) => return Err(ContextError::Opaque),
    }
    Ok(selector)
}

fn place<'a>(
    prop: ExtractStyleProp<'a>,
    parent: &ExtractStaticStyle,
) -> Result<ExtractStyleProp<'a>, ContextError> {
    match prop {
        ExtractStyleProp::Static(ExtractStyleValue::Static(mut style)) => {
            style.selector = match nest(parent.selector(), style.selector()) {
                Ok(selector) => selector,
                Err(ContextError::Never) => return Ok(ExtractStyleProp::StaticArray(Vec::new())),
                Err(ContextError::Opaque) => return Err(ContextError::Opaque),
            };
            if let Some(layer) = parent.layer() {
                crate::css_utils::nest_layer(layer, &mut style.layer);
            }
            Ok(ExtractStyleProp::Static(ExtractStyleValue::Static(style)))
        }
        ExtractStyleProp::Static(ExtractStyleValue::Dynamic(style)) => {
            let selector = match nest(parent.selector(), style.selector()) {
                Ok(selector) => selector,
                Err(ContextError::Never) => return Ok(ExtractStyleProp::StaticArray(Vec::new())),
                Err(ContextError::Opaque) => return Err(ContextError::Opaque),
            };
            let identifier = if style.important() {
                format!("{} !important", style.identifier())
            } else {
                style.identifier().to_string()
            };
            let mut nested =
                ExtractDynamicStyle::new(style.property(), style.level(), &identifier, selector);
            nested.layer = style.layer().map(str::to_string);
            if let Some(layer) = parent.layer() {
                crate::css_utils::nest_layer(layer, &mut nested.layer);
            }
            let mut value = ExtractStyleValue::Dynamic(nested);
            if let Some(order) = style.style_order() {
                value.set_style_order(order);
            }
            Ok(ExtractStyleProp::Static(value))
        }
        ExtractStyleProp::StaticArray(props) => Ok(ExtractStyleProp::StaticArray(
            props
                .into_iter()
                .map(|prop| place(prop, parent))
                .collect::<Result<_, _>>()?,
        )),
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
        } => Ok(ExtractStyleProp::Conditional {
            condition,
            consequent: consequent
                .map(|prop| place(*prop, parent).map(Box::new))
                .transpose()?,
            alternate: alternate
                .map(|prop| place(*prop, parent).map(Box::new))
                .transpose()?,
        }),
        ExtractStyleProp::Static(_)
        | ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::Enum { .. }
        | ExtractStyleProp::MemberExpression { .. }
        | ExtractStyleProp::Unreadable { .. } => Err(ContextError::Opaque),
    }
}
