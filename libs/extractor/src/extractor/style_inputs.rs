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
    values: Option<&'s StyleValues<'s>>,
}

impl<'s, 'a> StyleInputs<'s, 'a> {
    pub const fn with_values(ast: &'s AstBuilder<'a>, values: &'s StyleValues<'s>) -> Self {
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

pub(crate) fn absent(expression: &Expression<'_>, values: Option<&StyleValues<'_>>) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::extract_style_from_expression::{
        LiteralHandling, extract_style_from_expression,
    };
    use css::style_selector::StyleSelector;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    #[rstest::rstest]
    #[case::unproven("undefined", false)]
    #[case::null("null", true)]
    #[case::pure_void("void 0", true)]
    fn optional_params_require_proof_when_only_a_builder_is_supplied(
        #[case] params: &str,
        #[case] accepted: bool,
    ) {
        // Given an optional selector without any lexical absence metadata.
        let arena = Allocator::default();
        let ast = AstBuilder::new(&arena);
        let source = format!("({{params:{params},color:'red'}})");
        let mut expression = Parser::new(&arena, &source, SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"));
        // When the builder-only entry extracts the selector's rules.
        let result = extract_style_from_expression(
            (&ast).into(),
            None,
            &mut expression,
            0,
            &Some(StyleSelector::from("hover")),
            LiteralHandling::ExpandResponsiveThemeToken,
        );
        // Then literal absence retains the declaration, but spelling alone cannot prove it.
        if accepted {
            let [crate::ExtractStyleProp::Static(crate::ExtractStyleValue::Static(style))] =
                result.styles.as_slice()
            else {
                panic!("expected one hover declaration: {:?}", result.styles)
            };
            assert_eq!(
                (style.property.as_str(), style.value.as_str()),
                ("color", "red")
            );
            assert_eq!(
                style.selector.as_ref().map(ToString::to_string),
                Some("&:hover".to_string())
            );
        } else {
            let mut errors = Vec::new();
            crate::utils::unreadable_styles(&result.styles, false, &mut errors);
            assert_eq!(
                errors,
                vec![(
                    9,
                    "undefined".to_string(),
                    Some(crate::extractor::selector_params::SELECTOR_PARAMS)
                )]
            );
        }
    }
}
