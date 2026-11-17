//! Rust panics reach JavaScript as readable errors.
//!
//! The release profile aborts on panic, and wasm32 has no stderr, so without a
//! hook a panic is an opaque `RuntimeError: unreachable` with no message or
//! location. The hook throws a JS `Error` from inside the panic instead: the
//! call that hit it fails with the panic message and where it happened.

use std::panic::PanicHookInfo;

/// The message a panic reaches JavaScript with: what it is, the panic itself
/// (message and Rust location), and what the caller can do.
#[must_use]
pub fn panic_message(info: &PanicHookInfo<'_>) -> String {
    format!(
        "devup-ui internal error: {info}. This is a bug in Devup UI, not in your code: please report it at https://github.com/dev-five-git/devup-ui/issues with the file being compiled. The compiler state may now be inconsistent: restart the build."
    )
}

/// Install the hook once, when the module is instantiated.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        wasm_bindgen::throw_str(&panic_message(info));
    }));
}

#[cfg(test)]
mod tests {
    use super::panic_message;
    use serial_test::serial;
    use std::sync::mpsc::channel;

    #[test]
    #[serial]
    fn a_panic_names_the_bug_its_cause_and_the_way_out() {
        let (sender, messages) = channel();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = sender.send(panic_message(info));
        }));
        let result = std::panic::catch_unwind(|| panic!("index out of bounds"));
        std::panic::set_hook(previous);

        assert!(result.is_err());
        let message = messages.try_recv().unwrap_or_default();
        assert!(
            message.starts_with("devup-ui internal error: panicked at "),
            "{message}"
        );
        assert!(message.contains("panic.rs:"), "{message}");
        assert!(message.contains("index out of bounds"), "{message}");
        assert!(message.ends_with("restart the build."), "{message}");
    }
}
