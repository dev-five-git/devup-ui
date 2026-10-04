use super::{ExtractResult, LiteralHandling, extract_style_from_expression};
use crate::{
    composition::Composition,
    utils::{StyleArguments, style_arguments, unwrap_syntax_only},
};
use oxc_ast::{
    ast::{Argument, Expression},
    builder::AstBuilder,
};

pub(super) fn compose<'a>(
    ast: &AstBuilder<'a>,
    arguments: &[Argument<'a>],
) -> Option<(ExtractResult<'a>, Vec<Expression<'a>>)> {
    let mut composition = Composition::default();
    let mut classes = Vec::new();
    let mut result = ExtractResult::default();
    for argument in arguments {
        let mut rules = match argument.as_expression() {
            Some(rules) if written(rules) => rules.clone_in_with_semantic_ids(ast.allocator()),
            _ => {
                let StyleArguments {
                    classes: own,
                    rules,
                } = style_arguments(ast, std::slice::from_ref(argument))?;
                classes.extend(own);
                rules
            }
        };
        let part = extract_style_from_expression(
            ast,
            None,
            &mut rules,
            0,
            &None,
            LiteralHandling::ExpandResponsiveThemeToken,
        );
        composition.apply(ast, part.styles);
        result.style_order = part.style_order.or(result.style_order);
        result.style_vars = part.style_vars.or(result.style_vars);
        result.props = part.props.or(result.props);
    }
    result.styles = composition.into_props();
    Some((result, classes))
}

fn written(rules: &Expression<'_>) -> bool {
    match unwrap_syntax_only(rules) {
        Expression::ObjectExpression(_)
        | Expression::NullLiteral(_)
        | Expression::BooleanLiteral(_) => true,
        Expression::Identifier(identifier) => identifier.name == "undefined",
        Expression::ConditionalExpression(branch) => {
            written(&branch.consequent) && written(&branch.alternate)
        }
        _ => false,
    }
}

use oxc_allocator::{CloneIn, GetAllocator};

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::extract_style::{ExtractStyleProperty, extract_static_style::ExtractStaticStyle};

    #[test]
    fn undefined_part_when_next_to_written_choice_adds_no_style_or_class()
    -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let allocator = oxc_allocator::Allocator::default();
        let ast = AstBuilder::new(&allocator);
        let parsed = oxc_parser::Parser::new(
            &allocator,
            "parts(undefined, on ? { color: 'red' } : null)",
            oxc_span::SourceType::tsx(),
        )
        .parse_expression()
        .map_err(|error| format!("failed to parse valid call: {error:?}"))?;
        let Expression::CallExpression(call) = parsed else {
            panic!("expected call")
        };
        // When
        let (result, classes) =
            compose(&ast, &call.arguments).ok_or("written parts were unreadable")?;
        // Then
        assert_eq!(classes.len(), 0);
        assert!(matches!(
            result.styles.as_slice(),
            [crate::ExtractStyleProp::Conditional { .. }]
        ));
        Ok(())
    }

    #[rstest::rstest]
    #[case("true", "blue")]
    #[case("false", "red")]
    #[serial_test::serial]
    fn opaque_class_when_paired_with_written_choice_preserves_selected_atom(
        #[case] on: &str,
        #[case] expected: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let allocator = oxc_allocator::Allocator::default();
        let ast = AstBuilder::new(&allocator);
        let parsed = oxc_parser::Parser::new(
            &allocator,
            "parts(external, { color: 'red' }, on ? { color: 'blue' } : null)",
            oxc_span::SourceType::tsx(),
        )
        .parse_expression()
        .map_err(|error| format!("failed to parse valid call: {error:?}"))?;
        let Expression::CallExpression(call) = parsed else {
            panic!("expected call")
        };
        // When
        let (mut result, classes) =
            compose(&ast, &call.arguments).ok_or("style parts were unreadable")?;
        let class = crate::gen_class_name::gen_class_names(&ast, &mut result.styles, None, None)
            .ok_or("selected atom produced no class")?;
        let code = format!(
            "const on = {on}; {}",
            crate::utils::expression_to_code(&class)
        );
        let mut context = boa_engine::Context::default();
        let selected = context
            .eval(boa_engine::Source::from_bytes(&code))
            .map_err(|error| format!("choice failed to evaluate: {error}"))?
            .to_string(&mut context)
            .map_err(|error| format!("selected class failed string conversion: {error}"))?
            .to_std_string_escaped();
        // Then
        assert_eq!(classes.len(), 1);
        assert_eq!(
            crate::css_utils::rm_last_semi_colon(&crate::utils::expression_to_code(&classes[0])),
            "external"
        );
        assert_eq!(
            selected,
            ExtractStaticStyle::new("color", expected, 0, None)
                .extract(None)
                .to_string()
        );
        Ok(())
    }
}
