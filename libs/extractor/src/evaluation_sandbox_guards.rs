//! What the sandbox forbids reading, and the source that guards the globals.

use super::{Evidence, function, record};

pub(super) const EXACT_GLOBALS: &[&str] = &[
    "undefined",
    "NaN",
    "Infinity",
    "globalThis",
    "Math",
    "Object",
    "Array",
    "String",
    "Number",
    "Boolean",
    "BigInt",
    "Symbol",
    "JSON",
    "Reflect",
    "Proxy",
    "Promise",
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    "Error",
    "TypeError",
    "ReferenceError",
    "SyntaxError",
    "RangeError",
    "URIError",
    "EvalError",
    "Uint8Array",
    "Uint8ClampedArray",
    "Uint16Array",
    "Uint32Array",
    "Int8Array",
    "Int16Array",
    "Int32Array",
    "Float32Array",
    "Float64Array",
    "BigInt64Array",
    "BigUint64Array",
    "ArrayBuffer",
    "DataView",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "encodeURI",
    "decodeURI",
    "encodeURIComponent",
    "decodeURIComponent",
    "eval",
    "console",
];
use boa_engine::{
    Context, JsArgs, JsError, JsObject, JsResult, JsString, JsValue, NativeFunction, js_string,
    object::FunctionObjectBuilder, property::PropertyDescriptor,
};

fn guard_method(context: &mut Context, holder: &JsObject, label: (&str, &str)) -> JsResult<()> {
    let (owner, method) = label;
    let guard = function(
        context,
        method,
        NativeFunction::from_copy_closure_with_captures(
            |_, _, name: &String, context| {
                let site = context
                    .get_data::<Evidence>()
                    .and_then(|evidence| evidence.site.borrow().clone());
                Err(JsError::from_opaque(record(context, name, site)))
            },
            format!("{owner}.{method}"),
        ),
    );
    let setter = function(
        context,
        "set",
        NativeFunction::from_copy_closure_with_captures(
            |this, args, method: &String, context| {
                this.to_object(context)?.define_property_or_throw(
                    JsString::from(method.as_str()),
                    PropertyDescriptor::builder()
                        .value(args.get_or_undefined(0).clone())
                        .writable(true)
                        .enumerable(false)
                        .configurable(true),
                    context,
                )?;
                Ok(JsValue::undefined())
            },
            method.to_string(),
        ),
    );
    holder.define_property_or_throw(
        JsString::from(method),
        PropertyDescriptor::builder()
            .get(guard.clone())
            .set(setter)
            .enumerable(false)
            .configurable(true),
        context,
    )?;
    if let Some(evidence) = context.get_data::<Evidence>() {
        evidence
            .methods
            .borrow_mut()
            .push((guard, format!("{owner}.{method}")));
    }
    Ok(())
}

pub(super) fn guard_methods(context: &mut Context) -> JsResult<()> {
    let math = context.intrinsics().objects().math();
    guard_method(context, &math, ("Math", "random"))?;
    for (owner, methods) in GUARDED_METHODS {
        let constructors = context.intrinsics().constructors();
        let holder = match owner {
            "Object" => constructors.object(),
            "Number" => constructors.number(),
            "BigInt" => constructors.bigint(),
            "Array" => constructors.array(),
            "TypedArray" => constructors.typed_array(),
            "Function" => constructors.function(),
            _ => constructors.string(),
        }
        .prototype();
        for method in methods {
            guard_method(context, &holder, (&format!("{owner}.prototype"), method))?;
        }
    }
    guard_descriptor(context)
}

fn guard_descriptor(context: &mut Context) -> JsResult<()> {
    let object = context.intrinsics().constructors().object().constructor();
    let reflect = context.intrinsics().objects().reflect();
    for (holder, name, batch) in [
        (&object, "getOwnPropertyDescriptor", false),
        (&object, "getOwnPropertyDescriptors", true),
        (&reflect, "getOwnPropertyDescriptor", false),
    ] {
        let original = holder
            .get(JsString::from(name), context)?
            .to_object(context)?;
        let realm = context.realm().clone();
        let wrapper = FunctionObjectBuilder::new(
            &realm,
            NativeFunction::from_copy_closure_with_captures(
                |this, args, captures: &(JsObject, bool), context| {
                    let site = context
                        .get_data::<Evidence>()
                        .and_then(|evidence| evidence.site.borrow().clone());
                    let value = captures.0.call(this, args, context)?;
                    if captures.1 {
                        let descriptors = value.to_object(context)?;
                        for key in descriptors.own_property_keys(context)? {
                            check_descriptor(
                                &descriptors.get(key, context)?,
                                site.clone(),
                                context,
                            )?;
                        }
                    } else {
                        check_descriptor(&value, site, context)?;
                    }
                    Ok(value)
                },
                (original, batch),
            ),
        )
        .name(JsString::from(name))
        .length(if batch { 1 } else { 2 })
        .build();
        holder.define_property_or_throw(
            JsString::from(name),
            PropertyDescriptor::builder()
                .value(wrapper)
                .writable(true)
                .enumerable(false)
                .configurable(true),
            context,
        )?;
    }
    Ok(())
}

fn check_descriptor(
    value: &JsValue,
    site: Option<(String, u32)>,
    context: &mut Context,
) -> JsResult<()> {
    let getter = value.as_object().and_then(|descriptor| {
        descriptor
            .borrow()
            .properties()
            .get(&js_string!("get").into())
            .and_then(|property| property.value().cloned())
    });
    let name = context.get_data::<Evidence>().and_then(|evidence| {
        evidence.methods.borrow().iter().find_map(|(method, name)| {
            getter
                .as_ref()?
                .as_object()
                .filter(|getter| getter == method)
                .map(|_| name.clone())
        })
    });
    match name {
        Some(name) => Err(JsError::from_opaque(record(context, &name, site))),
        None => Ok(()),
    }
}

/// Globals the engine gives whose values depend on the locale or the clock;
/// any other global nothing declares is the environment's
pub(super) const GUARDED_GLOBALS: [&str; 5] = ["Date", "Intl", "performance", "crypto", "Temporal"];

/// Methods whose results depend on locale, Unicode data, or transformed source.
pub(super) const GUARDED_METHODS: [(&str, &[&str]); 7] = [
    ("Object", &["toLocaleString"]),
    ("Number", &["toLocaleString"]),
    ("BigInt", &["toLocaleString"]),
    ("Array", &["toLocaleString"]),
    ("TypedArray", &["toLocaleString"]),
    ("Function", &["toString"]),
    (
        "String",
        &[
            "localeCompare",
            "toLocaleUpperCase",
            "toLocaleLowerCase",
            "normalize",
        ],
    ),
];

/// Guards the globals with `forbid`, which records a read and gives the error
/// that fails it. Assigning a guarded global gives the code a value of its
/// own. Globals nothing declares are guarded alike, through the prototype of
/// the global object.
/// Keep function bodies and descriptors out of nested call arguments: Boa's
/// debug parser otherwise exhausts the default Windows main-thread stack.
pub(super) const SETUP: &str = r#"(function (forbid, config) {
  const global = globalThis;
  function assign(object, name, value) {
    const descriptor = { value, writable: true, enumerable: true, configurable: true };
    Object.defineProperty(object, name, descriptor);
  }
  function read(object, name, label) {
    const get = function () { throw forbid(label); };
    const set = function (value) { assign(object, name, value); };
    const descriptor = { get, set, enumerable: false, configurable: true };
    Object.defineProperty(object, name, descriptor);
  }
  for (const name of config.names) read(global, name, name);
  function get(target, key, receiver) {
    if (typeof key === "string" && !(key in target)) throw forbid(key);
    return Reflect.get(target, key, receiver);
  }
  function has(target, key) {
    return (typeof key === "string" && !(key in target)) || Reflect.has(target, key);
  }
  const handler = { get, has };
  const prototype = new Proxy(Object.getPrototypeOf(global), handler);
  Object.setPrototypeOf(global, prototype);
})"#;
