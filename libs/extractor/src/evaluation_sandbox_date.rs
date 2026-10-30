//! Stylesheet-only Date capability; the native constructor never enters authored reach.

use boa_engine::{
    Context, JsError, JsObject, JsResult, JsString, JsValue, NativeFunction, js_string,
    object::FunctionObjectBuilder,
    property::{Attribute, PropertyDescriptor, PropertyKey},
};

use super::{Evidence, guards::guard_method, record};
use std::cell::RefCell;

#[derive(Default)]
struct Created(RefCell<Vec<Creation>>);

struct Creation {
    date: JsObject,
    site: Option<(String, u32)>,
}

const READS: &[&str] = &[
    "getTime",
    "valueOf",
    "getUTCDate",
    "getUTCDay",
    "getUTCFullYear",
    "getUTCHours",
    "getUTCMilliseconds",
    "getUTCMinutes",
    "getUTCMonth",
    "getUTCSeconds",
    "toISOString",
    "toUTCString",
    "toGMTString",
];

pub(super) fn install(context: &mut Context) -> JsResult<()> {
    context.insert_data(Created::default());
    let intrinsic = context.intrinsics().constructors().date();
    let original = intrinsic.constructor();
    let prototype = intrinsic.prototype();
    let utc = original.get(js_string!("UTC"), context)?;
    let realm = context.realm().clone();
    let constructor = FunctionObjectBuilder::new(
        &realm,
        NativeFunction::from_copy_closure_with_captures(
            |new_target, args, original, context| {
                let site = context
                    .get_data::<Evidence>()
                    .and_then(|evidence| evidence.site.borrow().clone());
                // No coercion may send a string to the engine parser or reach local-time construction.
                if let Some(target) = new_target.as_object()
                    && let [value] = args
                    && value.is_number()
                {
                    let date = original.construct(args, Some(&target), context)?;
                    if let Some(created) = context.get_data::<Created>() {
                        created.0.borrow_mut().push(Creation {
                            date: date.clone(),
                            site,
                        });
                    }
                    return Ok(date.into());
                }
                Err(JsError::from_opaque(record(context, "Date", site)))
            },
            original,
        ),
    )
    .name(js_string!("Date"))
    .length(7)
    .constructor(true)
    .build();
    constructor.define_property_or_throw(
        js_string!("prototype"),
        PropertyDescriptor::builder()
            .value(prototype.clone())
            .writable(false)
            .enumerable(false)
            .configurable(false),
        context,
    )?;
    constructor.define_property_or_throw(
        js_string!("UTC"),
        PropertyDescriptor::builder()
            .value(utc)
            .writable(true)
            .enumerable(false)
            .configurable(true),
        context,
    )?;
    prototype.define_property_or_throw(
        js_string!("constructor"),
        PropertyDescriptor::builder()
            .value(constructor.clone())
            .writable(true)
            .enumerable(false)
            .configurable(true),
        context,
    )?;
    for key in prototype.own_property_keys(context)? {
        if let PropertyKey::String(key) = key {
            let name = key.to_std_string_escaped();
            if name != "constructor" && !READS.contains(&name.as_str()) {
                guard_method(context, &prototype, ("Date.prototype", &name))?;
            }
        }
    }
    for name in ["now", "parse"] {
        guard_method(context, &constructor, ("Date", name))?;
    }
    context.register_global_property(
        JsString::from("Date"),
        constructor,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    )
}

/// Date objects are evaluation capabilities, not plain exports to re-emit at runtime.
pub(crate) fn check_export(value: &JsValue, context: &mut Context) -> JsResult<()> {
    let site = value.as_object().and_then(|object| {
        context.get_data::<Created>().and_then(|created| {
            created.0.borrow().iter().find_map(|creation| {
                JsObject::equals(&creation.date, &object).then(|| creation.site.clone())
            })
        })
    });
    match site {
        Some(site) => Err(JsError::from_opaque(record(context, "Date", site))),
        None => Ok(()),
    }
}
