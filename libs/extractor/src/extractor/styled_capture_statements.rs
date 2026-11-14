use oxc_allocator::{FromIn, GetAllocator, Vec};
use oxc_ast::{
    ast::{
        BindingPattern, Expression, Ident, Statement, VariableDeclaration, VariableDeclarationKind,
        VariableDeclarator,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;

/// Capture ordered values without inferring names for anonymous definitions.
pub(crate) fn capture_statements<'a>(
    builder: &AstBuilder<'a>,
    bindings: std::vec::Vec<(String, Expression<'a>)>,
) -> Vec<'a, Statement<'a>> {
    let mut statements = Vec::with_capacity_in(bindings.len() + 1, builder);
    for (name, value) in bindings {
        let name = Ident::from_in(name.as_str(), builder.allocator());
        // Only an anonymous definition would take the name it is declared under.
        let value = if matches!(
            crate::utils::unwrap_syntax_only(&value),
            Expression::ArrowFunctionExpression(_)
                | Expression::FunctionExpression(_)
                | Expression::ClassExpression(_)
        ) {
            crate::utils::unbound_value(builder, value)
        } else {
            value
        };
        let declaration = VariableDeclarator::new(
            SPAN,
            BindingPattern::new_binding_identifier(SPAN, name, builder),
            None,
            Some(value),
            false,
            builder,
        );
        statements.push(Statement::VariableDeclaration(VariableDeclaration::boxed(
            SPAN,
            VariableDeclarationKind::Const,
            Vec::from_array_in([declaration], builder),
            false,
            builder,
        )));
    }
    statements
}
