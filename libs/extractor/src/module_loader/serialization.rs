use boa_engine::{Context, JsObject, JsResult, JsValue};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{array_items, is_plain_object, js_str, json_string, own_keys, string_code, to_text};

/// Unsupported values return `None`; reading user properties must still propagate exceptions.
pub(super) fn value_to_code(
    value: &JsValue,
    context: &mut Context,
    names: &FxHashMap<String, String>,
    seen: &mut Vec<JsObject>,
) -> JsResult<Option<String>> {
    crate::evaluation_sandbox::check_export(value, context)?;
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
    let mut supported = true;
    let code = if let Some(items) = array_items(value, context)? {
        for item in &items {
            if supported {
                match value_to_code(item, context, names, seen)? {
                    Some(code) => parts.push(code),
                    None => supported = false,
                }
            } else {
                check_data_exports(item, context)?;
            }
        }
        format!("[{}]", parts.join(", "))
    } else if is_plain_object(&object, context) {
        for (key, name) in own_keys(&object, context)? {
            if supported {
                let item = object.get(key, context)?;
                match value_to_code(&item, context, names, seen)? {
                    Some(code) => parts.push(format!("{}: {code}", json_string(&name))),
                    None => supported = false,
                }
            } else if let Some(item) = object
                .borrow()
                .properties()
                .get(&key)
                .and_then(|property| property.value().cloned())
            {
                check_data_exports(&item, context)?;
            }
        }
        if parts.is_empty() {
            "{}".to_string()
        } else {
            format!("{{ {} }}", parts.join(", "))
        }
    } else {
        seen.pop();
        return Ok(None);
    };
    seen.pop();
    Ok(supported.then_some(code))
}

/// After fallback is selected, inspect stored data without re-running getters or coercions.
fn check_data_exports(value: &JsValue, context: &mut Context) -> JsResult<()> {
    let mut pending = vec![value.clone()];
    let mut visited = FxHashSet::default();
    while let Some(value) = pending.pop() {
        crate::evaluation_sandbox::check_export(&value, context)?;
        let Some(object) = value.as_object() else {
            continue;
        };
        if object.is_callable()
            || !visited.insert(object.clone())
            || !(object.is_array() || is_plain_object(&object, context))
        {
            continue;
        }
        let keys = own_keys(&object, context)?;
        for (key, _) in keys.into_iter().rev() {
            if let Some(item) = object
                .borrow()
                .properties()
                .get(&key)
                .and_then(|property| property.value().cloned())
            {
                pending.push(item);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod serialization_coverage_tests;
#[cfg(test)]
mod serialization_gate_boundaries_tests;
