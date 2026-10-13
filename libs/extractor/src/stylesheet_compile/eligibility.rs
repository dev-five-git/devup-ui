use std::rc::Rc;

use oxc_ast::ast::{
    CallExpression, Expression, ImportDeclarationSpecifier, Program, Statement,
    TaggedTemplateExpression, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::SemanticBuilder;

use crate::{scope::Bindings, util_type::UtilType};

/// Only APIs unavailable to the stylesheet sandbox require normal extraction.
pub(super) fn required(program: &Program<'_>, package: &str) -> bool {
    let semantic = SemanticBuilder::new().build(program).semantic;
    let mut bindings = Bindings::default();
    bindings.scope(Rc::new(semantic.into_scoping()));
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement
            && import.source.value == package
            && !import.import_kind.is_type()
        {
            for specifier in import.specifiers.iter().flatten() {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier)
                        if !specifier.import_kind.is_type() =>
                    {
                        bindings.export(
                            specifier.local.symbol_id.get(),
                            specifier.imported.name().as_str(),
                        );
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                        bindings.namespace(specifier.local.symbol_id.get());
                    }
                    _ => {}
                }
            }
        }
    }
    let mut calls = Calls {
        bindings,
        package,
        required: false,
    };
    calls.visit_program(program);
    calls.required
}

struct Calls<'s> {
    bindings: Bindings,
    package: &'s str,
    required: bool,
}

impl Calls<'_> {
    fn inspect(&mut self, callee: &Expression<'_>) {
        self.required |= self.bindings.util(callee).is_some_and(|api| {
            matches!(
                api.as_ref(),
                UtilType::Css | UtilType::GlobalCss | UtilType::GlobalCssComponent
            )
        }) || self.bindings.is_styled(callee)
            || matches!(callee, Expression::StaticMemberExpression(member) if self.bindings.is_styled(&member.object));
    }
}

impl<'a> Visit<'a> for Calls<'_> {
    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        self.bindings.require(declaration, self.package);
        self.bindings.alias(declaration);
        walk::walk_variable_declarator(self, declaration);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        self.inspect(&call.callee);
        walk::walk_call_expression(self, call);
    }

    fn visit_tagged_template_expression(&mut self, template: &TaggedTemplateExpression<'a>) {
        self.inspect(&template.tag);
        walk::walk_tagged_template_expression(self, template);
    }
}

#[cfg(test)]
mod tests;
