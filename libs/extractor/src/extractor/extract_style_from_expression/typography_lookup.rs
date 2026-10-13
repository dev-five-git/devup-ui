use crate::{ExtractStyleProp, extractor::rule_payload::RuleClass, utils::call_with_values_box};
use css::style_selector::StyleSelector;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, TemplateElement, TemplateElementValue},
    builder::AstBuilder,
};
use oxc_span::SPAN;

pub(super) fn opaque(value: &Expression<'_>) -> bool {
    match crate::utils::unwrap_syntax_only(value) {
        Expression::Identifier(identifier) => {
            !super::IGNORED_IDENTIFIERS.contains(&identifier.name.as_str())
        }
        Expression::ComputedMemberExpression(member) => !matches!(
            crate::utils::unwrap_syntax_only(&member.object),
            Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
        ),
        Expression::UnaryExpression(_)
        | Expression::BinaryExpression(_)
        | Expression::StaticMemberExpression(_)
        | Expression::ChainExpression(_)
        | Expression::CallExpression(_) => true,
        _ => false,
    }
}

pub(super) fn dynamic<'a>(
    ast: &AstBuilder<'a>,
    value: &Expression<'a>,
    placement: (u8, &Option<StyleSelector>),
) -> ExtractStyleProp<'a, RuleClass<'a>> {
    let (level, selector) = placement;
    if let Some(style) = super::conditional_typography_payload(ast, value, level, selector) {
        return style;
    }
    let saved = Expression::new_identifier(SPAN, "__devupTypography", ast);
    let elements = ["typo-", ""].into_iter().enumerate().map(|(index, text)| {
        TemplateElement::new(
            SPAN,
            TemplateElementValue {
                raw: text.into(),
                cooked: None,
            },
            index == 1,
            ast,
        )
    });
    let class = Expression::new_template_literal(
        SPAN,
        oxc_allocator::Vec::from_iter_in(elements, ast),
        oxc_allocator::Vec::from_array_in([saved.clone_in(ast.allocator())], ast),
        ast,
    );
    let selected = Expression::new_conditional_expression(
        SPAN,
        saved,
        class,
        Expression::new_string_literal(SPAN, "", None, ast),
        ast,
    );
    ExtractStyleProp::Expression {
        expression: RuleClass::Call(call_with_values_box(
            ast,
            vec![(
                "__devupTypography".to_string(),
                value.clone_in_with_semantic_ids(ast.allocator()),
            )],
            selected,
        )),
        styles: vec![],
    }
}
