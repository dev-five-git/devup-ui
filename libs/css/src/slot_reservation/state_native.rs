use super::{Phase, State};

static STATE: std::sync::Mutex<State> = std::sync::Mutex::new(State {
    phase: Phase::Legacy,
    misses: 0,
});

pub(super) fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    f(&mut STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}
