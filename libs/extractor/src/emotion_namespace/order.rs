//! Reads of an eliminated alias must be able to run only after the alias is
//! initialized. Eliminating the alias removes the read, and with it the
//! `ReferenceError` the original code throws while the binding is still in its
//! temporal dead zone.
use super::{AstKind, GetSpan, Normalizer};
use oxc_ast::ast::{Expression, Function, FunctionType};
use oxc_syntax::{node::NodeId, reference::ReferenceId, symbol::SymbolId};
use std::collections::HashSet;

const fn is_function(kind: AstKind<'_>) -> bool {
    matches!(
        kind,
        AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
    )
}

impl Normalizer<'_, '_> {
    /// Check the read `expression` names when it is a tracked variable.
    pub(super) fn check_read(&mut self, expression: &Expression<'_>) {
        if let Expression::Identifier(identifier) = expression.get_inner_expression()
            && let Some(reference) = identifier.reference_id.get()
        {
            self.check_order(reference);
        }
    }

    /// Report a read of a tracked variable that can run before its declarator.
    pub(super) fn check_order(&mut self, id: ReferenceId) {
        let scoping = self.semantic.scoping();
        let reference = scoping.get_reference(id);
        let Some(symbol) = reference.symbol_id() else {
            return;
        };
        if !self.bindings.contains_key(&symbol) {
            return;
        }
        let declaration = scoping.symbol_declaration(symbol);
        if matches!(
            self.semantic.nodes().kind(declaration),
            AstKind::VariableDeclarator(_)
        ) && !self.runs_after(reference.node_id(), declaration, &mut HashSet::new())
        {
            let span = self.semantic.nodes().kind(reference.node_id()).span();
            self.error(
                span,
                "an alias must be initialized before any read that can run first; declare it above the earliest code that can reach this read",
            );
        }
    }

    /// Whether code at `node` can only run once `declaration` has completed.
    ///
    /// Straight-line code in the declaring function is ordered by position.
    /// Code in a nested function runs when that function is called, so the
    /// outermost such function decides: one created after the declaration is
    /// ordered, and a hoisted declaration is ordered only when everything that
    /// can call it is. Which calls actually happen is not decided here; any
    /// reference that could come first rejects the read.
    fn runs_after(
        &self,
        node: NodeId,
        declaration: NodeId,
        visiting: &mut HashSet<SymbolId>,
    ) -> bool {
        let nodes = self.semantic.nodes();
        let home = nodes
            .ancestor_ids(declaration)
            .find(|id| is_function(nodes.kind(*id)));
        let outer = nodes
            .ancestor_ids(node)
            .take_while(|id| Some(*id) != home)
            .filter(|id| is_function(nodes.kind(*id)))
            .last();
        if let Some(function) = outer
            && let AstKind::Function(declared) = nodes.kind(function)
            && declared.r#type == FunctionType::FunctionDeclaration
        {
            return self.callers_run_after(declared, declaration, visiting);
        }
        let anchor = outer.unwrap_or(node);
        let case = nodes
            .ancestor_ids(declaration)
            .take_while(|id| !is_function(nodes.kind(*id)))
            .find(|id| matches!(nodes.kind(*id), AstKind::SwitchCase(_)));
        nodes.kind(anchor).span().start >= nodes.kind(declaration).span().end
            && case.is_none_or(|case| nodes.ancestor_ids(anchor).any(|id| id == case))
    }

    fn callers_run_after(
        &self,
        function: &Function<'_>,
        declaration: NodeId,
        visiting: &mut HashSet<SymbolId>,
    ) -> bool {
        let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) else {
            return true;
        };
        if !visiting.insert(symbol) {
            return true;
        }
        let scoping = self.semantic.scoping();
        scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .map(|id| scoping.get_reference(*id))
            .filter(|reference| !reference.is_type())
            .all(|reference| self.runs_after(reference.node_id(), declaration, visiting))
    }
}
