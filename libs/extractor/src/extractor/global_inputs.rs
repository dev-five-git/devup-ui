mod fonts;
mod imports;

pub(super) use fonts::font_faces;
pub(super) use imports::imports;

use crate::{ExtractStyleProp, style_values::StyleValues, utils::readable_code};
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;

pub(crate) const FONT_TEMPLATE_INPUT: &str = "font-face template interpolations must be static CSS text or theme references in descriptor values";

pub(crate) struct GlobalInputs<'s, 'a> {
    pub file: &'s str,
    pub values: &'s StyleValues<'a>,
}

impl GlobalInputs<'_, '_> {
    fn absent(&self, expression: &Expression<'_>) -> bool {
        super::style_inputs::absent(expression, Some(self.values))
    }
}

fn reject<'a>(expression: &Expression<'a>, requirement: &'static str) -> ExtractStyleProp<'a> {
    super::extract_style_from_expression::misplaced(
        expression.span().start,
        readable_code(expression),
        requirement,
    )
}
