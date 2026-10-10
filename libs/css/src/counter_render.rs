use std::borrow::Cow;

use crate::allocation_input::{
    AllocationContext, LegacyDeclaration, LegacyInput, LegacyVariable, effective_file,
};
use crate::atom_name::hex;
use crate::legacy_variable_names::encode_selector;
use crate::num_to_nm_base::num_to_nm_base;
use crate::optimize_value::optimize_value;

pub(crate) fn declaration_body(declaration: &LegacyDeclaration, debug: bool) -> String {
    let value = declaration
        .value
        .as_deref()
        .map_or(Cow::Borrowed(""), optimize_value);
    let selector = declaration.selector.as_deref().unwrap_or_default().trim();
    let (value, selector) = if debug {
        (
            Cow::Owned(encode_selector(&value)),
            Cow::Owned(encode_selector(selector)),
        )
    } else {
        (value, Cow::Borrowed(selector))
    };
    format!(
        "{}-{}-{value}-{selector}-{}",
        declaration.property.trim(),
        declaration.level,
        declaration.order.unwrap_or(255),
    )
}

pub(crate) fn variable_body(variable: &LegacyVariable, debug: bool) -> String {
    let selector = variable.selector.as_deref().unwrap_or_default().trim();
    let selector = if debug {
        Cow::Owned(encode_selector(selector))
    } else {
        Cow::Borrowed(selector)
    };
    format!("{}-{}-{selector}", variable.property, variable.level)
}

pub(crate) fn debug_name(input: &LegacyInput, context: &AllocationContext) -> String {
    let prefix = &context.config.prefix;
    match input {
        LegacyInput::Declaration(declaration) => {
            let body = declaration_body(declaration, true);
            match effective_file(input, context) {
                Some(file) => format!("{prefix}{body}-{}", num_to_nm_base(file.ordinal())),
                None => format!("{prefix}{body}"),
            }
        }
        LegacyInput::Keyframes(keyframes) => format!("{prefix}k-{keyframes}"),
        LegacyInput::Variable(variable) => format!("--{prefix}{}", variable_body(variable, true)),
    }
}

pub(crate) fn atom_name(input: &LegacyInput, context: &AllocationContext) -> String {
    let prefix = &context.config.prefix;
    match input {
        LegacyInput::Declaration(declaration) => {
            let scope = if declaration.order == Some(0) {
                "g".to_string()
            } else {
                match &context.delivery {
                    None => "g".to_string(),
                    Some(delivery) if delivery.hoisted => "h".to_string(),
                    Some(delivery) => format!("l-{}", hex(&delivery.canonical)),
                }
            };
            let value = match declaration.value.as_deref() {
                None => "n".to_string(),
                Some(value) => format!("s-{}", hex(&optimize_value(value))),
            };
            format!(
                "{prefix}a1-{scope}-{}-{}-{value}-{}-{}",
                hex(declaration.property.trim()),
                declaration.level,
                hex(declaration.selector.as_deref().unwrap_or_default()),
                declaration.order.unwrap_or(255),
            )
        }
        LegacyInput::Keyframes(keyframes) => {
            let scope = match &context.delivery {
                None => "g".to_string(),
                Some(delivery) => format!("l-{}", hex(&delivery.canonical)),
            };
            format!("{prefix}k1-{scope}-{}", hex(keyframes))
        }
        LegacyInput::Variable(variable) => format!(
            "--{prefix}v1-{}-{}-{}",
            hex(variable.property.trim()),
            variable.level,
            hex(variable.selector.as_deref().unwrap_or_default()),
        ),
    }
}
