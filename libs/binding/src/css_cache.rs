//! Serialized stylesheets, reused until the sheet or a setting they read changes.
//!
//! A build serializes the same sheet many times between two extractions: the
//! transform that changed it, the shared base the host writes next, and every
//! stylesheet module the bundler loads. Each entry is exactly what
//! `StyleSheet::create_css` returned for its key, so a hit is byte-identical.

use sheet::StyleSheet;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{LazyLock, LockResult, Mutex, MutexGuard, PoisonError};

/// What a serialization reads besides the sheet: the bucket, whether a file
/// stylesheet imports the shared base, whether atom hoisting is in effect, and
/// whether the license banner is on.
type Key = (Option<String>, bool, bool, bool);

static CSS_CACHE: LazyLock<Mutex<HashMap<Key, String>>> = LazyLock::new(Mutex::default);

fn cache() -> MutexGuard<'static, HashMap<Key, String>> {
    CSS_CACHE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The key of a serialization; only a file stylesheet imports the shared base.
fn key(bucket: Option<&str>, import_main_css: bool) -> Key {
    (
        bucket.map(str::to_string),
        bucket.is_some() && import_main_css,
        css::atom_hoist::is_atom_hoist(),
        css::license_banner::is_license_banner(),
    )
}

/// Serialize `bucket` (the shared base when `None`), reusing an unchanged result.
pub(crate) fn cached_css(
    sheet: &StyleSheet,
    bucket: Option<&str>,
    import_main_css: bool,
) -> String {
    cache()
        .entry(key(bucket, import_main_css))
        .or_insert_with(|| sheet.create_css(bucket, import_main_css))
        .clone()
}

/// Forget every serialization; called whenever the sheet or a setting it reads changes.
pub(crate) fn invalidate_css() {
    cache().clear();
}

/// A stylesheet behind a mutex whose every mutable borrow forgets its serializations.
#[derive(Default)]
pub struct SheetCell(Mutex<StyleSheet>);

/// A locked stylesheet; borrowing it mutably forgets its serializations.
pub struct SheetGuard<'a>(MutexGuard<'a, StyleSheet>);

impl SheetCell {
    /// Lock the stylesheet as `Mutex::lock` does.
    ///
    /// # Errors
    /// The stylesheet, still usable, when a holder of the lock panicked.
    pub fn lock(&self) -> LockResult<SheetGuard<'_>> {
        self.0
            .lock()
            .map(SheetGuard)
            .map_err(|poisoned| PoisonError::new(SheetGuard(poisoned.into_inner())))
    }
}

impl SheetGuard<'_> {
    /// Borrow the sheet mutably, keeping its serializations for the caller to forget.
    pub(crate) fn untracked(&mut self) -> &mut StyleSheet {
        &mut self.0
    }
}

impl Deref for SheetGuard<'_> {
    type Target = StyleSheet;

    fn deref(&self) -> &StyleSheet {
        &self.0
    }
}

impl DerefMut for SheetGuard<'_> {
    fn deref_mut(&mut self) -> &mut StyleSheet {
        invalidate_css();
        &mut self.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn only_file_stylesheets_import_the_shared_base() {
        assert_eq!(key(None, true), key(None, false));
        assert_ne!(key(Some("a.tsx"), true), key(Some("a.tsx"), false));
    }

    #[test]
    #[serial_test::serial]
    fn the_license_banner_setting_keys_a_serialization() {
        let _state = css::test_state::TestStateGuard::new();
        invalidate_css();
        let mut sheet = StyleSheet::default();
        sheet.add_property("a", "color", 0, "red", None, Some(255), None);
        let with_banner = cached_css(&sheet, None, false);
        assert!(with_banner.starts_with("/*! devup-ui v"));
        // A serialization cached with the banner is not served without it
        css::license_banner::set_license_banner(false);
        assert_eq!(cached_css(&sheet, None, false), ".a{color:red}");
        css::license_banner::set_license_banner(true);
        assert_eq!(cached_css(&sheet, None, false), with_banner);
        invalidate_css();
    }

    #[test]
    fn a_poisoned_sheet_stays_usable() {
        let cell = SheetCell::default();
        let _ = std::panic::catch_unwind(|| {
            let _guard = cell.lock();
            panic!("poison the sheet");
        });
        let mut sheet = cell.lock().err().map(PoisonError::into_inner).unwrap();
        sheet.add_import("a.tsx", "b.css");
        assert_eq!(sheet.imports.len(), 1);
    }
}
