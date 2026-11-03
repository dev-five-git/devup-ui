use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::SPAN;

pub(super) fn reserve<'a>(
    builder: &AstBuilder<'a>,
    scope: &mut Expression<'a>,
    bindings: &[(String, Expression<'a>)],
) {
    if bindings.is_empty() {
        return;
    }
    let names = bindings.iter().map(|(name, _)| {
        Expression::new_identifier(
            SPAN,
            oxc_ast::ast::Ident::from_in(name.as_str(), builder.allocator()),
            builder,
        )
        .into()
    });
    let mut reserved = oxc_allocator::Vec::from_iter_in(names, builder);
    reserved.extend(
        bindings
            .iter()
            .map(|(_, value)| value.clone_in_with_semantic_ids(builder.allocator()).into()),
    );
    reserved.push(scope.clone_in(builder.allocator()).into());
    *scope = Expression::new_array_expression(SPAN, reserved, builder);
}

pub(super) fn prepare<'a>(
    builder: &AstBuilder<'a>,
    rules: &mut Expression<'a>,
    (scope, bindings): (&mut Expression<'a>, &mut Vec<(String, Expression<'a>)>),
) {
    if let Expression::ConditionalExpression(branch) = crate::utils::unwrap_syntax_only_mut(rules) {
        prepare(builder, &mut branch.consequent, (scope, bindings));
        prepare(builder, &mut branch.alternate, (scope, bindings));
    } else if let Some(prepared) = crate::extractor::styled_record::prepare(builder, rules, scope) {
        *rules = prepared.rules;
        reserve(builder, scope, &prepared.bindings);
        bindings.extend(prepared.bindings);
    }
}
