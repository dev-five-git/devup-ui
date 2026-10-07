//! Cross-crate test cleanup; retain the existing serial lock and keep scopes on
//! their creating thread. Only reset at quiescent extraction boundaries.

/// Clear imported evaluation and TypeScript caches on the calling thread.
pub fn reset_caches_for_testing() {
    crate::vanilla_extract::test_state::reset();
}

/// Clean CSS state and extraction caches, restoring both on scope exit/unwind.
#[must_use]
pub struct TestStateGuard {
    _caches: crate::vanilla_extract::test_state::CacheGuard,
    _css: css::test_state::TestStateGuard,
}

impl TestStateGuard {
    /// Own snapshots without holding locks across extraction.
    pub fn new() -> Self {
        Self {
            _caches: crate::vanilla_extract::test_state::CacheGuard::new(),
            _css: css::test_state::TestStateGuard::new(),
        }
    }
}

impl Default for TestStateGuard {
    fn default() -> Self {
        Self::new()
    }
}
