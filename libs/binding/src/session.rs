//! Engine sessions: independent engine states in one process.
//!
//! The engine's globals always hold one session's state, the live session.
//! Every other session's state is parked here. Switching to a session
//! exchanges its parked state with the live globals, so the code that reads
//! them does not know about sessions. Session 0 exists from the start; until a
//! host creates another, the engine behaves as a single process-wide engine.
//!
//! The parked CSS numbering and stylesheet may cross threads. The extractor's
//! stylesheet caches stay on the thread that filled them, so each thread keeps
//! its own per-session caches and aligns them on every switch.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use css::engine_state::CssState;
use extractor::ExtractorCaches;
use sheet::StyleSheet;

use crate::with_style_sheet_mut;

/// Everything one session keeps between calls, apart from thread-bound caches:
/// the CSS crate's numbering, registries and settings, and the stylesheet.
#[derive(Default)]
pub struct EngineState {
    css: CssState,
    sheet: StyleSheet,
}

impl EngineState {
    /// Exchange this state with the live one.
    pub fn swap_live(&mut self) {
        self.css.swap_live();
        with_style_sheet_mut(|live| std::mem::swap(live, &mut self.sheet));
    }
}

struct Sessions {
    next: u32,
    parked: BTreeMap<u32, EngineState>,
}

#[derive(Default)]
struct ThreadCaches {
    live: u32,
    parked: BTreeMap<u32, ExtractorCaches>,
}

/// The session whose state the globals hold.
static LIVE: AtomicU32 = AtomicU32::new(0);

static SESSIONS: Mutex<Sessions> = Mutex::new(Sessions {
    next: 1,
    parked: BTreeMap::new(),
});

/// Held while [`with_session`] runs, so no other thread switches under it.
static RUNNING: Mutex<()> = Mutex::new(());

thread_local! {
    static CACHES: RefCell<ThreadCaches> = RefCell::default();
    /// Whether this thread is inside [`with_session`]
    static INSIDE: Cell<bool> = const { Cell::new(false) };
}

fn sessions() -> MutexGuard<'static, Sessions> {
    SESSIONS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn running() -> MutexGuard<'static, ()> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner)
}

fn unknown(id: u32) -> String {
    format!("engine session {id} does not exist: create it with createSession() first")
}

fn busy(live: u32) -> String {
    format!("engine session {live} is running a call: switch sessions only after that call returns")
}

/// The session whose state is live.
pub fn live_session() -> u32 {
    LIVE.load(Ordering::Acquire)
}

/// Park the live state and make session `id` live, returning the session
/// that was live.
fn switch(id: u32) -> Result<u32, String> {
    let live = live_session();
    if id != live {
        // Callers hold the session lock, so the state can be swapped unlocked
        let mut state = sessions().parked.remove(&id).ok_or_else(|| unknown(id))?;
        state.swap_live();
        sessions().parked.insert(live, state);
        LIVE.store(id, Ordering::Release);
    }
    align_caches(id);
    Ok(live)
}

/// Make this thread's live caches those of session `id`.
fn align_caches(id: u32) {
    CACHES.with_borrow_mut(|caches| {
        if caches.live != id {
            let mut next = caches.parked.remove(&id).unwrap_or_default();
            next.swap_live();
            caches.parked.insert(caches.live, next);
            caches.live = id;
        }
    });
}

/// Create a session with a clean engine state, returning its id.
///
/// Ids are never handed out twice, even after the session is dropped.
pub fn create_session() -> u32 {
    let mut sessions = sessions();
    let id = sessions.next;
    sessions.next = id.saturating_add(1);
    sessions.parked.insert(id, EngineState::default());
    id
}

/// Make session `id` live until another is selected, returning the session
/// that was live.
///
/// # Errors
/// When the session does not exist, or a [`with_session`] call is running on
/// this thread with another session live.
pub fn select_session(id: u32) -> Result<u32, String> {
    if INSIDE.get() {
        let live = live_session();
        return if id == live {
            Ok(live)
        } else {
            Err(busy(live))
        };
    }
    let _running = running();
    switch(id)
}

/// Drop session `id` and its state, returning whether it existed. Dropping
/// the live session makes session 0 live first.
///
/// Caches the session left on other threads go when those threads end.
///
/// # Errors
/// For session 0, and when the session is live inside a running
/// [`with_session`] call.
pub fn drop_session(id: u32) -> Result<bool, String> {
    if id == 0 {
        return Err("engine session 0 cannot be dropped".to_string());
    }
    if live_session() == id {
        select_session(0)?;
    }
    CACHES.with_borrow_mut(|caches| caches.parked.remove(&id));
    Ok(sessions().parked.remove(&id).is_some())
}

/// Switches back to the session that was live, also on unwind.
struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) {
        INSIDE.set(false);
        switch(self.0).ok();
    }
}

/// Run `f` with session `id` live, then make the previous session live
/// again, holding the session lock throughout.
///
/// A nested call on the same thread runs directly when `id` is already live.
///
/// # Errors
/// When the session does not exist, or a nested call names another session.
pub fn with_session<R>(id: u32, f: impl FnOnce() -> R) -> Result<R, String> {
    if INSIDE.get() {
        let live = live_session();
        return if id == live { Ok(f()) } else { Err(busy(live)) };
    }
    let _running = running();
    let _restore = Restore(switch(id)?);
    INSIDE.set(true);
    Ok(f())
}

/// Forget every session but 0 and make it live, for the full test reset that
/// then clears the live state.
pub fn reset_sessions() {
    sessions().parked.clear();
    LIVE.store(0, Ordering::Release);
    CACHES.with_borrow_mut(|caches| *caches = ThreadCaches::default());
}

/// A value a host adapter keeps for each session itself, such as the module
/// resolver it calls back into: the live session's value is the one in use.
#[derive(Debug)]
pub struct PerSession<T>(BTreeMap<u32, T>);

impl<T> Default for PerSession<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PerSession<T> {
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl<T: Clone> PerSession<T> {
    /// The live session's value.
    pub fn live(&self) -> Option<T> {
        self.0.get(&live_session()).cloned()
    }

    /// Set or clear the live session's value.
    pub fn set_live(&mut self, value: Option<T>) {
        let live = live_session();
        match value {
            Some(value) => self.0.insert(live, value),
            None => self.0.remove(&live),
        };
    }

    /// Forget a dropped session's value.
    pub fn remove(&mut self, id: u32) {
        self.0.remove(&id);
    }
}

#[cfg(test)]
mod tests;
