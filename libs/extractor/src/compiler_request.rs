use super::extract_style_value::ExtractStyleValue as Value;
use super::{CounterProducerError, ExtractDynamicStyle, ProducerPolicy};
use super::{compiler_diagnostics::ProjectionError, extract_static_style::ExtractStaticStyle};
use css::allocation_input::{
    AllocationContext, CapturedNameConfig, LegacyDeclaration, LegacyInput,
};

pub(crate) fn configuration() -> CapturedNameConfig {
    css::allocation_input::capture_context(
        &LegacyInput::Keyframes(String::new()),
        None,
        css::CounterOwner::Inactive,
    )
    .config
}

fn static_key(style: &ExtractStaticStyle) -> impl PartialEq + '_ {
    (
        style.property(),
        style.value(),
        style.level(),
        style.selector(),
        style.style_order(),
        style.layer(),
        style.theme_token_resolution,
        style.naming,
        style.producer_policy(),
    )
}
fn dynamic_key(style: &ExtractDynamicStyle) -> impl PartialEq + '_ {
    (
        style.property(),
        style.level(),
        style.selector(),
        style.style_order(),
        style.layer(),
        style.important(),
        style.naming,
        style.identifier(),
        style.site(),
        style.producer_policy(),
    )
}
pub(crate) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Static(a), Value::Static(b)) => static_key(a) == static_key(b),
        (Value::Dynamic(a), Value::Dynamic(b)) => dynamic_key(a) == dynamic_key(b),
        (Value::Keyframes(a), Value::Keyframes(b)) => {
            a.producer_policy() == b.producer_policy()
                && a.keyframes
                    .iter()
                    .map(|(key, values)| (key, values.len()))
                    .eq(b.keyframes.iter().map(|(key, values)| (key, values.len())))
                && a.keyframes.values().flatten().map(static_key).eq(b
                    .keyframes
                    .values()
                    .flatten()
                    .map(static_key))
        }
        (
            Value::Typography(_)
            | Value::Css(_)
            | Value::Import(_)
            | Value::FontFace(_)
            | Value::Static(_)
            | Value::Dynamic(_)
            | Value::Keyframes(_),
            _,
        ) => false,
    }
}
fn declaration(property: &str, level: u8, order: Option<u8>) -> LegacyInput {
    LegacyInput::Declaration(LegacyDeclaration {
        property: property.to_string(),
        level,
        order,
        value: None,
        selector: None,
    })
}
pub(crate) fn context(
    value: &Value,
    filename: Option<&str>,
) -> Result<AllocationContext, ProjectionError> {
    let (policy, input) = match value {
        Value::Static(style) => (
            style.producer_policy(),
            declaration(style.property(), style.level(), style.style_order()),
        ),
        Value::Dynamic(style) => (
            style.producer_policy(),
            declaration(style.property(), style.level(), style.style_order()),
        ),
        Value::Keyframes(frames) => (
            frames.producer_policy(),
            LegacyInput::Keyframes(String::new()),
        ),
        Value::Typography(_) | Value::Css(_) | Value::Import(_) | Value::FontFace(_) => {
            return Err(ProjectionError::Producer(CounterProducerError::WrongPolicy));
        }
    };
    match policy {
        ProducerPolicy::Current => {
            Err(ProjectionError::Producer(CounterProducerError::WrongPolicy))
        }
        ProducerPolicy::CounterOriginal(original) => Ok(css::allocation_input::capture_context(
            &input,
            filename,
            css::CounterOwner::D9(original),
        )),
    }
}
