use boa_engine::JsNativeError;

use super::FinalizeError;

#[test]
fn located_failure_reports_its_message_as_a_type_error() {
    let message = "/a.css.ts:1:1: required native input `x` may be changed. Fix: keep it immutable";

    let error = FinalizeError::Located(message.into()).into_js();

    assert!(
        error
            .to_string()
            .starts_with(&format!("TypeError: {message}")),
        "{error}"
    );
}

#[test]
fn script_failure_keeps_its_own_error() {
    let original = JsNativeError::range().with_message("count out of range");

    let error = FinalizeError::Js(original.into()).into_js();

    assert!(
        error
            .to_string()
            .starts_with("RangeError: count out of range"),
        "{error}"
    );
}
