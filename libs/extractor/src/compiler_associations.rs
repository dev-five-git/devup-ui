pub use super::compiler_projection::{OrderEdge, RewriteEdge, VariableUse};
use super::compiler_receipts::{self as receipts, ReceiptId, WitnessDisposition};
use super::compiler_request::context;
use super::extract_style_value::ExtractStyleValue as Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurvivorId(pub(crate) usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationId(pub(crate) usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeId(pub(crate) usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperandWitnessId(pub(crate) usize);
#[derive(Clone, Debug)]
pub struct OperandWitness {
    pub id: OperandWitnessId,
    pub operand: Value,
    pub survivor: SurvivorId,
    pub invocation: InvocationId,
}
#[derive(Clone, Debug)]
pub struct FinalAssociation {
    pub operand: Value,
    pub survivor: SurvivorId,
    pub receipt: ReceiptId,
}
#[derive(Clone, Debug)]
pub struct NamingObservation {
    pub operand: Value,
    pub route: Option<String>,
    pub receipt: ReceiptId,
    pub invocation: InvocationId,
    pub scope: Option<ScopeId>,
}
#[derive(Clone, Debug)]
pub struct ActualInvocation {
    pub id: InvocationId,
    pub raw: String,
    pub original: u32,
}
#[derive(Clone, Debug)]
pub struct ConsumerFrame {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub route: Option<String>,
    pub invocation: InvocationId,
}
#[derive(Clone, Default)]
pub struct Graph {
    pub operands: Vec<OperandWitness>,
    pub emissions: Vec<FinalAssociation>,
    pub naming: Vec<NamingObservation>,
    pub invocations: Vec<ActualInvocation>,
    pub frames: Vec<ConsumerFrame>,
    pub variables: Vec<VariableUse>,
    pub rewrites: Vec<RewriteEdge>,
    pub orders: Vec<OrderEdge>,
    pub(crate) invocation: Option<InvocationId>,
    pub(crate) scope: Option<ScopeId>,
}
impl Graph {
    pub(crate) fn route(&self) -> Option<String> {
        self.scope
            .and_then(|scope| self.frames[scope.0].route.clone())
    }
}
pub(crate) struct RawScope(Option<InvocationId>);
impl RawScope {
    pub(crate) fn enter(raw: &str, original: u32) -> Self {
        Self(receipts::JOURNAL.with_borrow_mut(|journal| {
            journal.as_mut().and_then(|journal| {
                let graph = &mut journal.graph;
                let id = InvocationId(graph.invocations.len());
                graph.invocations.push(ActualInvocation {
                    id,
                    raw: raw.into(),
                    original,
                });
                graph.invocation.replace(id)
            })
        }))
    }
}
impl Drop for RawScope {
    fn drop(&mut self) {
        receipts::JOURNAL.with_borrow_mut(|journal| {
            if let Some(journal) = journal {
                journal.graph.invocation = self.0;
            }
        });
    }
}
pub(crate) struct ConsumerScope(Option<ScopeId>);
impl ConsumerScope {
    pub(crate) fn enter(route: Option<&str>) -> Self {
        Self(receipts::JOURNAL.with_borrow_mut(|journal| {
            journal.as_mut().and_then(|journal| {
                let graph = &mut journal.graph;
                let invocation = graph.invocation?;
                let parent = graph.scope;
                let inherited = parent.and_then(|id| graph.frames[id.0].route.clone());
                let id = ScopeId(graph.frames.len());
                graph.frames.push(ConsumerFrame {
                    id,
                    parent,
                    route: route.map(str::to_string).or(inherited),
                    invocation,
                });
                graph.scope.replace(id)
            })
        }))
    }
}
impl Drop for ConsumerScope {
    fn drop(&mut self) {
        receipts::JOURNAL.with_borrow_mut(|journal| {
            if let Some(journal) = journal {
                journal.graph.scope = self.0;
            }
        });
    }
}
#[derive(Default)]
pub(crate) struct CollectedStyles(rustc_hash::FxHashMap<Value, Option<SurvivorId>>);
impl CollectedStyles {
    pub(crate) fn insert(&mut self, value: Value) {
        receipts::JOURNAL.with_borrow_mut(|journal| {
            let survivor = *self.0.entry(value.clone()).or_insert_with(|| {
                journal
                    .as_ref()
                    .map(|journal| SurvivorId(journal.graph.operands.len()))
            });
            if let (Some(journal), Some(survivor)) = (journal, survivor)
                && let Some(invocation) = journal.graph.invocation
            {
                journal.graph.operands.push(OperandWitness {
                    id: OperandWitnessId(journal.graph.operands.len()),
                    operand: value,
                    survivor,
                    invocation,
                });
            }
        });
    }
    pub(crate) fn extend(&mut self, values: impl IntoIterator<Item = Value>) {
        for value in values {
            self.insert(value);
        }
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Value> {
        self.0.keys()
    }
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub(crate) fn into_set(self) -> rustc_hash::FxHashSet<Value> {
        self.0.into_keys().collect()
    }
}
pub(crate) fn complete(filename: &str, option: &crate::ExtractOption, styles: &CollectedStyles) {
    if crate::compiler_policy::active() && !receipts::reservation() {
        let mut sorted: Vec<_> = styles.iter().collect();
        sorted.sort();
        let bucket = crate::css_bucket(filename, option);
        for value in sorted {
            match value {
                Value::Static(_) | Value::Dynamic(_) | Value::Keyframes(_) => {}
                Value::Typography(_) | Value::Css(_) | Value::Import(_) | Value::FontFace(_) => {
                    continue;
                }
            }
            let result = context(value, bucket.as_deref()).and_then(|context| {
                let found = receipts::JOURNAL.with_borrow(|journal| {
                    journal
                        .as_ref()
                        .and_then(|journal| journal.find(value, &context))
                });
                found.map_or_else(
                    || receipts::acquire(value, bucket.as_deref(), context).map(|value| value.0),
                    Ok,
                )
            });
            if let Some(receipt) = receipts::collect_operand(result, value) {
                receipts::JOURNAL.with_borrow_mut(|journal| {
                    if let Some(journal) = journal
                        && let Some(Some(survivor)) = styles.0.get(value)
                    {
                        journal.witnesses[receipt.0].disposition = WitnessDisposition::Emission;
                        journal.graph.emissions.push(FinalAssociation {
                            operand: value.clone(),
                            survivor: *survivor,
                            receipt,
                        });
                    }
                });
            }
        }
    }
}
pub(crate) fn ordered(before: &Value, after: &Value, receipt: ReceiptId) {
    receipts::JOURNAL.with_borrow_mut(|journal| {
        if let Some(journal) = journal
            && let Some(invocation) = journal.graph.invocation
        {
            let graph = &mut journal.graph;
            for variable in &mut graph.variables {
                let original = Value::Dynamic(variable.original.clone());
                if variable.invocation == invocation
                    && variable.scope == graph.scope
                    && (super::compiler_request::same(before, &original)
                        || super::compiler_request::same(after, &original))
                    && !variable.receipts.contains(&receipt)
                {
                    variable.receipts.push(receipt);
                }
            }
            graph.orders.push(OrderEdge {
                before: before.clone(),
                after: after.clone(),
                receipt,
                invocation,
                scope: graph.scope,
            });
        }
    });
}
