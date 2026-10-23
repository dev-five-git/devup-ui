use crate::{
    style_values::StyleValues,
    utils::{get_string_by_literal_expression, unwrap_syntax_only},
};
use oxc_ast::{
    ast::{Expression, UnaryOperator},
    builder::AstBuilder,
};

#[derive(Clone, Copy)]
pub(crate) struct StyleInputs<'s, 'a> {
    pub ast: &'s AstBuilder<'a>,
    values: Option<&'s StyleValues<'a>>,
}

impl<'s, 'a> StyleInputs<'s, 'a> {
    pub const fn with_values(ast: &'s AstBuilder<'a>, values: &'s StyleValues<'a>) -> Self {
        Self {
            ast,
            values: Some(values),
        }
    }

    pub fn absent(self, expression: &Expression<'_>) -> bool {
        absent(expression, self.values)
    }
}

impl<'s, 'a> From<&'s AstBuilder<'a>> for StyleInputs<'s, 'a> {
    fn from(ast: &'s AstBuilder<'a>) -> Self {
        Self { ast, values: None }
    }
}

pub(super) fn absent(expression: &Expression<'_>, values: Option<&StyleValues<'_>>) -> bool {
    let mut expression = unwrap_syntax_only(expression);
    while let Expression::UnaryExpression(unary) = expression
        && unary.operator == UnaryOperator::Void
    {
        expression = unwrap_syntax_only(&unary.argument);
        if get_string_by_literal_expression(expression).is_some() {
            return true;
        }
    }
    match expression {
        Expression::NullLiteral(_) => true,
        Expression::Identifier(_) => {
            values.is_some_and(|values| values.is_unbound_undefined(expression))
        }
        _ => false,
    }
}
