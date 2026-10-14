use std::cell::Cell;

thread_local! {
    static HELD: Cell<bool> = const { Cell::new(false) };
}

/// Marks only the live sheet data lock, not the enclosing admission scope.
pub(crate) struct SheetEntry;

impl SheetEntry {
    pub(crate) fn enter() -> Self {
        HELD.with(|held| {
            assert!(!held.get(), "style sheet: recursive lock-held entry");
            held.set(true);
        });
        Self
    }
}

impl Drop for SheetEntry {
    fn drop(&mut self) {
        HELD.with(|held| held.set(false));
    }
}
