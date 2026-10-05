use std::collections::BTreeMap;

use boa_engine::{
    Context, JsArgs, JsNativeError, JsResult, JsValue, js_string,
    object::{ObjectInitializer, builtins::JsArray},
};

use super::{js_str, own_keys, to_text, token_walk::walk_object};

#[derive(PartialEq, Eq)]
enum Shape {
    Leaf,
    Object(BTreeMap<String, Shape>),
}

/// Normalization discards leaf values and skipped token kinds, not empty objects.
fn normalized_shape(tokens: &JsValue, context: &mut Context) -> JsResult<Shape> {
    let normalized = walk_object(tokens, context, &mut Vec::new(), &mut |_, _, _| {
        Ok(js_string!("").into())
    })?;
    shape(&normalized, context)
}

fn shape(value: &JsValue, context: &mut Context) -> JsResult<Shape> {
    let Some(object) = value.as_object() else {
        return Ok(Shape::Leaf);
    };
    let mut children = BTreeMap::new();
    for (key, name) in own_keys(&object, context)? {
        children.insert(name, shape(&object.get(key, context)?, context)?);
    }
    Ok(Shape::Object(children))
}

/// All three assignment APIs validate the complete normalized contract shape.
pub(super) fn assign_vars(
    contract: &JsValue,
    tokens: &JsValue,
    context: &mut Context,
) -> JsResult<Vec<(String, String)>> {
    if normalized_shape(contract, context)? != normalized_shape(tokens, context)? {
        return Err(JsNativeError::error()
            .with_message("Tokens don't match contract. Fix: supply every contract token and no extra tokens, or pass a matching subcontract")
            .into());
    }
    let mut vars = Vec::new();
    walk_object(
        tokens,
        context,
        &mut Vec::new(),
        &mut |value, path, context| {
            let mut target = contract.clone();
            for key in path {
                target = target
                    .to_object(context)?
                    .get(js_string!(key.as_str()), context)?;
            }
            let reference = to_text(&target, context)?;
            vars.push((reference, to_text(value, context)?));
            Ok(JsValue::undefined())
        },
    )?;
    Ok(vars)
}

pub(super) fn assign_vars_api(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let assigned = ObjectInitializer::new(context).build();
    for (name, value) in assign_vars(args.get_or_undefined(0), args.get_or_undefined(1), context)? {
        assigned.set(js_string!(name), js_string!(value), false, context)?;
    }
    Ok(assigned.into())
}

/// Exactly the names left unchanged by upstream `cssesc(..., {isIdentifier:true})`.
fn unescaped_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    let first = bytes.next();
    !(first.is_some_and(|byte| byte.is_ascii_digit())
        || first == Some(b'-')
            && bytes
                .next()
                .is_some_and(|byte| byte == b'-' || byte.is_ascii_digit()))
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub(super) fn create_global_theme_contract(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let map = args.get_or_undefined(1).as_callable();
    walk_object(
        args.get_or_undefined(0),
        context,
        &mut Vec::new(),
        &mut |value, path, context| {
            let raw_name = match &map {
                Some(map) => {
                    let argument_path = JsArray::from_iter(
                        path.iter().map(|key| js_string!(key.as_str()).into()),
                        context,
                    );
                    map.call(
                        &JsValue::undefined(),
                        &[value.clone(), argument_path.into()],
                        context,
                    )?
                }
                None => value.clone(),
            };
            let name = js_str(&raw_name);
            let name = name
                .as_deref()
                .map(|name| name.strip_prefix("--").unwrap_or(name));
            match name.filter(|name| unescaped_identifier(name)) {
                Some(name) => Ok(js_string!(format!("var(--{name})")).into()),
                None => Err(JsNativeError::error()
                    .with_message(format!(
                        "Invalid variable name for \"{}\". Fix: return a string name unchanged by CSS identifier escaping",
                        path.join(".")
                    ))
                    .into()),
            }
        },
    )
}
