use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, TemplateLiteral};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, Span};

pub(super) fn token<'a>(ast: &AstBuilder<'a>, value: &str, span: Span) -> Expression<'a> {
    let numeric = !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'e' | b'E'));
    if numeric && let Ok(number) = value.parse::<f64>() {
        return Expression::new_numeric_literal(
            span,
            number,
            None,
            oxc_ast::ast::NumberBase::Decimal,
            ast,
        );
    }
    let value = decode(value).unwrap_or_else(|| value.to_string());
    Expression::new_string_literal(span, ast.allocator().alloc_str(&value), None, ast)
}

fn decode(value: &str) -> Option<String> {
    let delimiter = value.chars().next()?;
    if !matches!(delimiter, '\'' | '"') || !value.ends_with(delimiter) || value.len() < 2 {
        return None;
    }
    let mut chars = value[1..value.len() - 1].chars().peekable();
    let mut result = String::new();
    while let Some(character) = chars.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        let character = chars.next()?;
        if character.is_ascii_hexdigit() {
            let mut hex = character.to_string();
            while hex.len() < 6 && chars.peek().is_some_and(char::is_ascii_hexdigit) {
                hex.push(chars.next()?);
            }
            if chars
                .peek()
                .is_some_and(|character| character.is_whitespace())
            {
                chars.next();
            }
            result.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?).unwrap_or('\u{fffd}'));
        } else if character != '\n' && character != '\r' {
            result.push(character);
        }
    }
    Some(result)
}

pub(super) fn canonical_key(value: &str) -> String {
    if value.contains('\\')
        && let Some(decoded) = decode(&format!("'{value}'"))
        && crate::style_order::reserved(&decoded)
    {
        decoded
    } else {
        value.to_string()
    }
}

pub(super) fn finite_text<'a>(
    ast: &AstBuilder<'a>,
    template: oxc_allocator::Box<'a, TemplateLiteral<'a>>,
) -> Expression<'a> {
    for (index, expression) in template.expressions.iter().enumerate() {
        if let Expression::ConditionalExpression(condition) =
            crate::utils::unwrap_syntax_only(expression)
        {
            let mut yes = template.clone_in_with_semantic_ids(ast.allocator());
            let mut no = template.clone_in_with_semantic_ids(ast.allocator());
            yes.expressions[index] = condition
                .consequent
                .clone_in_with_semantic_ids(ast.allocator());
            no.expressions[index] = condition
                .alternate
                .clone_in_with_semantic_ids(ast.allocator());
            return Expression::new_conditional_expression(
                template.span(),
                condition.test.clone_in_with_semantic_ids(ast.allocator()),
                finite_text(ast, yes),
                finite_text(ast, no),
                ast,
            );
        }
    }
    let mut result = String::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        result.push_str(&quasi.value.raw);
        if let Some(expression) = template.expressions.get(index) {
            let Some(text) = crate::utils::get_string_by_literal_expression(expression) else {
                return Expression::TemplateLiteral(template);
            };
            result.push_str(&text);
        }
    }
    let result = decode(&result).unwrap_or(result);
    Expression::new_string_literal(
        template.span(),
        ast.allocator().alloc_str(&result),
        None,
        ast,
    )
}
