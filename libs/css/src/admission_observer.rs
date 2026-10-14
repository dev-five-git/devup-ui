//! Gate-arrival witness only; this does not participate in admission decisions.

use std::{
    collections::HashSet,
    sync::{Condvar, LazyLock, Mutex},
    thread::ThreadId,
};

static BLOCKED: LazyLock<Mutex<HashSet<ThreadId>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static CHANGED: Condvar = Condvar::new();

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn arriving() {
    let mut threads = BLOCKED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    threads.insert(std::thread::current().id());
    drop(threads);
    CHANGED.notify_all();
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn acquired() {
    let mut threads = BLOCKED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    threads.remove(&std::thread::current().id());
    drop(threads);
    CHANGED.notify_all();
}

/// Wait for this particular thread to arrive before the normal outer gate lock.
/// The observing test must own admission so the contender cannot acquire it.
pub fn wait_for_thread(thread: ThreadId) {
    let blocked = BLOCKED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    drop(
        CHANGED
            .wait_while(blocked, |blocked| !blocked.contains(&thread))
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
}
