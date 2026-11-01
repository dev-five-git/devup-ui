use super::ExtractResult;
use crate::extractor::style_order_projection::AuthoredStyles;
#[path = "styled_rule_inputs.rs"]
mod inputs;
#[path = "styled_rule_part.rs"]
mod part;
use crate::{composition::Composition, utils::unwrap_syntax_only};
use oxc_ast::{
    ast::{Argument, Expression},
    builder::AstBuilder,
};

pub(super) fn contains_known_rules(
    rules: &Expression<'_>,
    bindings: &super::StyledBindings<'_>,
) -> bool {
    if bindings.styles(rules).is_some() {
        return true;
    }
    match unwrap_syntax_only(rules) {
        Expression::ConditionalExpression(branch) => {
            contains_known_rules(&branch.consequent, bindings)
                || contains_known_rules(&branch.alternate, bindings)
        }
        _ => false,
    }
}

#[cfg(test)]
pub(super) fn compose<'a>(
    ast: &AstBuilder<'a>,
    arguments: &[Argument<'a>],
) -> Option<(ExtractResult<'a>, Vec<Expression<'a>>)> {
    compose_bound(ast, arguments, (None, "parts.tsx"))
}

pub(super) fn compose_bound<'a>(
    ast: &AstBuilder<'a>,
    arguments: &[Argument<'a>],
    (bindings, filename): (Option<&super::StyledBindings<'_>>, &str),
) -> Option<(ExtractResult<'a>, Vec<Expression<'a>>)> {
    let mut composition = Composition::default();
    let mut classes = Vec::new();
    let mut result = ExtractResult::default();
    inputs::visit(ast, (arguments, bindings), |input| {
        let mut rules = match input {
            inputs::Input::Rules(rules) => rules.clone_in_with_semantic_ids(ast.allocator()),
            inputs::Input::Class(class) => {
                classes.push(class.clone_in_with_semantic_ids(ast.allocator()));
                return;
            }
            inputs::Input::Prepared(crate::utils::StyleArguments {
                classes: own,
                rules,
            }) => {
                classes.extend(own);
                rules
            }
        };
        let part = part::extract(ast, &mut rules, (bindings, filename));
        AuthoredStyles(part.styles).apply(ast, &mut composition);
        result.style_order = part.style_order.or(result.style_order);
        result.style_vars = part.style_vars.or_else(|| result.style_vars.take());
        result.props = part.props.or_else(|| result.props.take());
    })?;
    result.styles = composition.into_props();
    Some((result, classes))
}

pub(super) fn admitted<'a>(
    ast: &AstBuilder<'a>,
    arguments: &[Argument<'a>],
    bindings: Option<&super::StyledBindings<'_>>,
) -> bool {
    inputs::visit(ast, (arguments, bindings), |_| {}).is_some()
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
