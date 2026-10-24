use super::super::{StyledBindings, rule_choices, template_bindings::Schedule};
use crate::{
    ExtractStyleProp,
    extractor::style_order_projection::extract_with_order,
    utils::{build_time_error, readable_code, unwrap_syntax_only},
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::GetSpan;

pub(super) fn read<'a>(
    schedule: &mut Schedule<'_, 'a>,
    expression: &Expression<'a>,
    bindings: &StyledBindings<'_>,
) -> Result<Option<Vec<ExtractStyleProp<'a>>>, (u32, String)> {
    let ast = schedule.builder;
    let invalid = || {
        (
            expression.span().start,
            build_time_error(
                "styled",
                &readable_code(expression),
                crate::utils::STYLE_OBJECT,
            ),
        )
    };
    let mut expression = unwrap_syntax_only(expression).clone_in_with_semantic_ids(ast.allocator());
    let rules = match &expression {
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
            let Some((_, rules)) = crate::css_prop::render_function(&mut expression) else {
                return Ok(None);
            };
            rules.clone_in_with_semantic_ids(ast.allocator())
        }
        _ => expression.clone_in_with_semantic_ids(ast.allocator()),
    };
    match rule_choices::normalize(ast, &rules, bindings) {
        Ok(_) => {}
        Err(rule_choices::RuleError::BoundUndefined) => return Err(invalid()),
        Err(rule_choices::RuleError::Opaque) => {
            return if written_object(&rules) {
                Err(invalid())
            } else {
                Ok(None)
            };
        }
    }
    let fixed = matches!(
        expression,
        Expression::Identifier(_) | Expression::StringLiteral(_)
    ) && bindings.styles(&expression).is_some_and(|styles| {
        styles.iter().all(|style| {
            matches!(
                style,
                crate::ExtractStyleValue::Static(_) | crate::ExtractStyleValue::LayerOrder(_)
            )
        })
    });
    if !fixed {
        schedule.rules(&mut expression, bindings)?;
    }
    MixinReader { ast, bindings }
        .read(&mut expression)
        .map(Some)
        .ok_or_else(invalid)
}

fn written_object(rules: &Expression<'_>) -> bool {
    match unwrap_syntax_only(rules) {
        Expression::ObjectExpression(_) => true,
        Expression::ConditionalExpression(branch) => {
            written_object(&branch.consequent) || written_object(&branch.alternate)
        }
        Expression::LogicalExpression(logical) => {
            written_object(&logical.left) || written_object(&logical.right)
        }
        _ => false,
    }
}

pub(super) struct MixinReader<'s, 'a, 'b> {
    pub ast: &'s AstBuilder<'a>,
    pub bindings: &'s StyledBindings<'b>,
}

impl<'a> MixinReader<'_, 'a, '_> {
    pub fn read(&self, rules: &mut Expression<'a>) -> Option<Vec<ExtractStyleProp<'a>>> {
        let Self { ast, bindings } = *self;
        if let Some(styles) = bindings.styles(rules) {
            return Some(
                styles
                    .iter()
                    .cloned()
                    .map(ExtractStyleProp::Static)
                    .collect(),
            );
        }
        match rules {
            Expression::NullLiteral(_) => Some(Vec::new()),
            Expression::ConditionalExpression(branch) => {
                let test = branch.test.clone_in_with_semantic_ids(ast.allocator());
                let yes = self.read(&mut branch.consequent)?;
                let no = self.read(&mut branch.alternate)?;
                Some(vec![ExtractStyleProp::Conditional {
                    condition: test,
                    consequent: Some(Box::new(ExtractStyleProp::StaticArray(yes))),
                    alternate: Some(Box::new(ExtractStyleProp::StaticArray(no))),
                }])
            }
            Expression::ObjectExpression(_) => {
                let (_, authored) = extract_with_order(
                    ast,
                    None,
                    rules,
                    0,
                    &None,
                    super::super::LiteralHandling::ExpandResponsiveThemeToken,
                );
                let styles = authored.0;
                let mut unreadable = Vec::new();
                crate::utils::unreadable_styles(&styles, true, &mut unreadable);
                unreadable.is_empty().then_some(styles)
            }
            _ => None,
        }
    }
}
