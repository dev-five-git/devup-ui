//! Native observations of bundler-owned CSS objects; no export value is fabricated.

use boa_engine::{
    Context, JsArgs, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction,
};

use super::{Evidence, Kind, Recorded};

pub(super) struct Observation {
    pub(super) file: String,
    pub(super) error: JsError,
    pub(super) message: String,
}

#[derive(Clone, Copy)]
enum Operation {
    Get,
    Has,
    Keys,
    Descriptor,
    Prototype,
}

impl Operation {
    fn key(self, args: &[JsValue], context: &mut Context) -> JsResult<String> {
        let prefix = match self {
            Self::Get => "",
            Self::Has => "has:",
            Self::Descriptor => "getOwnPropertyDescriptor:",
            Self::Keys => return Ok("ownKeys".to_string()),
            Self::Prototype => return Ok("getPrototypeOf".to_string()),
        };
        let value = args.get_or_undefined(1);
        let key = match value.as_symbol() {
            Some(symbol) => symbol.descriptive_string().to_std_string_escaped(),
            None => value.to_string(context)?.to_std_string_escaped(),
        };
        Ok(format!("{prefix}{key}"))
    }
}

struct Request<'a> {
    file: &'a str,
    args: &'a [JsValue],
    operation: Operation,
}

pub(super) fn message(file: &str, key: &str) -> String {
    format!(
        "Cannot read CSS export '{key}' of '{file}' at build time: {}",
        Kind::Css.requirement()
    )
}

fn observe(request: Request<'_>, context: &mut Context) -> JsResult<JsValue> {
    let Some(evidence) = context.get_data::<Evidence>().cloned() else {
        return Err(JsNativeError::error()
            .with_message("CSS observation requires a sandbox")
            .into());
    };
    let file = request.file;
    let name = request.operation.key(request.args, context)?;
    let site = evidence.site.borrow().clone();
    let frame = site.as_ref().map_or_else(String::new, |(place, _)| {
        format!("\n    at <read> ({place})")
    });
    let frames = context
        .stack_trace()
        .enumerate()
        .filter_map(|(index, frame)| {
            let location = frame.position();
            if index == 0
                && let Some((place, _)) = &site
            {
                return Some(format!(
                    "\n    at {} ({place})",
                    location.function_name.to_std_string_escaped()
                ));
            }
            let position = location.position?;
            Some(format!(
                "\n    at {} ({}:{}:{})",
                location.function_name.to_std_string_escaped(),
                location.path,
                position.line_number(),
                position.column_number()
            ))
        })
        .collect::<String>();
    let immutable: JsError = JsNativeError::reference()
        .with_message(format!("{}{frame}{frames}", message(file, &name)))
        .into();
    let message = format!("ReferenceError: {}{frame}{frames}", message(file, &name));
    let error = immutable.clone().into_opaque(context)?;
    let mut reads = evidence.reads.borrow_mut();
    if !reads
        .iter()
        .any(|read| read.css.as_ref().is_some_and(|css| css.file == file) && read.site == site)
    {
        reads.push(Recorded {
            name,
            css: Some(Observation {
                file: file.to_string(),
                error: immutable,
                message,
            }),
            error: error.clone(),
            site,
        });
    }
    Err(JsError::from_opaque(error))
}

/// Creates an opaque object whose actual runtime traps record unknown observations.
pub(crate) fn css_object(context: &mut Context, file: &str) -> JsResult<JsObject> {
    let target = JsObject::with_null_proto();
    let handler = JsObject::with_null_proto();
    for (name, operation) in [
        ("get", Operation::Get),
        ("has", Operation::Has),
        ("ownKeys", Operation::Keys),
        ("getOwnPropertyDescriptor", Operation::Descriptor),
        ("getPrototypeOf", Operation::Prototype),
    ] {
        let native = NativeFunction::from_copy_closure_with_captures(
            move |_, args, file, context| {
                observe(
                    Request {
                        file,
                        args,
                        operation,
                    },
                    context,
                )
            },
            file.to_string(),
        );
        handler.create_data_property_or_throw(
            JsString::from(name),
            super::function(context, name, native),
            context,
        )?;
    }
    context
        .intrinsics()
        .constructors()
        .proxy()
        .constructor()
        .construct(&[target.into(), handler.into()], None, context)
}

#[cfg(test)]
#[path = "evaluation_sandbox_css_tests.rs"]
mod tests;
