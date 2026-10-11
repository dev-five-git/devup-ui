use std::collections::hash_map::Entry;

use super::{Binding, Context, FinalizeError, FxHashMap, JsValue, MutationCheck};
use boa_engine::{JsString, property::PropertyKey};

pub(in crate::vanilla_extract) fn check(
    checks: &[MutationCheck],
    values: &[(&Binding, JsValue)],
    context: &mut Context,
) -> Result<(), FinalizeError> {
    let mut known: FxHashMap<&str, JsValue> = values
        .iter()
        .map(|(binding, value)| (binding.read.as_str(), value.clone()))
        .collect();
    for check in checks {
        let value = match known.entry(check.read.as_str()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(super::observations::read(context, &check.read)?),
        };
        if mutable(value, &check.path) {
            return Err(FinalizeError::Located(format!(
                "{}: required native input `{}` may be changed outside its selected initialization slice. Fix: keep the exact mutation inside its initializer or use immutable input data",
                check.place, check.read,
            )));
        }
    }
    Ok(())
}

fn mutable(value: &JsValue, path: &[Option<String>]) -> bool {
    let mut value = value.clone();
    for key in path {
        let Some(object) = value.as_object() else {
            return false;
        };
        let Some(key) = key else { return true };
        if object.is::<boa_engine::builtins::proxy::Proxy>() {
            return true;
        }
        let key = PropertyKey::from(JsString::from(key.as_str()));
        let next = object
            .borrow()
            .properties()
            .get(&key)
            .and_then(|property| property.value().cloned());
        let Some(next) = next else { return true };
        value = next;
    }
    value.is_object()
}
