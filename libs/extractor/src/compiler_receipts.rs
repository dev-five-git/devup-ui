use super::compiler_associations::{Graph, InvocationId, NamingObservation};
pub(crate) use super::compiler_diagnostics::{collect, retain_terminal};
use super::compiler_projection::{ProjectionError, configuration};
use super::{ProducedAllocation, ProducedDynamic, extract_style_value::ExtractStyleValue};
use crate::compiler_policy::CounterCompileError as CompileError;
use css::allocation_input::{AllocationContext, CapturedNameConfig};
use css::style_origin::RealLocation;
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::Rc,
};

/// Actual producer output, not an independently minted authority certificate.
#[derive(Clone, Debug)]
pub enum ProducedData {
    Allocation(Rc<ProducedAllocation>),
    Dynamic(Rc<ProducedDynamic>),
}
impl ProducedData {
    pub fn class(&self) -> &ProducedAllocation {
        match self {
            Self::Allocation(value) => value,
            Self::Dynamic(value) => &value.class,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessDisposition {
    Emission,
    ReservationOnly,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptId(pub(crate) usize);
/// Immutable full operand and real allocation association.
#[derive(Clone, Debug)]
pub struct ReceiptWitness {
    pub id: ReceiptId,
    pub operand: ExtractStyleValue,
    pub produced: ProducedData,
    pub route: Option<String>,
    pub invocation: InvocationId,
    pub disposition: WitnessDisposition,
}
type CompileBrand<'compile> = fn(&'compile ()) -> &'compile ();
/// Opaque, invariant, thread-local attempt handoff. Only the compiler can seal it.
pub struct CompilerReceiptBatch<'compile> {
    styles: Vec<ExtractStyleValue>,
    journal: Journal,
    brand: PhantomData<(CompileBrand<'compile>, Rc<()>)>,
}
pub struct ReceiptBatchView<'view> {
    pub styles: &'view [ExtractStyleValue],
    pub witnesses: &'view [ReceiptWitness],
    pub associations: &'view Graph,
    seal: PhantomData<&'view ()>,
}
impl CompilerReceiptBatch<'_> {
    /// Consume the handoff; no borrowed view can escape the callback.
    /// # Errors
    /// Returns the consumer's error unchanged.
    pub fn consume<O, E>(
        self,
        consumer: impl for<'view> FnOnce(ReceiptBatchView<'view>) -> Result<O, E>,
    ) -> Result<O, E> {
        consumer(ReceiptBatchView {
            styles: &self.styles,
            witnesses: &self.journal.witnesses,
            associations: &self.journal.graph,
            seal: PhantomData,
        })
    }
}
#[derive(Clone)]
pub(crate) struct Journal {
    pub(crate) witnesses: Vec<ReceiptWitness>,
    pub(crate) graph: Graph,
    pub(crate) errors: Vec<(u32, ProjectionError, Option<RealLocation>)>,
    pub(crate) config: CapturedNameConfig,
}
thread_local! {
    pub(crate) static JOURNAL: RefCell<Option<Journal>> = const { RefCell::new(None) };
    static RESERVATION: Cell<bool> = const { Cell::new(false) };
    #[cfg(test)]
    pub(crate) static PRODUCTIONS: Cell<usize> = const { Cell::new(0) };
}
impl Journal {
    pub(crate) fn new() -> Self {
        Self {
            witnesses: Vec::new(),
            graph: Graph::default(),
            errors: Vec::new(),
            config: configuration(),
        }
    }
}
pub(crate) struct JournalScope(Option<Journal>, bool);
impl JournalScope {
    pub(crate) fn replace(value: Option<Journal>) -> Self {
        Self(JOURNAL.replace(value), RESERVATION.replace(false))
    }
}
impl Drop for JournalScope {
    fn drop(&mut self) {
        JOURNAL.replace(self.0.take());
        RESERVATION.set(self.1);
    }
}
pub(crate) struct Probe(Option<Journal>);
impl Probe {
    pub(crate) fn enter() -> Self {
        Self(JOURNAL.with_borrow(Clone::clone))
    }
    pub(crate) fn commit(&mut self) {
        self.0 = None;
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        if let Some(journal) = self.0.take() {
            JOURNAL.set(Some(journal));
        }
    }
}
pub(crate) struct ReservationScope(bool);
impl ReservationScope {
    pub(crate) fn enter() -> Self {
        Self(RESERVATION.replace(true))
    }
}
impl Drop for ReservationScope {
    fn drop(&mut self) {
        RESERVATION.set(self.0);
    }
}
pub(crate) fn acquire(
    operand: &ExtractStyleValue,
    route: Option<&str>,
    context: AllocationContext,
) -> Result<(ReceiptId, ProducedData), ProjectionError> {
    super::compiler_projection::frozen_configuration()?;
    JOURNAL.with_borrow_mut(|journal| {
        let journal = journal.as_mut().ok_or(ProjectionError::Context)?;
        let invocation = journal.graph.invocation.ok_or(ProjectionError::Context)?;
        let effective = route.map(str::to_string).or_else(|| journal.graph.route());
        let id = journal
            .find(operand, &context)
            .unwrap_or(ReceiptId(journal.witnesses.len()));
        if id.0 == journal.witnesses.len() {
            #[cfg(test)]
            PRODUCTIONS.set(PRODUCTIONS.get() + 1);
            let produced = super::compiler_projection::produce(operand, effective.as_deref())?;
            if produced.class().context != context {
                return Err(ProjectionError::Context);
            }
            journal.witnesses.push(ReceiptWitness {
                id,
                operand: operand.clone(),
                produced,
                route: effective,
                invocation,
                disposition: WitnessDisposition::ReservationOnly,
            });
        }
        journal.graph.naming.push(NamingObservation {
            operand: operand.clone(),
            route: route.map(str::to_string),
            receipt: id,
            invocation,
            scope: journal.graph.scope,
        });
        Ok((id, journal.witnesses[id.0].produced.clone()))
    })
}
impl Journal {
    pub(crate) fn find(
        &self,
        operand: &ExtractStyleValue,
        context: &AllocationContext,
    ) -> Option<ReceiptId> {
        self.witnesses
            .iter()
            .find(|witness| {
                witness.produced.class().context == *context
                    && super::compiler_projection::same(&witness.operand, operand)
            })
            .map(|witness| witness.id)
    }
}
pub(crate) fn reservation() -> bool {
    RESERVATION.get()
}
pub(crate) fn collect_operand<T>(
    result: Result<T, ProjectionError>,
    operand: &ExtractStyleValue,
) -> Option<T> {
    let origin = match operand {
        ExtractStyleValue::Static(style) => style.origin.location(),
        ExtractStyleValue::Dynamic(style) => style.origin.location(),
        ExtractStyleValue::Keyframes(style) => style.origin.location(),
        ExtractStyleValue::Typography(_)
        | ExtractStyleValue::Css(_)
        | ExtractStyleValue::Import(_)
        | ExtractStyleValue::FontFace(_) => None,
    };
    match (result, origin) {
        (Ok(value), _) => Some(value),
        (Err(error), None) => collect(Err(error), None),
        (Err(error), Some(origin)) => collect(
            Err(ProjectionError::Terminal(format!("{origin}: {error}"))),
            None,
        ),
    }
}
pub(crate) fn with_sealed<O, E>(
    styles: rustc_hash::FxHashSet<ExtractStyleValue>,
    consume: impl for<'compile> FnOnce(CompilerReceiptBatch<'compile>) -> Result<O, E>,
) -> Result<O, CompileError<E>> {
    let mut styles: Vec<_> = styles.into_iter().collect();
    styles.sort();
    let journal = JOURNAL.take().ok_or(ProjectionError::Context)?;
    if !super::compiler_projection::variables_ready(&journal.graph.variables) {
        return Err(ProjectionError::Variable.into());
    }
    consume(CompilerReceiptBatch {
        styles,
        journal,
        brand: PhantomData,
    })
    .map_err(CompileError::Consumer)
}
#[cfg(test)]
#[path = "compiler_receipt_tests.rs"]
pub(crate) mod tests;
