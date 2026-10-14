use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{BinaryOperator, Expression, Str},
    builder::AstBuilder,
};

use crate::{
    extract_style::extract_dynamic_style::ExtractDynamicStyle,
    utils::{expression_to_code, unwrap_syntax_only},
};

/// Recover only a fixed value tail removed from the complete extracted expression.
pub(crate) fn suffix(
    ast: &AstBuilder<'_>,
    source: &Expression<'_>,
    style: &ExtractDynamicStyle,
) -> Option<String> {
    let source = unwrap_syntax_only(source);
    let mut normalized = source.clone_in(ast.allocator());
    let tail = match &mut normalized {
        Expression::TemplateLiteral(template) => template.quasis.last_mut()?,
        Expression::BinaryExpression(binary)
            if binary.operator == BinaryOperator::Addition && string_result(&binary.left) =>
        {
            match &mut binary.right {
                Expression::StringLiteral(value) => {
                    let original = value.value.as_str();
                    let cleaned = clean_tail(original, style.important());
                    let suffix = original.strip_prefix(cleaned)?.to_string();
                    value.value = Str::from(ast.allocator().alloc_str(cleaned));
                    value.raw = None;
                    return (expression_to_code(&normalized).trim().trim_end_matches(';')
                        == style.identifier())
                    .then_some(suffix);
                }
                Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
                    template.quasis.last_mut()?
                }
                _ => return None,
            }
        }
        _ if style.important() => return Some(" !important".to_string()),
        _ => return None,
    };
    let original = tail.value.raw.as_str();
    let cleaned = clean_tail(original, style.important());
    let suffix = original.strip_prefix(cleaned)?.to_string();
    tail.value.raw = Str::from(ast.allocator().alloc_str(cleaned));
    if let Some(cooked) = tail.value.cooked {
        tail.value.cooked = Some(Str::from(
            ast.allocator()
                .alloc_str(clean_tail(cooked.as_str(), style.important())),
        ));
    }
    (expression_to_code(&normalized).trim().trim_end_matches(';') == style.identifier())
        .then_some(suffix)
}

fn clean_tail(value: &str, important: bool) -> &str {
    let value = value.trim_end_matches(';');
    if important {
        value.strip_suffix(" !important").unwrap_or(value)
    } else {
        value
    }
}

fn string_result(source: &Expression<'_>) -> bool {
    match unwrap_syntax_only(source) {
        Expression::TemplateLiteral(_) | Expression::StringLiteral(_) => true,
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            string_result(&binary.left) || string_result(&binary.right)
        }
        _ => false,
    }
}
