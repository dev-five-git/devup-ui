use super::{FONT_TEMPLATE_INPUT, GlobalInputs, reject};
use crate::{
    ExtractStyleProp,
    css_utils::{CssToStyleResult, css_to_style_template, theme_var_reference},
    extract_style::{extract_font_face::ExtractFontFace, extract_style_value::ExtractStyleValue},
    extractor::extract_style_from_expression::{flatten_spreads, unreadable_key},
    utils::{
        get_str_by_property_key, get_string_by_literal_expression, unwrap_syntax_only,
        unwrap_syntax_only_mut,
    },
};
use css::{
    disassemble_property,
    optimize_multi_css_value::{check_multi_css_optimize, optimize_multi_css_value, wrap_url},
};
use oxc_ast::{
    ast::{ArrayExpressionElement, Expression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::GetSpan;
use std::{borrow::Cow, collections::BTreeMap};

const FONT_TEXT: &str = "font-face descriptors and interpolations must be static text; templates may use theme references only in descriptor values; descriptor spreads must be known at build time";

pub(crate) fn font_faces<'a>(
    ast: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    inputs: &GlobalInputs<'_, '_>,
) -> Vec<ExtractStyleProp<'a>> {
    let expression = unwrap_syntax_only_mut(expression);
    let Expression::ArrayExpression(array) = expression else {
        return if inputs.absent(expression)
            || get_string_by_literal_expression(expression).is_some()
        {
            vec![]
        } else {
            vec![reject(expression, FONT_TEXT)]
        };
    };
    let mut styles = Vec::new();
    for entry in &mut array.elements {
        let start = styles.len();
        let properties = match entry {
            ArrayExpressionElement::ObjectExpression(object) => {
                flatten_spreads(ast, object);
                let invalid: Vec<_> = object
                    .properties
                    .iter()
                    .filter_map(|property| match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            crate::dead_properties::authored_declaration_error(
                                &get_str_by_property_key(&property.key)?,
                                property.key.span().start,
                            )
                        }
                        ObjectPropertyKind::SpreadProperty(_) => None,
                    })
                    .collect();
                if !invalid.is_empty() {
                    styles.extend(invalid);
                    continue;
                }
                let mut properties = BTreeMap::new();
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            styles.push(reject(&spread.argument, FONT_TEXT));
                        }
                        ObjectPropertyKind::ObjectProperty(property) => {
                            let Some(name) = get_str_by_property_key(&property.key) else {
                                styles.push(unreadable_key(&property.key, false));
                                continue;
                            };
                            let Some(text) = get_string_by_literal_expression(&property.value)
                            else {
                                if !inputs.absent(&property.value) {
                                    styles.push(reject(&property.value, FONT_TEXT));
                                }
                                continue;
                            };
                            for name in disassemble_property(&name) {
                                let value = if check_multi_css_optimize(&name) {
                                    optimize_multi_css_value(&text)
                                } else {
                                    Cow::Borrowed(text.as_ref())
                                };
                                let value = if name == "src" {
                                    wrap_url(&value).into_owned()
                                } else {
                                    value.into_owned()
                                };
                                properties.insert(name.into_owned(), value);
                            }
                        }
                    }
                }
                properties
            }
            ArrayExpressionElement::TemplateLiteral(template) => {
                let invalid = crate::dead_properties::template_errors(template);
                if !invalid.is_empty() {
                    styles.extend(invalid);
                    continue;
                }
                for expression in &template.expressions {
                    if get_string_by_literal_expression(expression).is_none()
                        && theme_var_reference(expression).is_none()
                    {
                        let requirement = if matches!(
                            unwrap_syntax_only(expression),
                            Expression::ArrowFunctionExpression(_)
                                | Expression::FunctionExpression(_)
                                | Expression::ClassExpression(_)
                        ) {
                            FONT_TEMPLATE_INPUT
                        } else {
                            FONT_TEXT
                        };
                        styles.push(reject(expression, requirement));
                    }
                }
                if styles.len() != start {
                    continue;
                }
                let parsed = css_to_style_template(template, 0, &None);
                styles.extend(
                    parsed
                        .statements
                        .into_iter()
                        .chain(parsed.unplaced)
                        .map(|index| reject(&template.expressions[index], FONT_TEMPLATE_INPUT)),
                );
                if styles.len() != start {
                    continue;
                }
                static_properties(parsed.styles)
            }
            ArrayExpressionElement::SpreadElement(spread) => {
                styles.push(reject(&spread.argument, FONT_TEXT));
                continue;
            }
            ArrayExpressionElement::Elision(_) => continue,
            _ => {
                let expression = entry.to_expression();
                if !inputs.absent(expression)
                    && get_string_by_literal_expression(expression).is_none()
                {
                    styles.push(reject(expression, FONT_TEXT));
                }
                continue;
            }
        };
        if styles.len() == start {
            styles.push(ExtractStyleProp::Static(ExtractStyleValue::FontFace(
                ExtractFontFace {
                    file: inputs.file.to_string(),
                    order: 0,
                    properties,
                },
            )));
        }
    }
    styles
}

fn static_properties(styles: Vec<CssToStyleResult>) -> BTreeMap<String, String> {
    styles
        .into_iter()
        .filter_map(|style| match style {
            CssToStyleResult::Static(style) => Some((style.property, style.value)),
            CssToStyleResult::Dynamic(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css_utils::css_to_style_literal;
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    #[test]
    fn parser_projection_only_retains_static_descriptors() {
        let arena = Allocator::default();
        let parsed = Parser::new(
            &arena,
            "`font-family: Face; font-weight: ${weight};`;",
            SourceType::tsx(),
        )
        .parse();
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("expected expression")
        };
        let Expression::TemplateLiteral(template) = &statement.expression else {
            panic!("expected template")
        };
        let properties = static_properties(css_to_style_literal(template, 0, &None));
        assert_eq!(
            properties,
            BTreeMap::from([("font-family".to_string(), "Face".to_string())])
        );
    }
}
