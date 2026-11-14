//! What the statements of a list need not keep: a generated constant no code
//! reads or only written into, a literal constant written in place of each
//! read, and a statement whose evaluation does nothing

use oxc_allocator::{CloneIn, GetAllocator, TakeIn, Vec};
use oxc_ast::{
    ast::{ArrayExpressionElement, AssignmentOperator, AssignmentTarget, Expression, Statement},
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_semantic::Scoping;
use oxc_syntax::{scope::ScopeId, symbol::SymbolId};

use super::super::{Known, literal, literal_object};
use super::{Before, before, constant, reads};
use crate::peephole::{is_void, unwrapped};

/// Counts, or with `rewrite` writes as the value it assigns, each assignment
/// to a member of `symbol` under a key the build reads
struct Writes<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    symbol: SymbolId,
    count: usize,
}

impl Writes<'_, '_> {
    fn of(&self, value: &Expression<'_>) -> bool {
        matches!(unwrapped(value), Expression::Identifier(id)
            if id.reference_id.get().and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                == Some(self.symbol))
    }

    fn written(&self, target: &AssignmentTarget<'_>) -> bool {
        match target {
            AssignmentTarget::StaticMemberExpression(member) => self.of(&member.object),
            AssignmentTarget::ComputedMemberExpression(member) => {
                literal(&member.expression) && self.of(&member.object)
            }
            _ => false,
        }
    }
}

impl<'a> Visit<'a> for Writes<'_, 'a> {
    fn visit_assignment_expression(&mut self, it: &oxc_ast::ast::AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign && self.written(&it.left) {
            self.count += 1;
        }
        walk::walk_assignment_expression(self, it);
    }
}

impl<'a> VisitMut<'a> for Writes<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        if let Expression::AssignmentExpression(assignment) = it
            && assignment.operator == AssignmentOperator::Assign
            && self.written(&assignment.left)
        {
            *it = assignment.right.take_in(self.ast);
        }
    }
}

/// Writes `value` for each read of `symbol` past the functions the list
/// declares, which may run before it is initialized
struct Every<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    symbol: SymbolId,
    value: &'s Expression<'a>,
    replaced: usize,
}

impl<'a> VisitMut<'a> for Every<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Expression::Identifier(id) = it
            && id
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                == Some(self.symbol)
        {
            *it = self.value.clone_in(self.ast.allocator());
            self.replaced += 1;
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

/// Whether `value` is a literal no longer than a name read in its place
fn short(value: &Expression<'_>) -> bool {
    matches!(
        unwrapped(value),
        Expression::BooleanLiteral(_) | Expression::NullLiteral(_)
    ) || is_void(value)
        || matches!(unwrapped(value), Expression::NumericLiteral(number)
            if number.value >= 0.0 && number.value.to_string().len() <= 6)
}

impl<'a> Known<'a> {
    /// The scope of the list the statements `before` belong to: the outermost
    /// scope binding what they declare
    fn scope(&self, before: &Before) -> Option<ScopeId> {
        before
            .symbols
            .iter()
            .map(|symbol| self.scoping.symbol_scope_id(*symbol))
            .min_by_key(|scope| self.scoping.scope_ancestors(*scope).count())
    }

    /// Whether evaluating `value` after the statements before `index` does
    /// nothing
    fn quiet(&self, value: &Expression<'_>, statements: &[Statement<'_>], index: usize) -> bool {
        let mut before = before(statements, index);
        before.loose = true;
        literal(value)
            || self
                .scope(&before)
                .is_some_and(|scope| self.fixed(value, scope, &before, &statements[..index]))
    }

    /// Drops from the statements what they need not keep
    pub(in crate::peephole) fn tidy(
        &self,
        ast: &AstBuilder<'a>,
        statements: &mut Vec<'a, Statement<'a>>,
    ) -> bool {
        let mut changed = false;
        for index in (0..statements.len()).rev() {
            // A script may read what its top level declares.
            if let Some((symbol, name, value)) =
                constant(statements, index).filter(|(symbol, _, _)| {
                    self.scoping.symbol_scope_id(*symbol) != self.scoping.root_scope_id()
                })
            {
                let count = reads(name, statements);
                if count == 0 && self.quiet(value, statements, index) {
                    statements.remove(index);
                    changed = true;
                } else if self.only_written(ast, symbol, value, count, statements) {
                    let mut writes = Writes {
                        ast,
                        scoping: &self.scoping,
                        symbol,
                        count: 0,
                    };
                    statements.remove(index);
                    for statement in statements.iter_mut().skip(index) {
                        VisitMut::visit_statement(&mut writes, statement);
                    }
                    changed = true;
                } else if short(value) {
                    let value = value.clone_in(ast.allocator());
                    let mut every = Every {
                        ast,
                        scoping: &self.scoping,
                        symbol,
                        value: &value,
                        replaced: 0,
                    };
                    for statement in statements.iter_mut().skip(index + 1) {
                        if !matches!(statement, Statement::FunctionDeclaration(_)) {
                            every.visit_statement(statement);
                        }
                    }
                    if constant(statements, index)
                        .is_some_and(|(_, name, _)| reads(name, statements) == 0)
                    {
                        statements.remove(index);
                    }
                    changed |= every.replaced > 0;
                }
            } else if let Statement::ExpressionStatement(statement) = &statements[index] {
                let value = unwrapped(&statement.expression);
                if self.quiet(value, statements, index) {
                    statements.remove(index);
                    changed = true;
                } else if let Expression::LogicalExpression(logical) = value
                    && self.quiet(&logical.right, statements, index)
                    && let Statement::ExpressionStatement(statement) = &mut statements[index]
                    && let Expression::LogicalExpression(logical) =
                        crate::peephole::unwrapped_mut(&mut statement.expression)
                {
                    statement.expression = logical.left.take_in(ast);
                    changed = true;
                }
            }
        }
        changed
    }

    /// Whether every read of `symbol`, bound to a literal of literals, writes
    /// a member of it
    fn only_written(
        &self,
        ast: &AstBuilder<'a>,
        symbol: SymbolId,
        value: &Expression<'_>,
        reads: usize,
        statements: &[Statement<'_>],
    ) -> bool {
        let literals = match unwrapped(value) {
            Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
                matches!(element, ArrayExpressionElement::Elision(_))
                    || element.as_expression().is_some_and(literal)
            }),
            value => literal_object(value),
        };
        let mut writes = Writes {
            ast,
            scoping: &self.scoping,
            symbol,
            count: 0,
        };
        for statement in statements {
            Visit::visit_statement(&mut writes, statement);
        }
        literals && reads > 0 && writes.count == reads
    }
}
