use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, ObjectProperty, ObjectPropertyKind, Str},
    builder::AstBuilder,
};
use oxc_ast_visit::VisitMut;
use oxc_span::{GetSpan, SPAN};

use crate::{ExtractStyleProp, utils::call_with_values};

#[cfg(test)]
#[path = "styled_creation_tests.rs"]
mod styled_creation_tests;

pub(crate) fn coalesce(styles: &mut [ExtractStyleProp<'_>]) {
    for index in 0..styles.len() {
        let (previous, rest) = styles.split_at_mut(index);
        if let Some(ExtractStyleProp::Evaluated { binding, styles: consumers, .. }) = rest.first_mut()
            && let Some(ExtractStyleProp::Evaluated { styles: owner, .. }) = previous.iter_mut().find(|style| {
                matches!(style, ExtractStyleProp::Evaluated { binding: existing, .. } if existing == binding)
            })
        {
            owner.append(consumers);
            crate::assignment_consumers::merge(owner);
            rest[0] = ExtractStyleProp::StaticArray(vec![]);
        }
    }
}

pub(crate) fn take_evaluations<'a>(
    styles: &mut [ExtractStyleProp<'a>],
) -> Vec<(String, Expression<'a>)> {
    let mut values = Vec::new();
    for style in styles {
        match style {
            ExtractStyleProp::Evaluated {
                binding,
                evaluation,
                ..
            } => {
                if let Some(value) = evaluation.take() {
                    values.push((binding.clone(), value));
                }
            }
            ExtractStyleProp::StaticArray(styles) => values.extend(take_evaluations(styles)),
            _ => {}
        }
    }
    values
}

pub(crate) fn orders(
    styles: &mut [ExtractStyleProp<'_>],
    alternate: Option<u8>,
    is_alternate: bool,
) {
    for style in styles {
        match style {
            ExtractStyleProp::Evaluated {
                alternate_order,
                alternate_class,
                ..
            } => {
                *alternate_order =
                    Some(crate::assignment_lowering::AlternateOrder { value: alternate });
                *alternate_class = is_alternate;
            }
            ExtractStyleProp::StaticArray(styles) => orders(styles, alternate, is_alternate),
            _ => {}
        }
    }
}

pub(crate) fn class_expression<'a>(
    ast: &AstBuilder<'a>,
    styles: &mut [ExtractStyleProp<'a>],
    expression: Option<Expression<'a>>,
) -> Option<Expression<'a>> {
    let mut values = take_evaluations(styles);
    values.sort_by_key(|(_, value)| crate::provenance::source_offset(value.span().start));
    expression.map(|expression| {
        if values.is_empty() {
            expression
        } else {
            call_with_values(ast, values, expression)
        }
    })
}

pub(crate) fn component<'a>(
    ast: &AstBuilder<'a>,
    component: &mut Expression<'a>,
    styles: &mut [ExtractStyleProp<'a>],
) {
    let mut values = take_evaluations(styles);
    values.sort_by_key(|(_, value)| crate::provenance::source_offset(value.span().start));
    if !values.is_empty()
        && let Expression::ArrowFunctionExpression(component) = component
        && let Some(body) = component.body.as_expression_mut()
    {
        let original = std::mem::replace(body, Expression::new_null_literal(SPAN, ast));
        *body = call_with_values(ast, values, original);
    }
}

/// Literal fields are evaluated when styled is created, not by its render body.
pub(crate) struct StyledCreation<'b, 'a> {
    pub(crate) source: &'b Expression<'a>,
    pub(crate) styles: &'b mut [ExtractStyleProp<'a>],
}

pub(crate) fn styled_creation<'a>(
    ast: &AstBuilder<'a>,
    component: &mut Expression<'a>,
    input: StyledCreation<'_, 'a>,
) {
    let mut values = take_evaluations(input.styles);
    let evaluation_count = values.len();
    let mut fields = Vec::new();
    literal_fields(input.source, &mut fields);
    let records = input
        .styles
        .iter()
        .flat_map(ExtractStyleProp::extract)
        .collect::<Vec<_>>();
    let source_code = crate::utils::expression_to_code(input.source);
    let component_code = crate::utils::expression_to_code(component);
    let mut replacements = Vec::new();
    for (field, typography) in fields {
        if values.iter().any(|(_, value)| value.span() == field.span()) {
            continue;
        }
        let mut name = format!(
            "__devupStyled{}",
            crate::provenance::source_offset(field.span().start)
        );
        while source_code.contains(&name) || component_code.contains(&name) {
            name.push('_');
        }
        let start = field.span().start;
        let site_span = oxc_span::Span::new(start, start.saturating_add(1));
        let variables = records
            .iter()
            .filter_map(|record| match record {
                crate::ExtractStyleValue::Dynamic(style)
                    if style.site.as_ref().is_some_and(|site| {
                        crate::sparse_sites::contains_site(site_span, site)
                    }) =>
                {
                    Some(style.variable_name())
                }
                _ => None,
            })
            .collect();
        replacements.push(CreationReference {
            code: crate::utils::expression_to_code(field),
            name: name.clone(),
            span: field.span(),
            typography,
            variables,
        });
        values.push((name, field.clone_in(ast.allocator())));
    }
    let mut replace = CreationReferences {
        ast,
        replacements: &replacements,
    };
    replace.visit_expression(component);
    // Only lowered evaluations use the references; capture arguments remain authored.
    for (_, value) in values.iter_mut().take(evaluation_count) {
        replace.visit_expression(value);
    }
    values.sort_by_key(|(_, value)| crate::provenance::source_offset(value.span().start));
    if !values.is_empty() {
        let original = std::mem::replace(component, Expression::new_null_literal(SPAN, ast));
        *component = call_with_values(ast, values, original);
    }
}

fn literal_fields<'b, 'a>(
    source: &'b Expression<'a>,
    fields: &mut Vec<(&'b Expression<'a>, bool)>,
) {
    if let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(source) {
        for property in &object.properties {
            match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    let value = &property.value;
                    match crate::utils::unwrap_syntax_only(value) {
                        Expression::ObjectExpression(_) => literal_fields(value, fields),
                        Expression::ArrowFunctionExpression(_)
                        | Expression::FunctionExpression(_) => {}
                        _ if crate::static_assignment::literal_source(value) => {}
                        _ => fields.push((
                            value,
                            property.key.name().is_some_and(|name| name == "typography"),
                        )),
                    }
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    literal_fields(&spread.argument, fields);
                }
            }
        }
    }
}

struct CreationReference {
    code: String,
    name: String,
    span: oxc_span::Span,
    typography: bool,
    variables: Vec<String>,
}

struct CreationReferences<'b, 'a> {
    ast: &'b AstBuilder<'a>,
    replacements: &'b [CreationReference],
}

impl<'a> VisitMut<'a> for CreationReferences<'_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if matches!(expression, Expression::ArrowFunctionExpression(function) if function.span != SPAN)
            || matches!(expression, Expression::FunctionExpression(_))
        {
            return;
        }
        let code = crate::utils::expression_to_code(expression);
        if let Some(reference) = self.replacements.iter().find(|reference| {
            reference.code == code
                && (reference.span == expression.span()
                    || expression.span() == SPAN && reference.typography)
        }) {
            *expression = Expression::new_identifier(
                expression.span(),
                Str::from_in(reference.name.as_str(), self.ast.allocator()),
                self.ast,
            );
        } else {
            oxc_ast_visit::walk_mut::walk_expression(self, expression);
        }
    }

    fn visit_object_property(&mut self, property: &mut ObjectProperty<'a>) {
        if let Some(reference) = self.replacements.iter().find(|reference| {
            property.key.name().is_some_and(|key| {
                reference
                    .variables
                    .iter()
                    .any(|variable| variable == key.as_ref())
            })
        }) {
            property.value = Expression::new_identifier(
                property.value.span(),
                Str::from_in(reference.name.as_str(), self.ast.allocator()),
                self.ast,
            );
        } else {
            oxc_ast_visit::walk_mut::walk_object_property(self, property);
        }
    }
}
