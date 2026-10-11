use boa_engine::{Context, JsNativeError, JsResult, JsValue, js_string};

use super::{js_str, to_text};

/// Non-final fallback arguments match upstream's anchored `var(--.*)` pattern.
pub(super) fn fallback_var(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let mut values = args.iter().rev();
    let mut result = match values.next() {
        Some(value) => to_text(value, context)?,
        None => String::new(),
    };
    for value in values {
        let reference = js_str(value);
        let head = reference.as_deref().and_then(|reference| {
            reference
                .strip_prefix("var(--")
                .filter(|body| !body.contains(['\n', '\r', '\u{2028}', '\u{2029}']))
                .and_then(|_| reference.strip_suffix(')'))
        });
        let Some(head) = head else {
            return Err(JsNativeError::error()
                .with_message(format!(
                    "Invalid variable name: {}. Fix: use var(--name) before the final fallback value",
                    to_text(value, context)?
                ))
                .into());
        };
        result = format!("{head}, {result})");
    }
    Ok(js_string!(result).into())
}
