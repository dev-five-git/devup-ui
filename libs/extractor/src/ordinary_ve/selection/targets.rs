use oxc_ast::{
    AstKind,
    ast::{Expression, ObjectPropertyKind},
};
use oxc_syntax::{node::NodeId, symbol::SymbolId};
use rustc_hash::FxHashSet;

use super::{apis::Apis, binding_path, returns};
use crate::utils::{get_string_by_literal_expression, unwrap_syntax_only};

pub(super) fn callables<'a>(expression: &Expression<'a>, apis: &Apis<'_, 'a>) -> FxHashSet<NodeId> {
    let mut targets = Targets::new(apis);
    targets.collect(expression);
    targets.found
}

pub(super) fn binding_callables(symbol: SymbolId, apis: &Apis<'_, '_>) -> FxHashSet<NodeId> {
    let mut targets = Targets::new(apis);
    targets.binding(symbol);
    targets.found
}

struct Targets<'r, 's, 'a> {
    apis: &'r Apis<'s, 'a>,
    seen: FxHashSet<SymbolId>,
    found: FxHashSet<NodeId>,
    returns_seen: FxHashSet<NodeId>,
}

impl<'r, 's, 'a> Targets<'r, 's, 'a> {
    fn new(apis: &'r Apis<'s, 'a>) -> Self {
        Self {
            apis,
            seen: FxHashSet::default(),
            found: FxHashSet::default(),
            returns_seen: FxHashSet::default(),
        }
    }

    fn binding(&mut self, symbol: SymbolId) {
        if !self.seen.insert(symbol) {
            return;
        }
        match self
            .apis
            .semantic
            .nodes()
            .kind(self.apis.semantic.scoping().symbol_declaration(symbol))
        {
            AstKind::Function(function) => {
                self.found.insert(function.node_id.get());
            }
            AstKind::VariableDeclarator(declarator) => {
                let Some(path) = binding_path::find(&declarator.id, symbol) else {
                    return;
                };
                for default in path.defaults {
                    self.collect(default);
                }
                let mut value = declarator.init.as_ref();
                for key in path.keys {
                    value = value.and_then(|value| self.property(value, &key));
                }
                if let Some(value) = value {
                    self.collect(value);
                }
            }
            _ => {}
        }
    }

    fn collect(&mut self, expression: &Expression<'a>) {
        match unwrap_syntax_only(expression) {
            Expression::FunctionExpression(function) => {
                self.found.insert(function.node_id.get());
            }
            Expression::ArrowFunctionExpression(function) => {
                self.found.insert(function.node_id.get());
            }
            Expression::Identifier(_) => {
                if let Some(symbol) = self.apis.symbol(expression) {
                    self.binding(symbol);
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.collect(&conditional.consequent);
                self.collect(&conditional.alternate);
            }
            Expression::LogicalExpression(logical) => {
                self.collect(&logical.left);
                self.collect(&logical.right);
            }
            Expression::SequenceExpression(sequence) => {
                if let Some(last) = sequence.expressions.last() {
                    self.collect(last);
                }
            }
            Expression::CallExpression(call) => {
                if let Expression::StaticMemberExpression(member) = unwrap_syntax_only(&call.callee)
                    && member.property.name == "bind"
                {
                    self.collect(&member.object);
                } else {
                    let previous = std::mem::take(&mut self.found);
                    self.collect(&call.callee);
                    let callees = std::mem::replace(&mut self.found, previous);
                    for callable in callees {
                        if self.returns_seen.insert(callable) {
                            for value in returns::values(self.apis.semantic, callable) {
                                self.collect(value);
                            }
                        }
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            self.collect(&property.value);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.collect(&spread.argument);
                        }
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(value) = element.as_expression() {
                        self.collect(value);
                    }
                }
            }
            Expression::StaticMemberExpression(member) => {
                if matches!(member.property.name.as_str(), "call" | "apply") {
                    self.collect(&member.object);
                } else if let Some(value) =
                    self.property(&member.object, member.property.name.as_str())
                {
                    self.collect(value);
                }
            }
            Expression::ComputedMemberExpression(member) => {
                if let Some(key) =
                    get_string_by_literal_expression(unwrap_syntax_only(&member.expression))
                    && let Some(value) = self.property(&member.object, &key)
                {
                    self.collect(value);
                }
            }
            _ => {}
        }
    }

    fn property<'e>(
        &mut self,
        expression: &'e Expression<'a>,
        key: &str,
    ) -> Option<&'e Expression<'a>> {
        match unwrap_syntax_only(expression) {
            Expression::Identifier(_) => {
                let symbol = self.apis.symbol(expression)?;
                if !self.seen.insert(symbol) {
                    return None;
                }
                let AstKind::VariableDeclarator(declarator) = self
                    .apis
                    .semantic
                    .nodes()
                    .kind(self.apis.semantic.scoping().symbol_declaration(symbol))
                else {
                    return None;
                };
                let value = declarator
                    .init
                    .as_ref()
                    .and_then(|init| self.property(init, key));
                self.seen.remove(&symbol);
                value
            }
            Expression::ObjectExpression(object) => {
                object
                    .properties
                    .iter()
                    .rev()
                    .find_map(|property| match property {
                        ObjectPropertyKind::ObjectProperty(property)
                            if property.key.static_name().as_deref() == Some(key) =>
                        {
                            Some(&property.value)
                        }
                        _ => None,
                    })
            }
            Expression::ArrayExpression(array) => {
                let Ok(index) = key.parse::<usize>() else {
                    return None;
                };
                array.elements.get(index)?.as_expression()
            }
            Expression::StaticMemberExpression(member) => {
                let value = self.property(&member.object, member.property.name.as_str())?;
                self.property(value, key)
            }
            Expression::ComputedMemberExpression(member) => {
                let member_key =
                    get_string_by_literal_expression(unwrap_syntax_only(&member.expression))?;
                let value = self.property(&member.object, &member_key)?;
                self.property(value, key)
            }
            _ => None,
        }
    }
}
