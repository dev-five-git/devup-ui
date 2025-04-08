use once_cell::sync::Lazy;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, JSXAttributeValue, Statement};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::{SPAN, SourceType};
use oxc_syntax::operator::UnaryOperator;
use std::collections::HashSet;

/// Convert a value to a pixel value
pub fn convert_value(value: &str) -> String {
    value
        .parse::<f64>()
        .map_or_else(|_| value.to_string(), |num| format!("{}px", num * 4.0))
}

pub fn expression_to_code(expression: &Expression) -> String {
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, "", SourceType::d_ts()).parse();
    parsed.program.body.insert(
        0,
        Statement::ExpressionStatement(
            oxc_ast::AstBuilder::new(&allocator)
                .alloc_expression_statement(SPAN, expression.clone_in(&allocator)),
        ),
    );
    let code = Codegen::new().build(&parsed.program).code;
    code[0..code.len() - 2].to_string()
}

static SPECIAL_PROPERTIES: Lazy<HashSet<&str>> = Lazy::new(|| {
    let mut set = HashSet::<&str>::new();
    for prop in [
        "style",
        "className",
        "role",
        "ref",
        "key",
        "id",
        "alt",
        "type",
        "src",
        "children",
        "placeholder",
        "tabIndex",
        "maxLength",
        "minLength",
        "disabled",
        "readOnly",
        "autoFocus",
        "required",
        "checked",
        "defaultChecked",
        "value",
        "defaultValue",
        "selected",
        "multiple",
        "accept",
    ] {
        set.insert(prop);
    }
    set
});

pub fn is_special_property(name: &str) -> bool {
    name.starts_with("on")
        || name.starts_with("data-")
        || name.starts_with("aria-")
        || SPECIAL_PROPERTIES.contains(name)
}

pub fn get_number_by_jsx_expression(expr: &JSXAttributeValue) -> Option<f64> {
    match expr {
        JSXAttributeValue::StringLiteral(sl) => get_number_by_literal_expression(
            &Expression::StringLiteral(sl.clone_in(&Allocator::default())),
        ),
        JSXAttributeValue::ExpressionContainer(ec) => {
            get_number_by_literal_expression(ec.expression.to_expression())
        }
        _ => None,
    }
}

pub fn get_number_by_literal_expression(expr: &Expression) -> Option<f64> {
    match expr {
        Expression::ParenthesizedExpression(parenthesized) => {
            get_number_by_literal_expression(&parenthesized.expression)
        }
        Expression::NumericLiteral(num) => Some(num.value),
        Expression::UnaryExpression(unary) => get_number_by_literal_expression(&unary.argument)
            .and_then(|num| match unary.operator {
                UnaryOperator::UnaryNegation => Some(-num),
                UnaryOperator::UnaryPlus => Some(num),
                _ => None,
            }),
        _ => None,
    }
}

pub fn get_string_by_literal_expression(expr: &Expression) -> Option<String> {
    get_number_by_literal_expression(expr)
        .map(|num| num.to_string())
        .or_else(|| match expr {
            Expression::ParenthesizedExpression(parenthesized) => {
                get_string_by_literal_expression(&parenthesized.expression)
            }
            Expression::StringLiteral(str) => Some(str.value.as_str().to_string()),
            Expression::TemplateLiteral(tmp) => {
                let mut collect = vec![];
                for (idx, q) in tmp.quasis.iter().enumerate() {
                    collect.push(q.value.raw.to_string());
                    if idx < tmp.expressions.len() {
                        if let Some(value) = get_string_by_literal_expression(&tmp.expressions[idx])
                        {
                            collect.push(value);
                        } else {
                            return None;
                        }
                    }
                }
                Some(collect.join(""))
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_value() {
        assert_eq!(convert_value("1px"), "1px");
        assert_eq!(convert_value("1%"), "1%");
        assert_eq!(convert_value("foo"), "foo");
        assert_eq!(convert_value("4"), "16px");
    }
}
