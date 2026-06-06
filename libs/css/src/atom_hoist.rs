use std::sync::atomic::{AtomicUsize, Ordering};

// Atom-level hoist threshold. 0 = disabled (default). N = a style atom whose
// content is used by >= N distinct routes is emitted into the shared global
// devup-ui.css (shipped once) instead of duplicated across per-route chunks.
static ATOM_HOIST_THRESHOLD: AtomicUsize = AtomicUsize::new(0);

#[inline(always)]
pub fn set_atom_hoist(threshold: Option<usize>) {
    ATOM_HOIST_THRESHOLD.store(threshold.unwrap_or(0), Ordering::Relaxed);
}

#[inline(always)]
#[must_use]
pub fn atom_hoist_threshold() -> Option<usize> {
    match ATOM_HOIST_THRESHOLD.load(Ordering::Relaxed) {
        0 => None,
        v => Some(v),
    }
}

#[inline(always)]
#[must_use]
pub fn is_atom_hoist() -> bool {
    ATOM_HOIST_THRESHOLD.load(Ordering::Relaxed) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_atom_hoist() {
        set_atom_hoist(None);
        assert!(!is_atom_hoist());
        assert_eq!(atom_hoist_threshold(), None);
        set_atom_hoist(Some(3));
        assert!(is_atom_hoist());
        assert_eq!(atom_hoist_threshold(), Some(3));
        set_atom_hoist(None);
        assert!(!is_atom_hoist());
    }
}
