use super::{FxHashMap, IMPORTED_RUNS, STRIPPED};

pub(crate) fn reset() {
    IMPORTED_RUNS.with_borrow_mut(FxHashMap::clear);
    STRIPPED.with_borrow_mut(FxHashMap::clear);
}

#[cfg(test)]
mod tests {
    use super::super::{CacheGuard, CollectedStyles};
    use super::*;

    #[test]
    fn caches_restore_when_nested_and_unwinding() {
        let _state = CacheGuard::default();
        super::super::strip_typescript("export const n: number = 1", "outer.ts");
        IMPORTED_RUNS.with_borrow_mut(|runs| {
            runs.insert(
                "outer.css.ts".into(),
                (1, "source".into(), "run".into(), CollectedStyles::default()),
            );
        });
        let baseline = STRIPPED.with_borrow(Clone::clone);
        let result = std::panic::catch_unwind(|| {
            let _nested = CacheGuard::default();
            assert_eq!(IMPORTED_RUNS.with_borrow(FxHashMap::len), 0);
            super::super::strip_typescript("export const n: number = 2", "inner.ts");
            panic!("test unwind");
        });
        assert!(result.is_err());
        assert_eq!(STRIPPED.with_borrow(Clone::clone), baseline);
        assert_eq!(IMPORTED_RUNS.with_borrow(|runs| runs["outer.css.ts"].0), 1);
        let nested = CacheGuard::default();
        drop(nested);
        assert_eq!(STRIPPED.with_borrow(Clone::clone), baseline);
    }

    #[test]
    fn caches_clear_when_populated_and_empty() {
        let _state = CacheGuard::default();
        super::super::strip_typescript("export const n: number = 1", "outer.ts");
        IMPORTED_RUNS.with_borrow_mut(|runs| {
            runs.insert(
                "outer.css.ts".into(),
                (1, "source".into(), "run".into(), CollectedStyles::default()),
            );
        });
        reset();
        assert_eq!(STRIPPED.with_borrow(FxHashMap::len), 0);
        assert_eq!(IMPORTED_RUNS.with_borrow(FxHashMap::len), 0);
        reset();
        assert_eq!(STRIPPED.with_borrow(FxHashMap::len), 0);
        assert_eq!(IMPORTED_RUNS.with_borrow(FxHashMap::len), 0);
    }
}
