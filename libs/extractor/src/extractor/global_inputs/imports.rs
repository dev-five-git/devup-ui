use super::{GlobalInputs, reject};
use crate::{
    ExtractStyleProp,
    extract_style::{extract_import::ExtractImport, extract_style_value::ExtractStyleValue},
    extractor::extract_style_from_expression::flatten_spreads,
    utils::{get_str_by_property_key, get_string_by_literal_expression, unwrap_syntax_only_mut},
};
use oxc_ast::{
    ast::{ArrayExpressionElement, Expression, ObjectPropertyKind},
    builder::AstBuilder,
};

const IMPORT_TEXT: &str = "import URLs and queries must be static text; import entries and spreads must be known at build time";

pub(crate) fn imports<'a>(
    ast: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    inputs: &GlobalInputs<'_, '_>,
) -> Vec<ExtractStyleProp<'a>> {
    let expression = unwrap_syntax_only_mut(expression);
    let Expression::ArrayExpression(array) = expression else {
        return if inputs.absent(expression)
            || get_string_by_literal_expression(expression).is_some()
            || matches!(expression, Expression::ObjectExpression(object) if object.properties.is_empty())
        {
            vec![]
        } else {
            vec![reject(expression, IMPORT_TEXT)]
        };
    };
    let mut styles = Vec::new();
    for entry in &mut array.elements {
        match entry {
            ArrayExpressionElement::SpreadElement(spread) => {
                styles.push(reject(&spread.argument, IMPORT_TEXT));
            }
            ArrayExpressionElement::Elision(_) => {}
            _ => {
                let element = entry.to_expression_mut();
                let text = match unwrap_syntax_only_mut(element) {
                    Expression::ObjectExpression(object) => {
                        flatten_spreads(ast, object);
                        let mut url = None;
                        let mut query = None;
                        let start = styles.len();
                        for property in &object.properties {
                            match property {
                                ObjectPropertyKind::SpreadProperty(spread) => {
                                    styles.push(reject(&spread.argument, IMPORT_TEXT));
                                }
                                ObjectPropertyKind::ObjectProperty(property) => {
                                    let Some(name) = get_str_by_property_key(&property.key) else {
                                        styles.push(crate::extractor::extract_style_from_expression::unreadable_key(&property.key, false));
                                        continue;
                                    };
                                    if matches!(name.as_ref(), "url" | "query") {
                                        let value = &property.value;
                                        let text = get_string_by_literal_expression(value);
                                        if text.is_none() && !inputs.absent(value) {
                                            styles.push(reject(value, IMPORT_TEXT));
                                        }
                                        if name == "url" {
                                            url = text;
                                        } else {
                                            query = text;
                                        }
                                    }
                                }
                            }
                        }
                        if styles.len() != start {
                            continue;
                        }
                        url.map(|url| {
                            let mut text = String::with_capacity(
                                url.len() + 2 + query.as_ref().map_or(0, |query| query.len() + 1),
                            );
                            text.push('"');
                            text.push_str(&url);
                            text.push('"');
                            if let Some(query) = query {
                                text.push(' ');
                                text.push_str(&query);
                            }
                            text
                        })
                    }
                    Expression::NumericLiteral(_) => None,
                    element => match get_string_by_literal_expression(element) {
                        Some(text) => Some(text.into_owned()),
                        None if inputs.absent(element) => None,
                        None => {
                            styles.push(reject(element, IMPORT_TEXT));
                            None
                        }
                    },
                };
                if let Some(url) = text {
                    styles.push(ExtractStyleProp::Static(ExtractStyleValue::Import(
                        ExtractImport {
                            file: inputs.file.to_string(),
                            order: 0,
                            url,
                        },
                    )));
                }
            }
        }
    }
    styles
}
