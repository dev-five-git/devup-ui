use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG: AtomicBool = AtomicBool::new(false);

#[inline(always)]
pub fn set_debug(value: bool) {
    DEBUG.store(value, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_debug() -> bool {
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
