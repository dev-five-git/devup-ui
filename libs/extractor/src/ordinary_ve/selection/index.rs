use oxc_ast::{
    AstKind,
    ast::{Declaration, ExportDefaultDeclarationKind, Expression, Program, Statement},
};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use oxc_syntax::{node::NodeId, symbol::SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

use super::plan::{Binding, Unit, UnitKind};
use crate::utils::unwrap_syntax_only;

pub(super) struct Index<'s, 'a> {
    pub semantic: &'s Semantic<'a>,
    pub units: FxHashMap<NodeId, Unit>,
    pub bindings: FxHashMap<SymbolId, NodeId>,
}

impl<'s, 'a> Index<'s, 'a> {
    pub fn new(program: &Program<'a>, semantic: &'s Semantic<'a>) -> Self {
        let mut index = Self {
            semantic,
            units: FxHashMap::default(),
            bindings: FxHashMap::default(),
        };
        for statement in &program.body {
            match statement {
                Statement::ImportDeclaration(_)
                | Statement::ExportNamedDeclaration(_)
                | Statement::ExportFromDeclaration(_)
                | Statement::ExportAllDeclaration(_)
                | Statement::TSTypeAliasDeclaration(_)
                | Statement::TSInterfaceDeclaration(_) => {}
                Statement::ExportDeclaration(export) => index.declaration(&export.declaration),
                Statement::ExportDefaultDeclaration(export) => match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => index.insert(
                        function.node_id.get(),
                        function.span,
                        UnitKind::Function {
                            erased: function.body.is_none(),
                        },
                    ),
                    declaration => index.insert(
                        declaration.node_id(),
                        declaration.span(),
                        UnitKind::Statement,
                    ),
                },
                statement => match statement.as_declaration() {
                    Some(declaration) => index.declaration(declaration),
                    None => {
                        index.insert(statement.node_id(), statement.span(), UnitKind::Statement);
                    }
                },
            }
        }
        let scoping = semantic.scoping();
        for symbol in scoping.symbol_ids() {
            let Some(owner) = index.owner(scoping.symbol_declaration(symbol)) else {
                continue;
            };
            index.bindings.insert(symbol, owner);
            if scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
                && let Some(unit) = index.units.get_mut(&owner)
            {
                unit.bindings.push(Binding {
                    symbol,
                    span: scoping.symbol_span(symbol),
                    name: scoping.symbol_name(symbol).to_string(),
                });
            }
        }
        for unit in index.units.values_mut() {
            unit.bindings.sort_by_key(|binding| binding.span.start);
        }
        index
    }

    fn insert(&mut self, node: NodeId, span: oxc_span::Span, kind: UnitKind) {
        self.units.insert(
            node,
            Unit {
                node,
                span,
                kind,
                bindings: Vec::new(),
            },
        );
    }

    fn declaration(&mut self, declaration: &Declaration<'a>) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    self.insert(
                        declarator.node_id.get(),
                        declarator.span,
                        UnitKind::Declarator {
                            kind: declaration.kind,
                            pattern: declarator.id.span(),
                            erased: declaration.declare,
                        },
                    );
                }
            }
            Declaration::FunctionDeclaration(function) => self.insert(
                function.node_id.get(),
                function.span,
                UnitKind::Function {
                    erased: function.body.is_none(),
                },
            ),
            Declaration::TSTypeAliasDeclaration(_) | Declaration::TSInterfaceDeclaration(_) => {}
            declaration => self.insert(
                declaration.node_id(),
                declaration.span(),
                UnitKind::Statement,
            ),
        }
    }

    pub fn owner(&self, node: NodeId) -> Option<NodeId> {
        std::iter::once(node)
            .chain(self.semantic.nodes().ancestor_ids(node))
            .find(|id| self.units.contains_key(id))
    }

    pub fn callable_context(&self, node: NodeId) -> Option<NodeId> {
        self.semantic.nodes().ancestor_ids(node).find(|id| {
            matches!(
                self.semantic.nodes().kind(*id),
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
            )
        })
    }

    pub fn helper_definition(&self, owner: NodeId) -> bool {
        match self.semantic.nodes().kind(owner) {
            AstKind::Function(_) => true,
            AstKind::VariableDeclarator(declarator) => matches!(
                declarator.init.as_ref().map(unwrap_syntax_only),
                Some(Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_))
            ),
            _ => false,
        }
    }

    pub fn ordered(&self, selected: &FxHashSet<NodeId>) -> Vec<Unit> {
        let mut units: Vec<_> = selected
            .iter()
            .filter_map(|id| self.units.get(id).cloned())
            .collect();
        units.sort_by_key(|unit| unit.span.start);
        units
    }
}
