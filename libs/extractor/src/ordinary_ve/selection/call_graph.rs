use oxc_ast::{AstKind, ast::Argument};
use oxc_span::GetSpan;
use oxc_syntax::node::NodeId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    apis::Apis,
    index::Index,
    plan::{HelperCall, NativeBinding, NativeCall, UnitKind},
    targets,
};

pub(super) struct Graph {
    pub edges: Vec<HelperCall>,
    pub native: Vec<NativeCall>,
    native_contexts: FxHashMap<NodeId, Vec<NodeId>>,
    pub roots: FxHashSet<NodeId>,
    pub active: FxHashSet<NodeId>,
    pub selected: FxHashSet<NodeId>,
}

impl Graph {
    pub fn new<'a>(index: &Index<'_, 'a>, apis: &Apis<'_, 'a>) -> Self {
        let mut graph = Self {
            edges: Vec::new(),
            native: Vec::new(),
            native_contexts: FxHashMap::default(),
            roots: FxHashSet::default(),
            active: FxHashSet::default(),
            selected: FxHashSet::default(),
        };
        for node in index.semantic.nodes().iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            let Some(caller) = index
                .callable_context(node.id())
                .or_else(|| index.owner(node.id()))
            else {
                continue;
            };
            match apis.binding(&call.callee) {
                Some(NativeBinding::Named { api, .. }) => {
                    graph.native.push(NativeCall {
                        node: node.id(),
                        span: call.span,
                        callee: call.callee.span(),
                        api,
                    });
                    graph
                        .native_contexts
                        .entry(caller)
                        .or_default()
                        .push(node.id());
                }
                Some(NativeBinding::Namespace) | None => {}
            }
            let mut targets = targets::callables(&call.callee, apis);
            for argument in &call.arguments {
                let expression = match argument {
                    Argument::SpreadElement(spread) => &spread.argument,
                    argument => argument.to_expression(),
                };
                targets.extend(targets::callables(expression, apis));
            }
            let mut targets: Vec<_> = targets.into_iter().collect();
            targets.sort_by_key(|id| index.semantic.nodes().kind(*id).span().start);
            graph
                .edges
                .extend(targets.into_iter().map(|callable| HelperCall {
                    caller,
                    callable,
                    span: call.span,
                }));
        }
        graph.roots = index
            .units
            .values()
            .filter(|unit| {
                !matches!(unit.kind, UnitKind::Function { .. }) && graph.contains_native(unit.node)
            })
            .map(|unit| unit.node)
            .collect();
        graph.active = graph
            .roots
            .iter()
            .flat_map(|root| graph.reachable(*root))
            .collect();
        graph
    }

    pub fn reachable(&self, root: NodeId) -> FxHashSet<NodeId> {
        let mut selected = FxHashSet::default();
        let mut pending = vec![root];
        while let Some(context) = pending.pop() {
            if !selected.insert(context) {
                continue;
            }
            pending.extend(
                self.edges
                    .iter()
                    .filter(|edge| edge.caller == context)
                    .map(|edge| edge.callable),
            );
        }
        selected
    }

    pub fn native_sites(&self, reachable: &FxHashSet<NodeId>) -> Vec<NodeId> {
        let selected: FxHashSet<_> = reachable
            .iter()
            .flat_map(|context| {
                self.native_contexts
                    .get(context)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect();
        self.native
            .iter()
            .filter(|call| selected.contains(&call.node))
            .map(|call| call.node)
            .collect()
    }

    pub fn contains_native(&self, context: NodeId) -> bool {
        !self.native_sites(&self.reachable(context)).is_empty()
    }

    pub fn helper(&self, symbol: oxc_syntax::symbol::SymbolId, apis: &Apis<'_, '_>) -> bool {
        targets::binding_callables(symbol, apis)
            .into_iter()
            .any(|callable| self.contains_native(callable))
    }
}
