use crate::prop_extract_utils::extract_style_prop_from_express;
use crate::utils::is_special_property;
use crate::ExtractStyleProp;
use oxc_ast::ast::{
    ArrayExpressionElement, Expression, JSXAttributeValue, JSXExpression, ObjectExpression,
    ObjectPropertyKind,
};
use oxc_ast::AstBuilder;

pub fn extract_object_from_jsx_attr<'a>(
    ast_builder: &AstBuilder<'a>,
    value: &JSXAttributeValue<'a>,
    media: Option<&str>,
) -> Option<Vec<ExtractStyleProp<'a>>> {
    match value {
        JSXAttributeValue::ExpressionContainer(expression) => match &expression.expression {
            JSXExpression::ObjectExpression(obj) => {
                let mut props = vec![];
                props.append(&mut extract_props_from_object_expression(
                    ast_builder,
                    obj,
                    0,
                    media,
                ));
                Some(props)
            }
            JSXExpression::ArrayExpression(arr) => {
                let mut props = vec![];
                for (idx, e) in arr.elements.iter().enumerate() {
                    if let ArrayExpressionElement::ObjectExpression(oo) = e {
                        props.append(&mut extract_props_from_object_expression(
                            ast_builder,
                            oo,
                            idx as u8,
                            media,
                        ));
                    }
                }
                Some(props)
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn extract_from_style_value<'a>(
    ast_builder: &AstBuilder<'a>,
    value: &Expression<'a>,
    media: Option<&str>,
) -> Option<Vec<ExtractStyleProp<'a>>> {
    match value {
        Expression::ObjectExpression(obj) => {
            let mut props = vec![];
            props.append(&mut extract_props_from_object_expression(
                ast_builder,
                obj,
                0,
                media,
            ));
            Some(props)
        }
        Expression::ArrayExpression(arr) => {
            let mut props = vec![];
            for (idx, e) in arr.elements.iter().enumerate() {
                if let ArrayExpressionElement::ObjectExpression(oo) = e {
                    props.append(&mut extract_props_from_object_expression(
                        ast_builder,
                        oo,
                        idx as u8,
                        media,
                    ));
                }
            }
            Some(props)
        }
        _ => None,
    }
}

fn extract_props_from_object_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    obj: &ObjectExpression<'a>,
    level: u8,
    media: Option<&str>,
) -> Vec<ExtractStyleProp<'a>> {
    let mut props = vec![];
    for p in obj.properties.iter() {
        if let ObjectPropertyKind::ObjectProperty(o) = p {
            let name = o.key.name().unwrap();
            if is_special_property(&name) {
                continue;
            }
            if let Some(ret) =
                extract_style_prop_from_express(ast_builder, &name, &o.value, level, media)
            {
                props.push(ret);
            }
        };
    }
    props
}
