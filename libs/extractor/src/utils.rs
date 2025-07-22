use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, JSXAttributeValue, Statement};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::{SPAN, SourceType};
use oxc_syntax::operator::UnaryOperator;
use phf::phf_set;

/// Convert a value to a pixel value
pub(super) fn convert_value(value: &str) -> String {
    value
        .parse::<f64>()
        .map_or_else(|_| value.to_string(), |num| format!("{}px", num * 4.0))
}

pub(super) fn expression_to_code(expression: &Expression) -> String {
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

pub(super) fn is_same_expression<'a>(a: &Expression<'a>, b: &Expression<'a>) -> bool {
    match (a, b) {
        (Expression::StringLiteral(a), Expression::StringLiteral(b)) => a.value == b.value,
        (Expression::TemplateLiteral(a), Expression::TemplateLiteral(b)) => {
            a.quasis.len() == b.quasis.len()
                && a.expressions.len() == b.expressions.len()
                && a.quasis
                    .iter()
                    .zip(b.quasis.iter())
                    .all(|(a, b)| a.value.raw == b.value.raw && a.tail == b.tail)
                && a.expressions
                    .iter()
                    .zip(b.expressions.iter())
                    .all(|(a, b)| is_same_expression(a, b))
        }
        (Expression::Identifier(a), Expression::Identifier(b)) => a.name == b.name,
        _ => false,
    }
}

static SPECIAL_PROPERTIES: phf::Set<&str> = phf_set! {
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
    "step",
    "autoComplete",
    "capture",
    "form",
    "formAction",
    "formEncType",
    "formMethod",
    "formNoValidate",
    "formTarget",
    "list",
    "max",
    "min",
    "name",
    "pattern",
    "size",
    "challenge",
    "keyType",
    "keyParams",
    "htmlFor",
    "crossOrigin",
    "fetchPriority",
    "href",
    "hrefLang",
    "integrity",
    "media",
    "imageSrcSet",
    "imageSizes",
    "referrerPolicy",
    "sizes",
    "charSet",
    "precedence",
    "autoPlay",
    "controls",
    "controlsList",
    "loop",
    "mediaGroup",
    "muted",
    "playsInline",
    "preload",
    "httpEquiv",
    "high",
    "low",
    "optimum",
    "classID",
    "data",
    "useMap",
    "wmode",
    "reversed",
    "start",
    "label",
    "async",
    "defer",
    "noModule",
    "srcSet",
    "scoped",
    "align",
    "bgcolor",
    "cellPadding",
    "cellSpacing",
    "frame",
    "rules",
    "summary",
    "cols",
    "dirName",
    "rows",
    "wrap",
    "colSpan",
    "headers",
    "rowSpan",
    "scope",
    "abbr",
    "valign",
    "dateTime",
    "default",
    "kind",
    "srcLang",
    "poster",
    "disablePictureInPicture",
    "disableRemotePlayback",
    "download",
    "target",
    "rel",
    "ping",
    "coords",
    "shape",
    "isMap",
    "longDesc",
    "loading",
    "decoding",
    "importance",
    "axis",
    "char",
    "charOff",
    "span",
    "noWrap",
    "vSpace",
    "hSpace",
    "compact",
    "scheme",
    "indeterminate",
    "defaultSelected",
    "selectedIndex",
    "selectedOptions"
};

pub(super) fn is_special_property(name: &str) -> bool {
    name.starts_with("on")
        || name.starts_with("data-")
        || name.starts_with("aria-")
        || SPECIAL_PROPERTIES.contains(name)
}

pub(super) fn jsx_expression_to_number(expr: &JSXAttributeValue) -> Option<f64> {
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

pub(super) fn get_number_by_literal_expression(expr: &Expression) -> Option<f64> {
    match expr {
        Expression::ParenthesizedExpression(parenthesized) => {
            get_number_by_literal_expression(&parenthesized.expression)
        }
        Expression::StringLiteral(sl) => sl.value.parse::<f64>().ok(),
        Expression::TemplateLiteral(tmp) => tmp
            .quasis
            .iter()
            .map(|q| q.value.raw.to_string())
            .collect::<String>()
            .parse::<f64>()
            .ok(),
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

pub(super) fn get_string_by_literal_expression(expr: &Expression) -> Option<String> {
    get_number_by_literal_expression(expr)
        .map(|num| num.to_string())
        .or_else(|| match expr {
            Expression::ParenthesizedExpression(parenthesized) => {
                get_string_by_literal_expression(&parenthesized.expression)
            }
            Expression::StringLiteral(str) => Some(str.value.into()),
            Expression::BooleanLiteral(bool) => Some(bool.value.to_string()),
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
    use oxc_allocator::Vec;
    use oxc_ast::ast::NumberBase;

    use super::*;

    #[test]
    fn test_convert_value() {
        assert_eq!(convert_value("1px"), "1px");
        assert_eq!(convert_value("1%"), "1%");
        assert_eq!(convert_value("foo"), "foo");
        assert_eq!(convert_value("4"), "16px");
    }

    #[test]
    fn test_get_number_by_literal_expression() {
        let allocator = Allocator::default();
        {
            let parsed = Parser::new(&allocator, "1", SourceType::d_ts()).parse();
            assert_eq!(parsed.program.body.len(), 1);
            assert!(matches!(
                parsed.program.body[0],
                Statement::ExpressionStatement(_)
            ));
            if let Statement::ExpressionStatement(expr) = &parsed.program.body[0] {
                assert_eq!(
                    get_number_by_literal_expression(&expr.expression),
                    Some(1.0)
                );
            }
        }
        {
            let parsed = Parser::new(&allocator, "-1", SourceType::d_ts()).parse();
            assert_eq!(parsed.program.body.len(), 1);
            assert!(matches!(
                parsed.program.body[0],
                Statement::ExpressionStatement(_)
            ));
            if let Statement::ExpressionStatement(expr) = &parsed.program.body[0] {
                assert_eq!(
                    get_number_by_literal_expression(&expr.expression),
                    Some(-1.0)
                );
            }
        }
        {
            let parsed = Parser::new(&allocator, "1.5", SourceType::d_ts()).parse();
            assert_eq!(parsed.program.body.len(), 1);
            assert!(matches!(
                parsed.program.body[0],
                Statement::ExpressionStatement(_)
            ));
            if let Statement::ExpressionStatement(expr) = &parsed.program.body[0] {
                assert_eq!(
                    get_number_by_literal_expression(&expr.expression),
                    Some(1.5)
                );
            }
        }
        {
            let parsed = Parser::new(&allocator, "delete 1", SourceType::d_ts()).parse();
            assert_eq!(parsed.program.body.len(), 1);
            assert!(matches!(
                parsed.program.body[0],
                Statement::ExpressionStatement(_)
            ));
            if let Statement::ExpressionStatement(expr) = &parsed.program.body[0] {
                assert_eq!(get_number_by_literal_expression(&expr.expression), None);
            }
        }
    }

    #[test]
    fn test_jsx_expression_to_number() {
        let allocator = Allocator::default();
        let builder = oxc_ast::AstBuilder::new(&allocator);
        assert_eq!(
            jsx_expression_to_number(
                builder
                    .jsx_attribute(
                        SPAN,
                        builder.jsx_attribute_name_identifier(SPAN, "styleOrder"),
                        Some(builder.jsx_attribute_value_string_literal(SPAN, "1", None)),
                    )
                    .value
                    .as_ref()
                    .unwrap()
            ),
            Some(1.0)
        );

        assert_eq!(
            jsx_expression_to_number(
                builder
                    .jsx_attribute(
                        SPAN,
                        builder.jsx_attribute_name_identifier(SPAN, "styleOrder"),
                        Some(builder.jsx_attribute_value_element(
                            SPAN,
                            builder.jsx_opening_element(
                                SPAN,
                                builder.jsx_element_name_identifier(SPAN, "div"),
                                Some(builder.ts_type_parameter_instantiation(
                                    SPAN,
                                    Vec::new_in(&allocator)
                                )),
                                Vec::new_in(&allocator),
                            ),
                            Vec::new_in(&allocator),
                            Some(builder.jsx_closing_element(
                                SPAN,
                                builder.jsx_element_name_identifier(SPAN, "div"),
                            )),
                        ))
                    )
                    .value
                    .as_ref()
                    .unwrap()
            ),
            None
        );
    }
    #[test]
    fn test_get_string_by_literal_expression() {
        let allocator = Allocator::default();
        let builder = oxc_ast::AstBuilder::new(&allocator);

        let expr = builder.expression_string_literal(SPAN, "hello", None);
        assert_eq!(
            super::get_string_by_literal_expression(&expr),
            Some("hello".to_string())
        );

        let expr = builder.expression_numeric_literal(SPAN, 42.0, None, NumberBase::Decimal);
        assert_eq!(
            super::get_string_by_literal_expression(&expr),
            Some("42".to_string())
        );

        let expr = builder.expression_boolean_literal(SPAN, true);
        assert_eq!(
            super::get_string_by_literal_expression(&expr),
            Some("true".to_string())
        );

        let expr = builder.expression_template_literal(
            SPAN,
            oxc_allocator::Vec::from_iter_in(
                vec![builder.template_element(
                    SPAN,
                    oxc_ast::ast::TemplateElementValue {
                        cooked: Some("template".into()),
                        raw: "template".into(),
                    },
                    true,
                )],
                &allocator,
            ),
            oxc_allocator::Vec::new_in(&allocator),
        );
        assert_eq!(
            super::get_string_by_literal_expression(&expr),
            Some("template".to_string())
        );

        let expr = builder.expression_template_literal(
            SPAN,
            oxc_allocator::Vec::from_iter_in(
                vec![
                    builder.template_element(
                        SPAN,
                        oxc_ast::ast::TemplateElementValue {
                            cooked: Some("a".into()),
                            raw: "a".into(),
                        },
                        false,
                    ),
                    builder.template_element(
                        SPAN,
                        oxc_ast::ast::TemplateElementValue {
                            cooked: Some("b".into()),
                            raw: "b".into(),
                        },
                        true,
                    ),
                ],
                &allocator,
            ),
            oxc_allocator::Vec::from_iter_in(
                vec![builder.expression_identifier(SPAN, builder.atom("x"))],
                &allocator,
            ),
        );
        assert_eq!(super::get_string_by_literal_expression(&expr), None);

        // Identifier 등 기타 타입 - None 반환
        let expr = builder.expression_identifier(SPAN, builder.atom("foo"));
        assert_eq!(super::get_string_by_literal_expression(&expr), None);
    }
}
