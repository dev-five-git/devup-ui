use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::Expression;

use crate::{ErrorDisposition, ExtractStyleProp, ExtractStyleValue};

/// The top-level roots constructed by composition; descendants retain the full prop grammar.
pub(crate) enum NormalisedProp<'a, E> {
    Static(ExtractStyleValue),
    Conditional {
        condition: Expression<'a>,
        consequent: Option<Box<ExtractStyleProp<'a, E>>>,
        alternate: Option<Box<ExtractStyleProp<'a, E>>>,
    },
    Supplied {
        styles: Vec<ExtractStyleValue>,
        expression: E,
    },
    Diagnostic {
        offset: u32,
        message: String,
        disposition: ErrorDisposition,
    },
    Unreadable {
        offset: u32,
        code: String,
        prop: bool,
    },
}

impl<'a, E> NormalisedProp<'a, E> {
    pub(crate) fn into_prop(self) -> ExtractStyleProp<'a, E> {
        match self {
            Self::Static(value) => ExtractStyleProp::Static(value),
            Self::Conditional {
                condition,
                consequent,
                alternate,
            } => ExtractStyleProp::Conditional {
                condition,
                consequent,
                alternate,
            },
            Self::Supplied { styles, expression } => {
                ExtractStyleProp::Expression { styles, expression }
            }
            Self::Diagnostic {
                offset,
                message,
                disposition,
            } => ExtractStyleProp::Diagnostic {
                offset,
                message,
                disposition,
            },
            Self::Unreadable { offset, code, prop } => {
                ExtractStyleProp::Unreadable { offset, code, prop }
            }
        }
    }

    /// Clone an observation without erasing the retained normalized payload.
    pub(crate) fn clone_payload_in<Q>(
        &self,
        alloc: &'a Allocator,
        clone_payload: impl Fn(&E, &'a Allocator) -> Q,
    ) -> NormalisedProp<'a, Q> {
        match self {
            Self::Static(value) => NormalisedProp::Static(value.clone()),
            Self::Conditional {
                condition,
                consequent,
                alternate,
            } => NormalisedProp::Conditional {
                condition: condition.clone_in(alloc),
                consequent: consequent
                    .as_ref()
                    .map(|prop| Box::new(prop.clone_payload_in(alloc, &clone_payload))),
                alternate: alternate
                    .as_ref()
                    .map(|prop| Box::new(prop.clone_payload_in(alloc, &clone_payload))),
            },
            Self::Supplied { styles, expression } => NormalisedProp::Supplied {
                styles: styles.clone(),
                expression: clone_payload(expression, alloc),
            },
            Self::Diagnostic {
                offset,
                message,
                disposition,
            } => NormalisedProp::Diagnostic {
                offset: *offset,
                message: message.clone(),
                disposition: *disposition,
            },
            Self::Unreadable { offset, code, prop } => NormalisedProp::Unreadable {
                offset: *offset,
                code: code.clone(),
                prop: *prop,
            },
        }
    }
}
