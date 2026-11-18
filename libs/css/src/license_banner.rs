//! Whether emitted stylesheets start with the `/*! devup-ui ... */` license banner.

use std::sync::atomic::{AtomicBool, Ordering};

static LICENSE_BANNER: AtomicBool = AtomicBool::new(true);

/// Turn the license banner of emitted stylesheets on or off (on by default).
#[inline(always)]
pub fn set_license_banner(value: bool) {
    LICENSE_BANNER.store(value, Ordering::Relaxed);
}

/// Whether emitted stylesheets start with the license banner.
#[inline(always)]
pub fn is_license_banner() -> bool {
    LICENSE_BANNER.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_set_license_banner() {
        let _state = crate::test_state::TestStateGuard::new();
        assert!(is_license_banner());
        set_license_banner(false);
        assert!(!is_license_banner());
        set_license_banner(true);
        assert!(is_license_banner());
    }
}
