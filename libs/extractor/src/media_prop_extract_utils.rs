use crate::prop_extract_utils::extract_style_prop_from_express;
use crate::ExtractStyleProp;
use oxc_ast::ast::{
    ArrayExpressionElement, JSXAttributeValue, JSXExpression, ObjectExpression, ObjectPropertyKind,
};
use oxc_ast::AstBuilder;

pub fn extract_media_prop<'a>(
    ast_builder: &AstBuilder<'a>,
    value: &JSXAttributeValue<'a>,
    media: &str,
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

fn extract_props_from_object_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    obj: &ObjectExpression<'a>,
    level: u8,
    media: &str,
) -> Vec<ExtractStyleProp<'a>> {
    let mut props = vec![];
    for p in obj.properties.iter() {
        if let ObjectPropertyKind::ObjectProperty(o) = p {
            if let Some(ret) = extract_style_prop_from_express(
                ast_builder,
                o.key.name().unwrap().to_string().as_str(),
                &o.value,
                level,
                Some(media),
            ) {
                props.push(ret);
            }
        };
    }
    props
}
