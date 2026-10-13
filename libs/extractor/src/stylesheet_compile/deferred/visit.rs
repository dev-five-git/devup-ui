use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::ast::{Expression, ObjectProperty, Statement};
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::{node::NodeId, symbol::SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) struct Deferred<'a, 's> {
    pub(super) ast: oxc_ast::builder::AstBuilder<'a>,
    pub(super) scoping: &'s Scoping,
    pub(super) calls: &'s FxHashMap<NodeId, oxc_span::Span>,
    pub(super) bindings: &'s FxHashMap<SymbolId, String>,
    pub(super) namespace: &'s str,
    pub(super) keys: FxHashSet<String>,
    pub(super) exports: Vec<String>,
}

impl<'a> Deferred<'a, '_> {
    pub(super) fn access(
        &mut self,
        span: oxc_span::Span,
        key: &str,
    ) -> oxc_allocator::Box<'a, oxc_ast::ast::ComputedMemberExpression<'a>> {
        self.keys.insert(key.to_string());
        oxc_ast::ast::ComputedMemberExpression::boxed(
            span,
            Expression::new_identifier(
                span,
                oxc_ast::ast::Str::from_in(self.namespace, self.ast.allocator()),
                &self.ast,
            ),
            Expression::new_string_literal(
                span,
                oxc_ast::ast::Str::from_in(key, self.ast.allocator()),
                None,
                &self.ast,
            ),
            false,
            &self.ast,
        )
    }

    fn symbol(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(identifier) = expression else {
            return None;
        };
        self.scoping
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
            .filter(|symbol| self.bindings.contains_key(symbol))
    }
}

impl<'a> VisitMut<'a> for Deferred<'a, '_> {
    fn visit_statement(&mut self, statement: &mut Statement<'a>) {
        match statement {
            Statement::ImportDeclaration(import) if import.source.value == "react" => {
                if let Some(specifiers) = &mut import.specifiers {
                    specifiers.retain(|specifier| {
                        specifier.local().name
                            != crate::extractor::extract_style_from_styled::FORWARD_REF
                    });
                }
                if import
                    .specifiers
                    .as_ref()
                    .is_some_and(|specifiers| specifiers.is_empty())
                {
                    *statement = Statement::EmptyStatement(oxc_ast::ast::EmptyStatement::boxed(
                        import.span,
                        &self.ast,
                    ));
                }
            }
            Statement::VariableDeclaration(declaration) => {
                declaration.declarations.retain(|declarator| {
                    !declarator.id.get_binding_identifiers().iter().any(|id| {
                        id.symbol_id
                            .get()
                            .is_some_and(|symbol| self.bindings.contains_key(&symbol))
                    })
                });
                if declaration.declarations.is_empty() {
                    *statement = Statement::EmptyStatement(oxc_ast::ast::EmptyStatement::boxed(
                        declaration.span,
                        &self.ast,
                    ));
                }
            }
            Statement::ExportDeclaration(export) => {
                if let oxc_ast::ast::Declaration::VariableDeclaration(declaration) =
                    &mut export.declaration
                {
                    declaration.declarations.retain(|declarator| {
                        if let Some(name) = declarator
                            .id
                            .get_binding_identifiers()
                            .iter()
                            .find_map(|id| self.bindings.get(&id.symbol_id.get()?))
                        {
                            self.exports.push(name.clone());
                            false
                        } else {
                            true
                        }
                    });
                    if declaration.declarations.is_empty() {
                        *statement = Statement::EmptyStatement(
                            oxc_ast::ast::EmptyStatement::boxed(export.span, &self.ast),
                        );
                    }
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                export.specifiers.retain(|specifier| {
                    let oxc_ast::ast::ModuleExportName::IdentifierReference(identifier) =
                        &specifier.local
                    else {
                        return true;
                    };
                    let Some(symbol) = identifier
                        .reference_id
                        .get()
                        .and_then(|id| self.scoping.get_reference(id).symbol_id())
                    else {
                        return true;
                    };
                    if self.bindings.contains_key(&symbol) {
                        self.exports.push(specifier.exported.name().to_string());
                        false
                    } else {
                        true
                    }
                });
            }
            Statement::ExportDefaultDeclaration(export)
                if export.declaration.as_expression().is_some_and(
                    |expression| match expression.without_parentheses() {
                        Expression::CallExpression(call) => {
                            self.calls.contains_key(&call.node_id.get())
                        }
                        expression => self.symbol(expression).is_some(),
                    },
                ) =>
            {
                self.exports.push("default".to_string());
                *statement = Statement::EmptyStatement(oxc_ast::ast::EmptyStatement::boxed(
                    export.span,
                    &self.ast,
                ));
            }
            _ => {}
        }
        walk_mut::walk_statement(self, statement);
    }

    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if let Some(symbol) = self.symbol(expression) {
            *expression = Expression::ComputedMemberExpression(
                self.access(expression.span(), &self.bindings[&symbol].clone()),
            );
            return;
        }
        if let Expression::CallExpression(call) = expression
            && let Some(site) = self.calls.get(&call.node_id.get())
        {
            *expression = Expression::ComputedMemberExpression(self.access(*site, "component"));
            return;
        }
        walk_mut::walk_expression(self, expression);
    }

    fn visit_object_property(&mut self, property: &mut ObjectProperty<'a>) {
        if property.shorthand && self.symbol(&property.value).is_some() {
            property.shorthand = false;
        }
        walk_mut::walk_object_property(self, property);
    }

    fn visit_simple_assignment_target(
        &mut self,
        target: &mut oxc_ast::ast::SimpleAssignmentTarget<'a>,
    ) {
        self.rewrite_target(target);
        walk_mut::walk_simple_assignment_target(self, target);
    }

    fn visit_assignment_target_property(
        &mut self,
        property: &mut oxc_ast::ast::AssignmentTargetProperty<'a>,
    ) {
        self.rewrite_property(property);
        walk_mut::walk_assignment_target_property(self, property);
    }
}
