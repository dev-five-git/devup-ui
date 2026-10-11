use oxc_ast::ast::Expression;

use crate::{ExtractStyleProp, extract_style::extract_keyframes::ExtractKeyframes};

pub(super) mod extract_global_style_from_expression;
pub(super) mod extract_keyframes_from_expression;
pub(super) mod extract_style_from_expression;
pub(super) mod extract_style_from_jsx;
pub(super) mod extract_style_from_member_expression;
pub(super) mod extract_style_from_styled;
pub(super) mod extract_style_from_stylex;
pub(crate) mod rule_payload;

/**
 * type
 * 1. jsx -> <Extract a={1} />
 * 2. object -> createElement('div', {a: 1})
 * 3. object with select -> createElement('div', {a: 1})
 */

#[derive(Debug)]
pub struct ExtractResult<'a, E = Expression<'a>> {
    // attribute will be maintained
    pub styles: Vec<ExtractStyleProp<'a, E>>,
    pub tag: Option<Expression<'a>>,
    pub style_order: Option<u8>,
    pub style_vars: Option<Expression<'a>>,
    pub props: Option<Expression<'a>>,
}

impl<E> Default for ExtractResult<'_, E> {
    fn default() -> Self {
        Self {
            styles: Vec::new(),
            tag: None,
            style_order: None,
            style_vars: None,
            props: None,
        }
    }
}

impl<'a, E> ExtractResult<'a, E> {
    pub(crate) fn map_payload<Q>(self, map: impl Fn(E) -> Q) -> ExtractResult<'a, Q> {
        ExtractResult {
            styles: self
                .styles
                .into_iter()
                .map(|prop| prop.map_payload(&map))
                .collect(),
            tag: self.tag,
            style_order: self.style_order,
            style_vars: self.style_vars,
            props: self.props,
        }
    }
}

#[derive(Debug)]
pub struct GlobalExtractResult<'a> {
    pub styles: Vec<ExtractStyleProp<'a>>,
    pub style_order: Option<u8>,
}

#[derive(Debug)]
pub struct KeyframesExtractResult {
    pub keyframes: ExtractKeyframes,
    /// A value only known at runtime, which keyframes cannot hold
    pub runtime_value: Option<String>,
}
