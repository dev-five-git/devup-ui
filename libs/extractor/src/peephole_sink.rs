//! A generated constant read once, written where it is read when reading its
//! value there does nothing and gives what it gave where it was declared

use std::cell::Cell;

use oxc_allocator::Vec;
use oxc_ast::ast::{
    BinaryOperator, BindingPattern, CallExpression, Expression, IdentifierReference, ObjectPattern,
    Program, SimpleAssignmentTarget, Statement, UnaryOperator, VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_semantic::Scoping;
use oxc_syntax::{
    scope::{ScopeFlags, ScopeId},
    symbol::{SymbolFlags, SymbolId},
};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{INHERITED, Known, literal};
use crate::peephole::{generated, unwrapped};

/// Which top-level statement makes each scope, and declares each top-level
/// lexical binding: a function a later statement makes runs only once the
/// earlier statements initialized what they declare
#[derive(Default)]
pub(super) struct Order {
    statement: usize,
    scopes: FxHashMap<ScopeId, usize>,
    declared: FxHashMap<SymbolId, usize>,
}

impl Order {
    pub(super) fn of(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut order = Self::default();
        for (index, statement) in program.body.iter().enumerate() {
            order.statement = index;
            order.visit_statement(statement);
        }
        order.declared.retain(|symbol, _| {
            scoping.symbol_scope_id(*symbol) == scoping.root_scope_id()
                && scoping
                    .symbol_flags(*symbol)
                    .intersects(SymbolFlags::BlockScopedVariable | SymbolFlags::Class)
        });
        order
    }
}

impl<'a> Visit<'a> for Order {
    fn enter_scope(&mut self, _: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.scopes
            .extend(scope_id.get().map(|scope| (scope, self.statement)));
    }

    fn visit_binding_identifier(&mut self, it: &oxc_ast::ast::BindingIdentifier<'a>) {
        self.declared
            .extend(it.symbol_id.get().map(|symbol| (symbol, self.statement)));
    }
}

/// The bindings earlier statements of a list declare in its scope, and of
/// those, the ones an object rest binds: a new object of data properties
#[derive(Default)]
struct Before {
    symbols: FxHashSet<SymbolId>,
    rests: FxHashSet<SymbolId>,
}

impl<'a> Visit<'a> for Before {
    fn visit_binding_identifier(&mut self, it: &oxc_ast::ast::BindingIdentifier<'a>) {
        self.symbols.extend(it.symbol_id.get());
    }

    fn visit_object_pattern(&mut self, it: &ObjectPattern<'a>) {
        if let Some(rest) = &it.rest
            && let BindingPattern::BindingIdentifier(id) = &rest.argument
            && let Some(symbol) = id.symbol_id.get()
        {
            self.rests.insert(symbol);
        }
        walk::walk_object_pattern(self, it);
    }
}

/// Whether the statements read `name` only where nothing changes its `key`:
/// the members they read or write under other keys, its spreads, and
/// `Object.entries` of it
struct Untouched<'s> {
    name: &'s str,
    key: &'s str,
    reads: usize,
    kept: usize,
    changed: bool,
}

impl Untouched<'_> {
    fn named(&self, value: &Expression<'_>) -> bool {
        matches!(unwrapped(value), Expression::Identifier(id) if id.name == self.name)
    }
}

impl<'a> Visit<'a> for Untouched<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == self.name {
            self.reads += 1;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        match it {
            Expression::StaticMemberExpression(member) if self.named(&member.object) => {
                self.kept += 1;
            }
            // A method called on it, with it as `this`, may change it.
            Expression::CallExpression(call) => {
                if let Expression::StaticMemberExpression(member) = unwrapped(&call.callee)
                    && self.named(&member.object)
                {
                    self.changed = true;
                }
                if entries(call).is_some_and(|argument| self.named(argument)) {
                    self.kept += 1;
                }
            }
            Expression::UnaryExpression(unary)
                if unary.operator == UnaryOperator::Delete
                    && matches!(unwrapped(&unary.argument), Expression::StaticMemberExpression(member)
                        if self.named(&member.object)) =>
            {
                self.changed = true;
            }
            _ => {}
        }
        walk::walk_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::StaticMemberExpression(member) = it
            && self.named(&member.object)
        {
            self.kept += 1;
            self.changed |= member.property.name == self.key;
        }
        walk::walk_simple_assignment_target(self, it);
    }

    fn visit_spread_element(&mut self, it: &oxc_ast::ast::SpreadElement<'a>) {
        if self.named(&it.argument) {
            self.kept += 1;
        }
        walk::walk_spread_element(self, it);
    }

    fn visit_jsx_spread_attribute(&mut self, it: &oxc_ast::ast::JSXSpreadAttribute<'a>) {
        if self.named(&it.argument) {
            self.kept += 1;
        }
        walk::walk_jsx_spread_attribute(self, it);
    }
}

/// The argument of `Object.entries(argument)`
fn entries<'s, 'a>(call: &'s CallExpression<'a>) -> Option<&'s Expression<'a>> {
    if let Expression::StaticMemberExpression(member) = unwrapped(&call.callee)
        && let Expression::Identifier(object) = unwrapped(&member.object)
        && object.name == "Object"
        && member.property.name == "entries"
        && let [argument] = call.arguments.as_slice()
    {
        return argument.as_expression();
    }
    None
}

/// Puts `value` where the statements read `symbol`, unless that runs in a
/// function or loop, which may run it later or again, or in a scope of its
/// own, whose names may hide what `value` reads
struct Place<'k, 'a> {
    scoping: &'k Scoping,
    symbol: SymbolId,
    value: Option<Expression<'a>>,
    depth: usize,
}

impl<'a> VisitMut<'a> for Place<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.depth == 0
            && let Expression::Identifier(id) = it
            && id
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                == Some(self.symbol)
            && let Some(value) = self.value.take()
        {
            *it = value;
            return;
        }
        let deeper = matches!(
            it,
            Expression::ArrowFunctionExpression(_)
                | Expression::FunctionExpression(_)
                | Expression::ClassExpression(_)
        );
        self.depth += usize::from(deeper);
        walk_mut::walk_expression(self, it);
        self.depth -= usize::from(deeper);
    }

    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        let deeper = matches!(
            it,
            Statement::ForStatement(_)
                | Statement::ForInStatement(_)
                | Statement::ForOfStatement(_)
                | Statement::WhileStatement(_)
                | Statement::DoWhileStatement(_)
                | Statement::FunctionDeclaration(_)
                | Statement::ClassDeclaration(_)
                | Statement::BlockStatement(_)
                | Statement::SwitchStatement(_)
                | Statement::TryStatement(_)
                | Statement::WithStatement(_)
        );
        self.depth += usize::from(deeper);
        walk_mut::walk_statement(self, it);
        self.depth -= usize::from(deeper);
    }
}

/// How often the statements name `name`
struct Count<'s> {
    name: &'s str,
    reads: usize,
}

impl<'a> Visit<'a> for Count<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == self.name {
            self.reads += 1;
        }
    }
}

impl<'a> Known<'a> {
    /// Whether evaluating `value` anywhere after the statements `before`
    /// declare does nothing and gives the same value: literals, bindings
    /// nothing reassigns once initialized, members of an object rest no
    /// code changes, and tests of those
    fn fixed(
        &self,
        value: &Expression<'_>,
        scope: ScopeId,
        before: &Before,
        statements: &[Statement<'_>],
    ) -> bool {
        match unwrapped(value) {
            value if literal(value) || self.undefined(value) => true,
            value @ Expression::Identifier(_) => self.symbol(value).is_some_and(|symbol| {
                !self.scoping.symbol_is_mutated(symbol)
                    && if self.scoping.symbol_scope_id(symbol) == scope {
                        before.symbols.contains(&symbol)
                    } else {
                        self.order.declared.get(&symbol).is_some_and(|declared| {
                            self.order
                                .scopes
                                .get(&scope)
                                .is_some_and(|made| declared < made)
                        })
                    }
            }),
            Expression::StaticMemberExpression(member) => {
                self.symbol(&member.object).is_some_and(|symbol| {
                    !INHERITED.contains(&member.property.name.as_str())
                        && before.rests.contains(&symbol)
                        && self.scoping.symbol_scope_id(symbol) == scope
                        && !self.scoping.symbol_is_mutated(symbol)
                        && {
                            let mut untouched = Untouched {
                                name: self.scoping.symbol_name(symbol),
                                key: &member.property.name,
                                reads: 0,
                                kept: 0,
                                changed: false,
                            };
                            for statement in statements {
                                untouched.visit_statement(statement);
                            }
                            !untouched.changed && untouched.reads == untouched.kept
                        }
                })
            }
            Expression::LogicalExpression(logical) => {
                self.fixed(&logical.left, scope, before, statements)
                    && self.fixed(&logical.right, scope, before, statements)
            }
            Expression::ConditionalExpression(conditional) => {
                self.fixed(&conditional.test, scope, before, statements)
                    && self.fixed(&conditional.consequent, scope, before, statements)
                    && self.fixed(&conditional.alternate, scope, before, statements)
            }
            Expression::BinaryExpression(binary) => {
                matches!(
                    binary.operator,
                    BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
                ) && self.fixed(&binary.left, scope, before, statements)
                    && self.fixed(&binary.right, scope, before, statements)
            }
            Expression::UnaryExpression(unary) => {
                matches!(
                    unary.operator,
                    UnaryOperator::LogicalNot | UnaryOperator::Void
                ) && self.fixed(&unary.argument, scope, before, statements)
            }
            _ => false,
        }
    }

    /// `const __devupA = value; … __devupA …` as `… value …` where nothing
    /// else reads it and `value` is fixed
    pub(in crate::peephole) fn sink(&self, statements: &mut Vec<'a, Statement<'a>>) -> bool {
        let mut sunk = false;
        for index in (0..statements.len()).rev() {
            if let Some(symbol) = self.sinkable(statements, index)
                && let (head, rest) = statements.split_at_mut(index + 1)
                && let Statement::VariableDeclaration(declaration) = &mut head[index]
            {
                let mut place = Place {
                    scoping: &self.scoping,
                    symbol,
                    value: declaration.declarations[0].init.take(),
                    depth: 0,
                };
                for statement in rest {
                    place.visit_statement(statement);
                }
                if let Some(value) = place.value {
                    declaration.declarations[0].init = Some(value);
                } else {
                    statements.remove(index);
                    sunk = true;
                }
            }
        }
        sunk
    }

    fn sinkable(&self, statements: &[Statement<'a>], index: usize) -> Option<SymbolId> {
        let Statement::VariableDeclaration(declaration) = &statements[index] else {
            return None;
        };
        let [declarator] = declaration.declarations.as_slice() else {
            return None;
        };
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        let symbol = id.symbol_id.get()?;
        if declaration.kind != VariableDeclarationKind::Const || !generated(&id.name) {
            return None;
        }
        let mut count = Count {
            name: &id.name,
            reads: 0,
        };
        // A function declared before may read it too.
        for statement in statements {
            count.visit_statement(statement);
        }
        if count.reads != 1 {
            return None;
        }
        let scope = self.scoping.symbol_scope_id(symbol);
        let mut before = Before::default();
        for statement in &statements[..index] {
            if let Statement::VariableDeclaration(declaration) = statement
                && declaration.kind != VariableDeclarationKind::Var
            {
                for declarator in &declaration.declarations {
                    before.visit_binding_pattern(&declarator.id);
                }
            }
        }
        let value = declarator.init.as_ref()?;
        self.fixed(value, scope, &before, statements)
            .then_some(symbol)
    }
}
