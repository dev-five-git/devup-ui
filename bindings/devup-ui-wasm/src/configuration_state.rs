use std::{cell::RefCell, collections::HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Field {
    Debug,
    Prefix,
    Canonical,
    Routes,
    Threshold,
    Theme,
    Shorthands,
}

thread_local! {
    static APPLIED: RefCell<HashSet<Field>> = RefCell::default();
}

pub(crate) fn applied(field: Field, changed: bool) -> bool {
    let first = APPLIED.with_borrow_mut(|fields| fields.insert(field));
    if changed {
        extractor::invalidate_evaluation_cache();
    }
    first || changed
}

pub(crate) fn replaced() {
    APPLIED.with_borrow_mut(HashSet::clear);
    extractor::invalidate_evaluation_cache();
}

#[cfg(test)]
pub(crate) struct TestStateGuard(HashSet<Field>);

#[cfg(test)]
impl TestStateGuard {
    pub(crate) fn new() -> Self {
        Self(APPLIED.with_borrow_mut(std::mem::take))
    }
}

#[cfg(test)]
impl Drop for TestStateGuard {
    fn drop(&mut self) {
        APPLIED.with_borrow_mut(|fields| *fields = std::mem::take(&mut self.0));
    }
}
