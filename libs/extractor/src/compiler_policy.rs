use crate::extract_style::compiler_projection as projection;
use crate::extract_style::compiler_receipts::{self as receipts, CompilerReceiptBatch};
use crate::import_alias_visit::Edit;
use crate::provenance::SiteScope;
use crate::{Evaluated, ExtractOption, ExtractOutput, ModuleResolver};
use std::{cell::Cell, collections::BTreeSet, error::Error};

/// Existing compiler inputs, with the exact received filename.
pub struct CompilerInput<'a> {
    pub filename: &'a str,
    pub code: &'a str,
    pub option: ExtractOption,
    pub source_map: bool,
    pub resolver: Option<&'a ModuleResolver>,
}
/// JavaScript metadata; final styles belong exclusively to the sealed batch.
pub struct CompilerOutputMetadata {
    pub code: String,
    pub map: Option<String>,
    pub css_file: Option<String>,
    pub dependencies: Vec<String>,
}
/// Compilation and consumer failure remain distinct.
#[derive(Debug)]
pub enum CounterCompileError<E> {
    Compile(Box<dyn Error>),
    Consumer(E),
}
thread_local! {
    static ORIGINAL: Cell<Option<u32>> = const { Cell::new(None) };
}
struct Invocation(Option<u32>);
impl Drop for Invocation {
    fn drop(&mut self) {
        ORIGINAL.set(self.0);
    }
}
pub(crate) fn active() -> bool {
    ORIGINAL.get().is_some()
}
pub(crate) fn current(input: CompilerInput<'_>) -> Result<ExtractOutput, Box<dyn Error>> {
    let _invocation = Invocation(ORIGINAL.replace(None));
    let _sites = SiteScope::suspend_counter();
    let _journal = receipts::JournalScope::replace(None);
    let source = crate::provenance::normalize_source(input.code);
    invoke(&input, &source, None)
}
fn invoke(
    input: &CompilerInput<'_>,
    source: &str,
    evaluated: Option<Evaluated<'_>>,
) -> Result<ExtractOutput, Box<dyn Error>> {
    crate::extract_source(
        input.filename,
        source,
        evaluated,
        input.option.clone(),
        input.source_map,
        input.resolver,
    )
}
pub(crate) fn inherited(input: CompilerInput<'_>) -> Result<ExtractOutput, Box<dyn Error>> {
    if !active() {
        return current(input);
    }
    let _reservation = receipts::ReservationScope::enter();
    let result = registered(input);
    if let Err(error) = &result {
        receipts::retain_terminal(error.as_ref());
    }
    result
}
fn registered(input: CompilerInput<'_>) -> Result<ExtractOutput, Box<dyn Error>> {
    let original = css::file_map::get_or_insert_original_id(input.filename)?;
    let _invocation = Invocation(ORIGINAL.replace(Some(original)));
    let _raw =
        crate::extract_style::compiler_associations::RawScope::enter(input.filename, original);
    let source = crate::provenance::normalize_source(input.code);
    let _sites = SiteScope::enter_counter_numbered(original, &source, &[]);
    probe(&input, &source)
}
/// Compile through the real parser and consume a generatively branded receipt batch.
///
/// # Errors
/// Compilation or consumption failure rolls back the complete naming attempt.
pub fn with_counter_extract<O, E>(
    input: CompilerInput<'_>,
    consume: impl for<'compile> FnOnce(
        CompilerOutputMetadata,
        CompilerReceiptBatch<'compile>,
    ) -> Result<O, E>,
) -> Result<O, CounterCompileError<E>> {
    css::exact_attempt::with_exclusive_attempt(|| {
        let _journal = receipts::JournalScope::replace(Some(receipts::Journal::new()));
        let output = registered(input).map_err(CounterCompileError::Compile)?;
        let metadata = CompilerOutputMetadata {
            code: output.code,
            map: output.map,
            css_file: output.css_file,
            dependencies: output.dependencies,
        };
        receipts::with_sealed(output.styles, |batch| consume(metadata, batch))
    })
}
fn probe(input: &CompilerInput<'_>, source: &str) -> Result<ExtractOutput, Box<dyn Error>> {
    let result: Result<_, Box<dyn Error>> = css::exact_attempt::with_exclusive_attempt(|| {
        let mut frame = receipts::Probe::enter();
        let output = invoke(input, source, None);
        projection::check_terminal(input.filename, source, &[])?;
        let output = output?;
        frame.commit();
        Ok(output)
    });
    match result {
        Ok(output) => Ok(output),
        Err(error) => match error.downcast::<RetryPlan>() {
            Err(error) => Err(error),
            Ok(plan) => continue_retry(input, source, *plan),
        },
    }
}
#[derive(Debug)]
struct RetryPlan {
    computed: String,
    value_edits: Vec<Edit>,
    alias_edits: Vec<Edit>,
    read: BTreeSet<String>,
    risky: Vec<(usize, usize)>,
}
impl std::fmt::Display for RetryPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("deferred Counter evaluation continuation")
    }
}
impl Error for RetryPlan {}
type RetryValues = (String, Vec<Edit>, BTreeSet<String>, Vec<(usize, usize)>);
pub(crate) fn retry(
    (computed, value_edits, read, risky): RetryValues,
    alias_edits: &[Edit],
) -> Box<dyn Error> {
    Box::new(RetryPlan {
        computed,
        value_edits,
        alias_edits: alias_edits.to_vec(),
        read,
        risky,
    })
}
fn continue_retry(
    input: &CompilerInput<'_>,
    source: &str,
    plan: RetryPlan,
) -> Result<ExtractOutput, Box<dyn Error>> {
    #[cfg(test)]
    tests::observe(true);
    css::exact_attempt::with_exclusive_attempt(|| {
        let mut frame = receipts::Probe::enter();
        let edits = [plan.value_edits.as_slice(), plan.alias_edits.as_slice()];
        let output = invoke(
            input,
            &plan.computed,
            Some(Evaluated {
                source,
                edits: &edits,
                risky: &plan.risky,
            }),
        );
        projection::check_terminal(input.filename, source, &edits)?;
        let mut output = output?;
        let mut dependencies: BTreeSet<_> = output.dependencies.into_iter().collect();
        dependencies.extend(plan.read);
        output.dependencies = dependencies.into_iter().collect();
        frame.commit();
        Ok(output)
    })
}
pub(crate) fn sites(source: (&str, &str, bool), edits: &[&[Edit]]) -> SiteScope {
    match ORIGINAL.get() {
        None => SiteScope::enter(source.0, source.1, edits),
        Some(original) if source.2 => SiteScope::enter_counter_generated(original, source.1),
        Some(original) => SiteScope::enter_counter_numbered(original, source.1, edits),
    }
}
#[cfg(test)]
#[path = "compiler_policy_tests.rs"]
pub(crate) mod tests;
