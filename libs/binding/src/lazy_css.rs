//! The CSS a transform returns, serialized only when the host reads it.
//!
//! Most hosts never read it, and serializing the shared base after every
//! transform is quadratic over a single-CSS build. The value is still the sheet
//! as that transform left it: the sheet, and every setting a serialization
//! reads, changes only after the one unread result has been serialized.

use css::atom_hoist::{AtomModeGuard, is_suppressed};
use sheet::StyleSheet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use crate::css_cache::cached_css;
use crate::with_style_sheet;

struct Pending {
    bucket: Option<String>,
    import_main_css: bool,
    /// Whether the transform suppressed atom mode while it committed.
    suppressed: bool,
    value: Mutex<Option<String>>,
}

/// The latest transform result, which alone can describe the current sheet.
static PENDING: Mutex<Weak<Pending>> = Mutex::new(Weak::new());

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Pending {
    /// The value, serializing `sheet` under the transform's atom mode on the first call.
    fn value(&self, sheet: &StyleSheet) -> String {
        lock(&self.value)
            .get_or_insert_with(|| {
                let _mode = AtomModeGuard::exact(self.suppressed);
                cached_css(sheet, self.bucket.as_deref(), self.import_main_css)
            })
            .clone()
    }
}

/// CSS of a transform, serialized on its first read.
pub struct LazyCss(Arc<Pending>);

impl LazyCss {
    /// Defer serializing `bucket` (the shared base when `None`) of the sheet as it stands.
    pub(crate) fn defer(bucket: Option<&str>, import_main_css: bool) -> Self {
        let pending = Arc::new(Pending {
            bucket: bucket.map(str::to_string),
            import_main_css,
            suppressed: is_suppressed(),
            value: Mutex::default(),
        });
        *lock(&PENDING) = Arc::downgrade(&pending);
        Self(pending)
    }

    /// The CSS of the sheet as the transform left it.
    #[must_use]
    pub fn get(&self) -> String {
        with_style_sheet(|sheet| self.0.value(sheet))
    }
}

/// Serialize the unread result of the latest transform before `sheet`, or a
/// setting its serialization reads, changes. A dropped result costs nothing.
pub(crate) fn settle(sheet: &StyleSheet) {
    let pending = std::mem::take(&mut *lock(&PENDING)).upgrade();
    if let Some(pending) = pending {
        pending.value(sheet);
    }
}

/// Settle before a setting that serializations read changes.
pub(crate) fn settle_live() {
    with_style_sheet(settle);
}
