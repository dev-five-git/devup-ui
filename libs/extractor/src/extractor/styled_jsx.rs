use oxc_allocator::{FromIn, GetAllocator, Vec};
use oxc_ast::{
    ast::{
        Argument, BindingIdentifier, BindingPattern, Expression, Ident, IdentifierName,
        IdentifierReference, ImportDeclaration, ImportDeclarationSpecifier, ImportOrExportKind,
        ModuleExportName, ObjectPropertyKind, Program, PropertyKey, PropertyKind, Statement,
        StringLiteral, VariableDeclaration, VariableDeclarationKind, VariableDeclarator,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::Visit;
use oxc_span::{FileExtension, SPAN};
use rustc_hash::FxHashSet;

#[cfg(test)]
#[path = "styled_jsx_tests.rs"]
mod tests;

/// The module's fresh React factory, used only by generated non-JSX renderers.
pub(crate) struct StyledJsx {
    name: String,
    used: bool,
}

#[derive(Default)]
struct Names<'a>(FxHashSet<&'a str>);

impl<'a> Visit<'a> for Names<'a> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.0.insert(it.name.as_str());
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.0.insert(it.name.as_str());
    }
}

impl StyledJsx {
    /// Reserve a name across all scopes before extraction removes or rewrites nodes.
    pub(crate) fn reserve(program: &Program<'_>) -> Option<Self> {
        if program.source_type.is_jsx()
            && !matches!(
                program.source_type.extension(),
                Some(FileExtension::Js | FileExtension::Mjs | FileExtension::Cjs)
            )
        {
            return None;
        }
        let mut names = Names::default();
        names.visit_program(program);
        let mut suffix = 0;
        let name = loop {
            let name = format!("__devupJsx{suffix}");
            if !names.0.contains(name.as_str()) {
                break name;
            }
            suffix += 1;
        };
        Some(Self { name, used: false })
    }

    /// Render the generated element with its existing class and style expressions.
    pub(crate) fn render<'a>(
        &mut self,
        ast: &AstBuilder<'a>,
        target: Expression<'a>,
        props: (Expression<'a>, Expression<'a>),
    ) -> Expression<'a> {
        self.used = true;
        let mut properties = Vec::with_capacity_in(3, ast);
        properties.push(ObjectPropertyKind::new_spread_property(
            SPAN,
            Expression::new_identifier(SPAN, "rest", ast),
            ast,
        ));
        for (key, value) in [("className", props.0), ("style", props.1)] {
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::new_static_identifier(SPAN, key, ast),
                value,
                false,
                false,
                false,
                ast,
            ));
        }
        Expression::new_call_expression(
            SPAN,
            Expression::new_identifier(SPAN, Ident::from_in(&self.name, ast.allocator()), ast),
            None,
            Vec::from_array_in(
                [
                    Argument::from(target),
                    Argument::from(Expression::new_object_expression(SPAN, properties, ast)),
                ],
                ast,
            ),
            false,
            ast,
        )
    }

    /// Insert a value binding only after a renderer has actually used it.
    pub(crate) fn binding<'a>(
        &self,
        ast: &AstBuilder<'a>,
        commonjs: bool,
    ) -> Option<Statement<'a>> {
        if !self.used {
            return None;
        }
        let name = Ident::from_in(&self.name, ast.allocator());
        if commonjs {
            let require = Expression::new_call_expression(
                SPAN,
                Expression::new_identifier(SPAN, "require", ast),
                None,
                Vec::from_array_in(
                    [Argument::from(Expression::new_string_literal(
                        SPAN,
                        "react/jsx-runtime",
                        None,
                        ast,
                    ))],
                    ast,
                ),
                false,
                ast,
            );
            let init = Expression::new_static_member_expression(
                SPAN,
                require,
                IdentifierName::new(SPAN, "jsx", ast),
                false,
                ast,
            );
            Some(Statement::VariableDeclaration(VariableDeclaration::boxed(
                SPAN,
                VariableDeclarationKind::Const,
                Vec::from_array_in(
                    [VariableDeclarator::new(
                        SPAN,
                        BindingPattern::new_binding_identifier(SPAN, name, ast),
                        None,
                        Some(init),
                        false,
                        ast,
                    )],
                    ast,
                ),
                false,
                ast,
            )))
        } else {
            Some(Statement::ImportDeclaration(ImportDeclaration::boxed(
                SPAN,
                Some(Vec::from_array_in(
                    [ImportDeclarationSpecifier::new_import_specifier(
                        SPAN,
                        ModuleExportName::new_identifier_name(SPAN, "jsx", ast),
                        BindingIdentifier::new(SPAN, name, ast),
                        ImportOrExportKind::Value,
                        ast,
                    )],
                    ast,
                )),
                StringLiteral::new(SPAN, "react/jsx-runtime", None, ast),
                None,
                None,
                ImportOrExportKind::Value,
                ast,
            )))
        }
    }
}
