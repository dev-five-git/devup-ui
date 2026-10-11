use oxc_ast::ast::Expression;

pub(super) fn kept(
    object: &oxc_ast::ast::ObjectExpression<'_>,
    takes_styles: bool,
) -> rustc_hash::FxHashSet<String> {
    if takes_styles {
        return rustc_hash::FxHashSet::default();
    }
    object
        .properties
        .iter()
        .filter_map(|property| match property {
            oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) => property
                .key
                .static_name()
                .filter(|key| key != "css")
                .map(|key| key.to_string()),
            oxc_ast::ast::ObjectPropertyKind::SpreadProperty(_) => None,
        })
        .collect()
}

pub(super) fn branches(expression: &Expression<'_>) -> bool {
    match crate::utils::unwrap_syntax_only(expression) {
        Expression::ConditionalExpression(_) | Expression::LogicalExpression(_) => true,
        Expression::ObjectExpression(object) => {
            object.properties.iter().any(|property| match property {
                oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) => {
                    branches(&property.value)
                }
                oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                    branches(&spread.argument)
                }
            })
        }
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .filter_map(|value| value.as_expression())
            .any(branches),
        Expression::TemplateLiteral(template) => template.expressions.iter().any(branches),
        _ => false,
    }
}
