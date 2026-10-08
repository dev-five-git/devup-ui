use super::component_layers::LAYER_NAME_REQUIREMENT;
use crate::utils::{
    build_time_error, get_string_by_property_key, readable_code, unwrap_syntax_only,
};
use oxc_ast::ast::{ArrowFunctionBody, Expression, ObjectPropertyKind};
use oxc_span::GetSpan;
use oxc_syntax::operator::LogicalOperator;

pub(crate) fn object_layer_errors(expression: &Expression<'_>, api: &str) -> Vec<(u32, String)> {
    match unwrap_syntax_only(expression) {
        Expression::CallExpression(call) => call
            .arguments
            .iter()
            .filter_map(|argument| argument.as_expression())
            .flat_map(|part| rule_layer_errors(part, api))
            .collect(),
        expression => rule_layer_errors(expression, api),
    }
}

/// Validate rule values without treating application-call arguments as CSS.
fn rule_layer_errors(expression: &Expression<'_>, api: &str) -> Vec<(u32, String)> {
    match unwrap_syntax_only(expression) {
        Expression::ObjectExpression(object) => {
            let mut errors = Vec::new();
            for property in &object.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    continue;
                };
                if property
                    .key
                    .static_name()
                    .is_some_and(|name| name == "@layer")
                {
                    if let Expression::ObjectExpression(layers) =
                        unwrap_syntax_only(&property.value)
                    {
                        for entry in &layers.properties {
                            let ObjectPropertyKind::ObjectProperty(entry) = entry else {
                                continue;
                            };
                            let name = get_string_by_property_key(&entry.key);
                            if name.as_deref().and_then(super::parse_layer_name).is_none() {
                                let code = name.unwrap_or_else(|| {
                                    entry
                                        .key
                                        .as_expression()
                                        .map_or_else(String::new, readable_code)
                                });
                                errors.push((
                                    entry.key.span().start,
                                    build_time_error(
                                        api,
                                        &format!("@layer {code}"),
                                        LAYER_NAME_REQUIREMENT,
                                    ),
                                ));
                            }
                            errors.extend(match unwrap_syntax_only(&entry.value) {
                                Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => {
                                    super::expression_layer_errors(&entry.value, api)
                                }
                                value => rule_layer_errors(value, api),
                            });
                        }
                    }
                } else {
                    errors.extend(rule_layer_errors(&property.value, api));
                }
            }
            errors
        }
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .filter_map(|element| element.as_expression())
            .flat_map(|part| rule_layer_errors(part, api))
            .collect(),
        Expression::ConditionalExpression(choice) => [&choice.consequent, &choice.alternate]
            .into_iter()
            .flat_map(|part| rule_layer_errors(part, api))
            .collect(),
        Expression::LogicalExpression(logical) => {
            let mut errors = match logical.operator {
                LogicalOperator::And => vec![],
                LogicalOperator::Or | LogicalOperator::Coalesce => {
                    rule_layer_errors(&logical.left, api)
                }
            };
            errors.extend(rule_layer_errors(&logical.right, api));
            errors
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            let returned = match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => crate::css_prop::returned(body),
                body => body.as_expression(),
            };
            returned.map_or_else(Vec::new, |rules| rule_layer_errors(rules, api))
        }
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => {
            function
                .body
                .as_deref()
                .and_then(crate::css_prop::returned)
                .map_or_else(Vec::new, |rules| rule_layer_errors(rules, api))
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    #[rstest]
    #[case("styled.div(p => ({'@layer': {inherit: {color: p.color}}}))", 1)]
    #[case(
        "styled.div(function(p) { return {'@layer': {inherit: {color: p.color}}}; })",
        1
    )]
    #[case(
        "css({'@layer': {base: {width: getWidth({'@layer': {inherit: 1}, padding: 3})}}})",
        0
    )]
    #[case("css({width: new Width({'@layer': {inherit: 1}, padding: 3})})", 0)]
    #[case("styled.div(p => ({'@layer': {inherit: {color: p.color}}}) || {})", 1)]
    #[case("styled.div(p => ({} ?? {'@layer': {inherit: {color: p.color}}}))", 1)]
    #[case("styled.div(p => ({'@layer': {inherit: {color: p.color}}}) ?? {})", 1)]
    #[case("styled.div(p => ({} || {'@layer': {inherit: {color: p.color}}}))", 1)]
    #[case("styled.div(p => ({'@layer': {inherit: 1}}) && {})", 0)]
    #[case(
        "styled.div(p => p.active && {'@layer': {inherit: {color: p.color}}})",
        1
    )]
    #[case(
        "styled.div(p => [ , p.active ? {'@layer': {inherit: {color: p.color}}} : {}])",
        1
    )]
    #[case("styled.div(async p => ({'@layer': {inherit: {color: p.color}}}))", 0)]
    #[case(
        "styled.div(function* (p) { return {'@layer': {inherit: {color: p.color}}}; })",
        0
    )]
    #[case(
        "styled.div(p => { sideEffect(); return {'@layer': {inherit: {color: p.color}}}; })",
        0
    )]
    fn object_policy_when_rules_return_or_call_preserves_the_boundary(
        #[case] source: &str,
        #[case] count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let allocator = Allocator::default();
        let expression = Parser::new(&allocator, source, SourceType::tsx())
            .parse_expression()
            .map_err(|error| format!("{error:?}"))?;
        // When
        let errors = super::object_layer_errors(&expression, "styled");
        // Then
        assert_eq!(errors.len(), count, "{errors:?}");
        for (offset, _) in errors {
            assert!(source[usize::try_from(offset)?..].starts_with("inherit"));
        }
        Ok(())
    }
}
