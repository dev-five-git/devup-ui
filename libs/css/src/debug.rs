use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG: AtomicBool = AtomicBool::new(false);

#[inline(always)]
pub fn set_debug(value: bool) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("set_debug");
    DEBUG.store(value, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_debug() -> bool {
    let _admission = crate::admission::enter();
    DEBUG.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_set_debug() {
        set_debug(true);
        assert!(is_debug());
        set_debug(false);
        assert!(!is_debug());
    }
}
