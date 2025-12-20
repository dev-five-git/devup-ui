use oxc_ast::ast::Expression;

use crate::{ExtractStyleProp, extract_style::extract_keyframes::ExtractKeyframes};

pub(super) mod extract_global_style_from_expression;
pub(super) mod extract_keyframes_from_expression;
pub(super) mod extract_style_from_expression;
pub(super) mod extract_style_from_jsx;
pub(super) mod extract_style_from_member_expression;
pub(super) mod extract_style_from_styled;
pub(super) mod extract_vanilla_extract;

/**
 * type
 * 1. jsx -> <Extract a={1} />
 * 2. object -> createElement('div', {a: 1})
 * 3. object with select -> createElement('div', {a: 1})
 */

#[derive(Debug, Default)]
pub struct ExtractResult<'a> {
    // attribute will be maintained
    pub styles: Vec<ExtractStyleProp<'a>>,
    pub tag: Option<Expression<'a>>,
    pub style_order: Option<u8>,
    pub style_vars: Option<Expression<'a>>,
    pub props: Option<Expression<'a>>,
}

#[derive(Debug)]
pub struct GlobalExtractResult<'a> {
    pub styles: Vec<ExtractStyleProp<'a>>,
    pub style_order: Option<u8>,
}

#[derive(Debug)]
pub struct KeyframesExtractResult {
    pub keyframes: ExtractKeyframes,
}
