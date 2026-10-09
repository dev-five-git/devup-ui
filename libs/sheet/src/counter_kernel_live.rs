use super::{
    FrozenAuthority, LinkedBatch,
    authority::Retained,
    error::{KernelError, UpdateError},
    prepare,
    publication::Publication,
    records,
};
use crate::{StyleSheet, counter_evidence::CounterEvidence};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use rustc_hash::FxHashSet;
use std::marker::PhantomData;

type Brand<'id> = PhantomData<(fn(&'id mut ()) -> &'id mut (), std::rc::Rc<()>)>;

/// Opaque local Rust sidecar. Default adopts only emission-empty sheets.
#[derive(Default)]
pub struct KernelEvidence {
    pub(super) retained: Option<Retained>,
}

/// Existing Output inputs; routing and cleanup are derived internally.
pub struct UpdateRequest<'a> {
    pub raw_source: &'a str,
    pub single_css: bool,
}

/// Actual insertion and guarded BASE cleanup effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateEffects {
    pub collected: bool,
    pub updated_base_style: bool,
    pub default_collected: bool,
}

/// Exclusive borrowed sheet and sidecar; acquire admission before external locks.
///
/// Successful empty update:
/// ```
/// use sheet::{StyleSheet, counter_kernel::{CounterSheet, KernelEvidence, UpdateRequest}};
/// let mut sheet = StyleSheet::default();
/// let mut evidence = KernelEvidence::default();
/// let result = CounterSheet::new(&mut sheet, &mut evidence).with_attempt(|attempt| {
///     attempt.prepare(&Default::default(), UpdateRequest { raw_source: "a", single_css: true })?
///         .finish(|_, _| Ok::<_, std::convert::Infallible>(17))
/// });
/// assert_eq!(result, Ok(17));
/// ```
/// Completion cannot be transferred between attempts:
/// ```compile_fail
/// use sheet::{StyleSheet, counter_kernel::{CounterSheet, KernelEvidence, UpdateRequest}};
/// let (mut a, mut b) = (StyleSheet::default(), StyleSheet::default());
/// let (mut ea, mut eb) = (KernelEvidence::default(), KernelEvidence::default());
/// let _: Result<(), sheet::counter_kernel::UpdateError<std::convert::Infallible>> =
/// CounterSheet::new(&mut a, &mut ea).with_attempt(|outer| {
///     let completed = outer.prepare(&Default::default(), UpdateRequest { raw_source: "a", single_css: true })?
///         .finish(|_, _| Ok::<_, std::convert::Infallible>(17))?;
///     CounterSheet::new(&mut b, &mut eb).with_attempt(|_| Ok(completed))?;
///     unreachable!()
/// });
/// ```
/// Prepared borrows cannot escape:
/// ```compile_fail
/// use sheet::{StyleSheet, counter_kernel::{CounterSheet, KernelEvidence, UpdateRequest, UpdateError, CompletedUpdate}};
/// let mut sheet = StyleSheet::default();
/// let mut evidence = KernelEvidence::default();
/// let mut escaped = None;
/// let _ = CounterSheet::new(&mut sheet, &mut evidence).with_attempt(|attempt| {
///     escaped = Some(attempt.prepare(&Default::default(), UpdateRequest { raw_source: "a", single_css: true })?);
///     Err::<CompletedUpdate<'_, ()>, _>(UpdateError::Output(()))
/// });
/// ```
/// Prospective sheet borrows cannot escape through output:
/// ```compile_fail
/// use sheet::{StyleSheet, counter_kernel::{CounterSheet, KernelEvidence, UpdateRequest}};
/// let mut sheet = StyleSheet::default();
/// let mut evidence = KernelEvidence::default();
/// let _ = CounterSheet::new(&mut sheet, &mut evidence).with_attempt(|attempt| {
///     attempt.prepare(&Default::default(), UpdateRequest { raw_source: "a", single_css: true })?
///         .finish(|sheet, _| Ok::<_, std::convert::Infallible>(sheet))
/// });
/// ```
/// Construction capabilities cannot migrate away from the admission-owner thread:
/// ```compile_fail,E0277
/// use sheet::counter_kernel::KernelAttempt;
/// fn requires_send<T: Send>() {}
/// fn migrate<'id>() {
///     requires_send::<KernelAttempt<'id>>();
/// }
/// ```
/// Construction capabilities cannot be shared between threads:
/// ```compile_fail,E0277
/// use sheet::counter_kernel::KernelAttempt;
/// fn requires_sync<T: Sync>() {}
/// fn share<'id>() {
///     requires_sync::<KernelAttempt<'id>>();
/// }
/// ```
/// Prepared capabilities cannot migrate away from the admission-owner thread:
/// ```compile_fail,E0277
/// use sheet::counter_kernel::PreparedUpdate;
/// fn requires_send<T: Send>() {}
/// fn migrate<'id>() {
///     requires_send::<PreparedUpdate<'id>>();
/// }
/// ```
/// Prepared capabilities cannot be shared between threads:
/// ```compile_fail,E0277
/// use sheet::counter_kernel::PreparedUpdate;
/// fn requires_sync<T: Sync>() {}
/// fn share<'id>() {
///     requires_sync::<PreparedUpdate<'id>>();
/// }
/// ```
/// Completion capabilities cannot migrate away from the admission-owner thread:
/// ```compile_fail,E0277
/// use sheet::counter_kernel::CompletedUpdate;
/// fn requires_send<T: Send>() {}
/// fn migrate<'id>() {
///     requires_send::<CompletedUpdate<'id, ()>>();
/// }
/// ```
/// Completion capabilities cannot be shared between threads:
/// ```compile_fail,E0277
/// use sheet::counter_kernel::CompletedUpdate;
/// fn requires_sync<T: Sync>() {}
/// fn share<'id>() {
///     requires_sync::<CompletedUpdate<'id, ()>>();
/// }
/// ```
pub struct CounterSheet<'a> {
    sheet: &'a mut StyleSheet,
    evidence: &'a mut KernelEvidence,
}

/// Invariant, generative, single-use construction capability.
pub struct KernelAttempt<'id> {
    sheet: &'id mut StyleSheet,
    evidence: &'id KernelEvidence,
    brand: Brand<'id>,
}

/// Prepared prospective live sheet, not yet committed.
pub struct PreparedUpdate<'id> {
    sheet: &'id mut StyleSheet,
    pending: KernelEvidence,
    effects: UpdateEffects,
    brand: Brand<'id>,
}

/// Provisional output; only the outer callback boundary may commit it.
pub struct CompletedUpdate<'id, O> {
    sheet: &'id mut StyleSheet,
    pending: KernelEvidence,
    output: O,
    brand: Brand<'id>,
}

impl KernelEvidence {
    pub(super) fn snapshot(&self) -> Self {
        Self {
            retained: self.retained.clone(),
        }
    }
    pub(super) fn validate(&self, sheet: &StyleSheet) -> Result<LinkedBatch, KernelError> {
        self.retained.as_ref().map_or_else(
            || {
                records::coverage(sheet, &[])?;
                if sheet.atom_plan.is_some() && sheet.atom_plan != css::atom_hoist::atom_plan() {
                    return Err(KernelError::Authority);
                }
                LinkedBatch::link_captured_batch(
                    &[],
                    &CounterEvidence::default(),
                    &FrozenAuthority::live(),
                )
                .map_err(KernelError::from)
            },
            |retained| retained.validate(sheet),
        )
    }
}

impl<'a> CounterSheet<'a> {
    #[must_use]
    pub const fn new(sheet: &'a mut StyleSheet, evidence: &'a mut KernelEvidence) -> Self {
        Self { sheet, evidence }
    }

    /// Run one generative attempt. No capability or sheet borrow can escape as O.
    ///
    /// # Errors
    /// Rejects invalid BASE before build and invalid final authority after build.
    /// # Panics
    /// Propagates caller panics after CSS rollback followed by typed sheet rollback.
    pub fn with_attempt<O, E>(
        &mut self,
        build: impl for<'id> FnOnce(
            KernelAttempt<'id>,
        ) -> Result<CompletedUpdate<'id, O>, UpdateError<E>>,
    ) -> Result<O, UpdateError<E>> {
        css::admission::with_admission(|| {
            let mut publication = Publication::new(self.sheet, self.evidence);
            publication.evidence.validate(publication.sheet)?;
            let output = css::exact_attempt::with_exclusive_attempt(|| {
                let (pending, output) = {
                    let CompletedUpdate {
                        sheet,
                        pending,
                        output,
                        brand: _,
                    } = build(KernelAttempt {
                        sheet: publication.sheet,
                        evidence: publication.evidence,
                        brand: PhantomData,
                    })?;
                    pending.validate(sheet)?;
                    (pending, output)
                };
                *publication.evidence = pending;
                Ok::<O, UpdateError<E>>(output)
            })?;
            publication.commit();
            Ok(output)
        })
    }
}

impl<'id> KernelAttempt<'id> {
    /// Revalidate BASE immediately before the single genuine producer traversal.
    /// # Errors
    /// Rejects stale BASE, invalid IR policy, seed, lookup or actual receipt authority.
    pub fn prepare(
        self,
        styles: &FxHashSet<ExtractStyleValue>,
        request: UpdateRequest<'_>,
    ) -> Result<PreparedUpdate<'id>, KernelError> {
        let base = self.evidence.validate(self.sheet)?;
        let (pending, effects) = prepare::prepare(
            self.sheet,
            prepare::Preparation {
                base: &base,
                evidence: self.evidence,
                styles,
                request,
            },
        )?;
        Ok(PreparedUpdate {
            sheet: self.sheet,
            pending,
            effects,
            brand: self.brand,
        })
    }
}

impl<'id> PreparedUpdate<'id> {
    /// Construct output from the real prospective sheet with its existing Theme.
    /// # Errors
    /// Returns output errors; final linkage is checked after the enclosing callback.
    pub fn finish<O, E>(
        self,
        build_output: impl FnOnce(&StyleSheet, UpdateEffects) -> Result<O, E>,
    ) -> Result<CompletedUpdate<'id, O>, UpdateError<E>> {
        let output = build_output(self.sheet, self.effects).map_err(UpdateError::Output)?;
        Ok(CompletedUpdate {
            sheet: self.sheet,
            pending: self.pending,
            output,
            brand: self.brand,
        })
    }
}
