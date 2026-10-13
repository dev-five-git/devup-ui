use boa_engine::{
    Context, JsError, JsObject, JsResult, JsString, NativeFunction,
    property::{Attribute, PropertyDescriptor},
};

use super::{Evidence, function, record};

pub(crate) fn defer_rendering(
    context: &mut Context,
    evaluation: &crate::stylesheet_compile::Evaluation,
) -> JsResult<()> {
    let namespace = JsObject::with_null_proto();
    for key in &evaluation.keys {
        let getter = function(
            context,
            "deferred rendering",
            NativeFunction::from_copy_closure_with_captures(
                |_, _, name, context| {
                    let site = context
                        .get_data::<Evidence>()
                        .and_then(|evidence| evidence.site.borrow().clone());
                    Err(JsError::from_opaque(record(context, name, site)))
                },
                format!("rendering scaffold {key}"),
            ),
        );
        namespace.define_property_or_throw(
            JsString::from(key.as_str()),
            PropertyDescriptor::builder()
                .get(getter.clone())
                .set(getter),
            context,
        )?;
    }
    context.register_global_property(
        JsString::from(evaluation.namespace.as_str()),
        namespace,
        Attribute::empty(),
    )
}
