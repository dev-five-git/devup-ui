use boa_engine::{Context, JsObject, JsResult, JsValue};
use rustc_hash::FxHashMap;

use super::{array_items, is_plain_object, js_str, json_string, own_keys, string_code, to_text};

/// Unsupported values return `None`; reading user properties must still propagate exceptions.
pub(super) fn value_to_code(
    value: &JsValue,
    context: &mut Context,
    names: &FxHashMap<String, String>,
    seen: &mut Vec<JsObject>,
) -> JsResult<Option<String>> {
    if let Some(text) = js_str(value) {
        return Ok(Some(string_code(&text, names)));
    }
    if value.is_null_or_undefined() || value.is_number() || value.as_boolean().is_some() {
        return to_text(value, context).map(Some);
    }
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    if object.is_callable()
        || seen
            .iter()
            .any(|visited| JsObject::equals(visited, &object))
    {
        return Ok(None);
    }
    seen.push(object.clone());
    let mut parts = Vec::new();
    let code = if let Some(items) = array_items(value, context)? {
        for item in &items {
            let Some(code) = value_to_code(item, context, names, seen)? else {
                return Ok(None);
            };
            parts.push(code);
        }
        format!("[{}]", parts.join(", "))
    } else if is_plain_object(&object, context) {
        for (key, name) in own_keys(&object, context)? {
            let item = object.get(key, context)?;
            let Some(code) = value_to_code(&item, context, names, seen)? else {
                return Ok(None);
            };
            parts.push(format!("{}: {code}", json_string(&name)));
        }
        if parts.is_empty() {
            "{}".to_string()
        } else {
            format!("{{ {} }}", parts.join(", "))
        }
    } else {
        return Ok(None);
    };
    seen.pop();
    Ok(Some(code))
}

#[cfg(test)]
mod serialization_coverage_tests;
