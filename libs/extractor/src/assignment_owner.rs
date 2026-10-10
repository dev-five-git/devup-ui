use std::{cell::RefCell, collections::BTreeMap};

use css::Site;
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;

use crate::utils::{expression_to_code, unwrap_syntax_only};
use crate::{ExtractStyleProp, ExtractStyleValue};

#[cfg(test)]
#[path = "assignment_owner_tests.rs"]
mod tests;

struct Owner {
    start: u32,
    assignment: String,
    sites: BTreeMap<usize, Option<Site>>,
}

thread_local! {
    static OWNER: RefCell<Option<Owner>> = const { RefCell::new(None) };
}

pub(crate) struct AssignmentOwner(Option<Owner>);

impl AssignmentOwner {
    pub(crate) fn enter(expression: &Expression<'_>) -> Self {
        Self(OWNER.with_borrow_mut(|owner| {
            owner.replace(Owner {
                start: expression.span().start,
                assignment: expression_to_code(expression),
                sites: BTreeMap::new(),
            })
        }))
    }

    pub(crate) fn active() -> bool {
        OWNER.with_borrow(Option::is_some)
    }
}

impl Drop for AssignmentOwner {
    fn drop(&mut self) {
        OWNER.with_borrow_mut(|owner| *owner = self.0.take());
    }
}

/// A complete selected assignment registers once; atoms only consume its site.
pub(crate) fn site(
    start: u32,
    style: &crate::extract_style::extract_dynamic_style::ExtractDynamicStyle,
) -> Option<Site> {
    OWNER.with_borrow_mut(|owner| match owner {
        Some(owner) => owner
            .sites
            .entry(crate::sparse_sites::numeric_role(
                owner.start,
                style.level(),
                style.property(),
            ))
            .or_insert_with(|| {
                crate::provenance::site_at(
                    owner.start,
                    crate::sparse_sites::numeric_role(owner.start, style.level(), style.property()),
                    &owner.assignment,
                )
            })
            .clone(),
        None => crate::provenance::site_at(
            start,
            crate::sparse_sites::numeric_role(start, 0, style.property()),
            style.identifier(),
        ),
    })
}

pub(crate) fn capture_styles<'a>(
    source: Expression<'a>,
    styles: Vec<ExtractStyleProp<'a>>,
) -> Vec<ExtractStyleProp<'a>> {
    if !matches!(unwrap_syntax_only(&source), Expression::ArrayExpression(_))
        && let Some(scalar) = crate::assignment_owner::scalar(&source, &styles)
    {
        return vec![ExtractStyleProp::Static(scalar)];
    }
    vec![ExtractStyleProp::Evaluated {
        binding: crate::sparse_sites::binding_name(source.span().start),
        styles,
        source,
        evaluation: None,
        alternate_order: None,
        alternate_class: false,
    }]
}

pub(crate) fn scalar(
    source: &Expression<'_>,
    styles: &[ExtractStyleProp<'_>],
) -> Option<ExtractStyleValue> {
    let values: Vec<_> = styles.iter().flat_map(ExtractStyleProp::extract).collect();
    let [ExtractStyleValue::Dynamic(first), ..] = values.as_slice() else {
        return None;
    };
    if styles.iter().all(always_present)
        && values.iter().all(|value| {
            matches!(value, ExtractStyleValue::Dynamic(style)
        if style.property() == first.property() && style.level() == first.level()
        && style.selector() == first.selector() && style.layer() == first.layer()
        && style.naming() == first.naming() && style.conversion() == first.conversion()
        && !style.important())
        })
        && scalar_shape(source)
        && !expression_to_code(source).contains('`')
    {
        let mut selected = first.clone();
        selected.replace_identifier(expression_to_code(source).trim().trim_end_matches(';'));
        Some(ExtractStyleValue::Dynamic(selected))
    } else {
        None
    }
}

pub(crate) fn contains_assignment(
    span: oxc_span::Span,
    style: &crate::extract_style::extract_dynamic_style::ExtractDynamicStyle,
) -> bool {
    style
        .site
        .as_ref()
        .is_some_and(|site| crate::sparse_sites::contains_site(span, site))
}

pub(crate) fn contains_consumer(span: oxc_span::Span, style: &ExtractStyleProp<'_>) -> bool {
    let contains = |source: &Expression<'_>| {
        let start = crate::provenance::source_offset(source.span().start);
        (crate::provenance::source_offset(span.start)..crate::provenance::source_offset(span.end))
            .contains(&start)
    };
    match style {
        ExtractStyleProp::Static(ExtractStyleValue::Dynamic(style)) => {
            contains_assignment(span, style)
        }
        ExtractStyleProp::Static(_) | ExtractStyleProp::Unreadable { .. } => false,
        ExtractStyleProp::StaticArray(styles) => {
            styles.iter().any(|style| contains_consumer(span, style))
        }
        ExtractStyleProp::Evaluated { source, .. } => contains(source),
        ExtractStyleProp::Conditional { condition, .. }
        | ExtractStyleProp::Enum { condition, .. } => contains(condition),
        ExtractStyleProp::Expression { expression, .. }
        | ExtractStyleProp::MemberExpression { expression, .. } => contains(expression),
    }
}

pub(crate) fn take_consumers<'a>(
    span: oxc_span::Span,
    styles: &mut [ExtractStyleProp<'a>],
) -> Vec<ExtractStyleProp<'a>> {
    let mut consumers = Vec::new();
    for style in styles {
        match style {
            ExtractStyleProp::StaticArray(styles) => consumers.extend(take_consumers(span, styles)),
            style if contains_consumer(span, style) => consumers.push(std::mem::replace(
                style,
                ExtractStyleProp::StaticArray(vec![]),
            )),
            _ => {}
        }
    }
    consumers
}

fn always_present(style: &ExtractStyleProp<'_>) -> bool {
    match style {
        ExtractStyleProp::Static(ExtractStyleValue::Dynamic(style)) => !style.presence(),
        ExtractStyleProp::StaticArray(styles) => {
            !styles.is_empty() && styles.iter().all(always_present)
        }
        ExtractStyleProp::Conditional {
            consequent: Some(consequent),
            alternate: Some(alternate),
            ..
        } => always_present(consequent) && always_present(alternate),
        _ => false,
    }
}

fn scalar_shape(source: &Expression<'_>) -> bool {
    use crate::utils::unwrap_syntax_only;
    match unwrap_syntax_only(source) {
        Expression::ArrayExpression(_) => false,
        Expression::ConditionalExpression(source) => {
            scalar_shape(&source.consequent) && scalar_shape(&source.alternate)
        }
        Expression::LogicalExpression(source) => {
            scalar_shape(&source.left) && scalar_shape(&source.right)
        }
        Expression::ComputedMemberExpression(source) => match unwrap_syntax_only(&source.object) {
            Expression::ArrayExpression(source) => source
                .elements
                .iter()
                .filter_map(|value| value.as_expression())
                .all(scalar_shape),
            Expression::ObjectExpression(source) => {
                source.properties.iter().all(|property| match property {
                    oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) => {
                        scalar_shape(&property.value)
                    }
                    oxc_ast::ast::ObjectPropertyKind::SpreadProperty(_) => false,
                })
            }
            _ => true,
        },
        _ => true,
    }
}
