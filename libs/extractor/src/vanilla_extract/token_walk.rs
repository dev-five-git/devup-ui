use std::collections::BTreeSet;

use boa_engine::{
    Context, JsResult, JsValue, builtins::object::OrdinaryObject, js_string,
    object::ObjectInitializer, property::PropertyKey,
};

use super::own_keys;

type Leaf<'a> = dyn FnMut(&JsValue, &[String], &mut Context) -> JsResult<JsValue> + 'a;

/// Keys visited by JavaScript `for...in`, including inherited enumerable keys.
fn enumerable_keys(
    tokens: &JsValue,
    context: &mut Context,
) -> JsResult<Vec<(PropertyKey, String)>> {
    let mut keys = Vec::new();
    let mut seen = BTreeSet::new();
    let mut prototype = if tokens.is_null_or_undefined() {
        None
    } else {
        Some(tokens.to_object(context)?)
    };
    while let Some(object) = prototype {
        for (key, name) in own_keys(&object, context)? {
            if seen.insert(name.clone())
                && OrdinaryObject::property_is_enumerable(
                    &object.clone().into(),
                    &[js_string!(name.as_str()).into()],
                    context,
                )?
                .to_boolean()
            {
                keys.push((key, name));
            }
        }
        prototype =
            OrdinaryObject::get_prototype_of(&JsValue::undefined(), &[object.into()], context)?
                .as_object();
    }
    Ok(keys)
}

/// Replace upstream token leaves; arrays, functions and booleans are skipped.
pub(super) fn walk_object(
    tokens: &JsValue,
    context: &mut Context,
    path: &mut Vec<String>,
    leaf: &mut Leaf<'_>,
) -> JsResult<JsValue> {
    let walked = ObjectInitializer::new(context).build();
    for (key, name) in enumerable_keys(tokens, context)? {
        let value = tokens.to_object(context)?.get(key, context)?;
        path.push(name.clone());
        let mapped = if value.is_string() || value.is_number() || value.is_null_or_undefined() {
            Some(leaf(&value, path, context)?)
        } else if value
            .as_object()
            .is_some_and(|object| !object.is_array() && !object.is_callable())
        {
            Some(walk_object(&value, context, path, leaf)?)
        } else {
            None
        };
        path.pop();
        if let Some(mapped) = mapped {
            walked.set(js_string!(name), mapped, false, context)?;
        }
    }
    Ok(walked.into())
}
