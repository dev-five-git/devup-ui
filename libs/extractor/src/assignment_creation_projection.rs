use super::{
    CreationReference, CreationReferences, ExtractStyleProp, StyledCreation, call_with_values,
    literal_fields, take_evaluations,
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_ast_visit::VisitMut;
use oxc_span::{GetSpan, SPAN};

pub(super) fn styled_creation<'a>(
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
                    crate::extract_style::compiler_projection::variable_for(style, start, &name)
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
