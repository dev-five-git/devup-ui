mod capabilities;
mod decoder;
mod diagnostic;
mod forward;
mod source_type;
mod state;
mod wiring;

use wasm_bindgen::JsValue;

pub(crate) use capabilities::Resolver;
pub(crate) use state::FaultScope as Scope;
#[cfg(test)]
pub(crate) use state::State;
pub(crate) use wiring::RESOLVER;

thread_local! {
    static STATE: state::State<JsValue> = const { state::State::new() };
}

pub(crate) fn clear_cause() {
    STATE.with(state::State::clear_cause);
}

pub(crate) fn extraction_error<V, E: Into<V>>(
    message: &str,
    new_error: fn(&str) -> E,
    attach: fn(V) -> V,
) -> V {
    attach(new_error(message).into())
}

pub(crate) fn attach_cause(error: JsValue) -> JsValue {
    STATE.with(|state| {
        state.attach_cause(
            error,
            &crate::resident_owner::ERRORS,
            crate::resident_owner::Transport::attach_cause,
        )
    })
}

pub(crate) fn importer() -> Option<String> {
    STATE.with(state::State::importer)
}

pub(crate) fn retain_importer() {
    STATE.with(state::State::retain_importer);
}

#[cfg(test)]
mod decoder_tests;
#[cfg(test)]
mod diagnostic_tests;
#[cfg(test)]
pub(crate) mod native_tests;
#[cfg(test)]
mod source_type_tests;
#[cfg(test)]
mod state_tests;
#[cfg(test)]
pub(crate) mod value_tests;
