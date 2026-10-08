use super::{
    StyledBindings, call_with_props, identifier, props_function, props_leaves, rule_choices,
};
use crate::{
    ExtractStyleProp,
    composition::Composition,
    css_utils::css_to_style_template,
    utils::{build_time_error, readable_code, unplaced_error, unwrap_syntax_only},
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, FormalParameters, TemplateLiteral},
    builder::AstBuilder,
};
use oxc_span::GetSpan;

#[path = "styled_mixin_context.rs"]
mod context;
#[path = "styled_template_parts.rs"]
mod parts;

pub(super) struct TemplateComposition<'a> {
    pub styles: Vec<ExtractStyleProp<'a>>,
    pub statements: Vec<usize>,
    pub errors: Vec<(u32, String)>,
}

#[cfg(test)]
#[path = "w27_styled_template_coverage.rs"]
mod coverage_tests;

pub(super) fn compose<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    bindings: &StyledBindings<'_>,
) -> TemplateComposition<'a> {
    let mut composition = Composition::default();
    let mut statements = Vec::new();
    let mut errors = Vec::new();
    for part in parts::split(ast, template, |expression| {
        bindings.styles(expression).is_some()
    }) {
        match part {
            parts::Part::Text(text) => composition.apply(
                ast,
                css_to_style_template(&text, 0, &None)
                    .styles
                    .into_iter()
                    .map(|style| ExtractStyleProp::Static(style.into()))
                    .collect(),
            ),
            parts::Part::Unplaced(index) => {
                let expression = &template.expressions[index];
                errors.push((expression.span().start, unplaced_error(expression)));
            }
            parts::Part::Mixin { index, context } => {
                let expression = &template.expressions[index];
                match mixin(ast, expression, bindings) {
                    Ok(Some(props)) => match context::contextualize(props, &context) {
                        Ok(props) => composition.apply(ast, props),
                        Err(context::ContextError::Never | context::ContextError::Opaque) => {
                            errors.push(nested_error(expression));
                        }
                    },
                    Ok(None) if context.is_empty() => statements.push(index),
                    Ok(None) => errors.push(nested_error(expression)),
                    Err(MixinError::Rules) => errors.push((
                        expression.span().start,
                        build_time_error(
                            "styled",
                            &readable_code(expression),
                            crate::utils::STYLE_OBJECT,
                        ),
                    )),
                }
            }
        }
    }
    TemplateComposition {
        styles: composition.into_props(),
        statements,
        errors,
    }
}

fn nested_error(expression: &Expression<'_>) -> (u32, String) {
    (
        expression.span().start,
        build_time_error(
            "styled",
            &readable_code(expression),
            "a nested mixin must give styles the build can read, so they inherit its enclosing selector, media query and layer",
        ),
    )
}

enum MixinError {
    Rules,
}

fn mixin<'a>(
    ast: &AstBuilder<'a>,
    expression: &Expression<'a>,
    bindings: &StyledBindings<'_>,
) -> Result<Option<Vec<ExtractStyleProp<'a>>>, MixinError> {
    let mut expression = unwrap_syntax_only(expression).clone_in_with_semantic_ids(ast.allocator());
    let (params, rules) = match &expression {
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
            let Some((params, rules)) = crate::css_prop::render_function(&mut expression) else {
                return Ok(None);
            };
            (
                Some(params.clone_in_with_semantic_ids(ast.allocator())),
                rules.clone_in_with_semantic_ids(ast.allocator()),
            )
        }
        _ => (None, expression),
    };
    let mut lowered = match rule_choices::normalize(ast, &rules, bindings) {
        Ok(lowered) => lowered,
        Err(rule_choices::RuleError::BoundUndefined) => return Err(MixinError::Rules),
        Err(rule_choices::RuleError::Opaque) => {
            return if written_object(&rules) {
                Err(MixinError::Rules)
            } else {
                Ok(None)
            };
        }
    };
    MixinReader {
        ast,
        bindings,
        params: params.as_ref(),
    }
    .read(&mut lowered)
    .map(Some)
    .ok_or(MixinError::Rules)
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

struct MixinReader<'s, 'a, 'b> {
    ast: &'s AstBuilder<'a>,
    bindings: &'s StyledBindings<'b>,
    params: Option<&'s FormalParameters<'a>>,
}

impl<'a> MixinReader<'_, 'a, '_> {
    fn read(&self, rules: &mut Expression<'a>) -> Option<Vec<ExtractStyleProp<'a>>> {
        let Self {
            ast,
            bindings,
            params,
        } = *self;
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
                let test = match params {
                    Some(params) => crate::utils::wrap_direct_call(
                        ast,
                        &props_function(
                            ast,
                            params,
                            branch.test.clone_in_with_semantic_ids(ast.allocator()),
                            "opacity",
                        ),
                        &[identifier(ast, "rest")],
                    ),
                    None => branch.test.clone_in_with_semantic_ids(ast.allocator()),
                };
                let yes = self.read(&mut branch.consequent)?;
                let no = self.read(&mut branch.alternate)?;
                let mut composition = Composition::default();
                composition.apply_conditional(ast, &test, yes, no);
                Some(composition.into_props())
            }
            Expression::ObjectExpression(object) => {
                if let Some(params) = params {
                    props_leaves(ast, object, params)?;
                }
                call_with_props(ast, object);
                let styles = super::extract_style_from_expression(
                    ast,
                    None,
                    rules,
                    0,
                    &None,
                    super::LiteralHandling::ExpandResponsiveThemeToken,
                )
                .styles;
                let mut unreadable = Vec::new();
                crate::utils::unreadable_styles(&styles, true, &mut unreadable);
                unreadable.is_empty().then_some(styles)
            }
            _ => None,
        }
    }
}
