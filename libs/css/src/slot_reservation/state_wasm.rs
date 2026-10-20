use super::{Phase, State};

thread_local! {
    static STATE: std::cell::RefCell<State> = const {
        std::cell::RefCell::new(State { phase: Phase::Legacy, misses: 0 })
    };
}

pub(super) fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with_borrow_mut(f)
}
