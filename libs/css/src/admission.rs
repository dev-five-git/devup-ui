//! Synchronous ownership of CSS state, before any individual data root is acquired.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Mutex, MutexGuard};
use std::{cell::Cell, marker::PhantomData, rc::Rc};

#[cfg(not(target_arch = "wasm32"))]
static GATE: Mutex<()> = Mutex::new(());
thread_local! {
    static OWNER_DEPTH: Cell<usize> = const { Cell::new(0) };
    static EXACT_DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub(crate) struct Lease {
    #[cfg(not(target_arch = "wasm32"))]
    _guard: Option<MutexGuard<'static, ()>>,
    _thread: PhantomData<Rc<()>>,
}

pub(crate) fn enter() -> Lease {
    let nested = OWNER_DEPTH.with(|depth| depth.get() > 0);
    #[cfg(not(target_arch = "wasm32"))]
    let guard = if nested {
        None
    } else {
        #[cfg(any(test, feature = "admission-observer"))]
        observer::arriving();
        let guard = GATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        #[cfg(any(test, feature = "admission-observer"))]
        observer::acquired();
        Some(guard)
    };
    #[cfg(target_arch = "wasm32")]
    let _ = nested;
    OWNER_DEPTH.with(|depth| depth.set(depth.get() + 1));
    Lease {
        #[cfg(not(target_arch = "wasm32"))]
        _guard: guard,
        _thread: PhantomData,
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        OWNER_DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}

/// Hold native CSS admission through a synchronous callback.
///
/// Same-owner nesting skips only the admission mutex, never a data-root lock.
/// Do not suspend or wait for a competing CSS caller inside the callback.
pub fn with_admission<R>(build: impl FnOnce() -> R) -> R {
    let _lease = enter();
    build()
}

/// Reject owner administration that would invalidate an exact attempt's inputs.
/// The caller must obtain admission first.
///
/// # Panics
/// Panics with `{operation}: an exact attempt is active` before mutation.
/// Later linked activation must use a typed error where this is recoverable.
#[doc(hidden)]
pub fn assert_administration_allowed(operation: &str) {
    EXACT_DEPTH.with(|depth| {
        assert!(depth.get() == 0, "{operation}: an exact attempt is active");
    });
}

pub(crate) struct ExactScope(PhantomData<Rc<()>>);

impl ExactScope {
    pub(crate) fn enter() -> Self {
        EXACT_DEPTH.with(|depth| depth.set(depth.get() + 1));
        Self(PhantomData)
    }
}

impl Drop for ExactScope {
    fn drop(&mut self) {
        EXACT_DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}

#[cfg(any(test, feature = "admission-observer"))]
#[doc(hidden)]
#[path = "admission_observer.rs"]
pub mod observer;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    #[test]
    fn permits_immutable_reborrows_when_wasm_callbacks_hold_refcell_reads() {
        // Given / When / Then
        crate::class_map::with_class_map(|outer| {
            crate::class_map::with_class_map(|inner| assert_eq!(inner, outer));
        });
        crate::file_map::with_file_map(|outer| {
            crate::file_map::with_file_map(|inner| assert_eq!(inner, outer));
        });
        crate::file_map::with_canonical_map(|outer| {
            crate::file_map::with_canonical_map(|inner| assert_eq!(inner, outer));
        });
        crate::file_routes::with_file_routes(|outer| {
            crate::file_routes::with_file_routes(|inner| assert_eq!(inner, outer));
        });
        crate::with_prefix(|outer| {
            crate::with_prefix(|inner| assert_eq!(inner, outer));
        });
    }
}
