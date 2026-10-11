use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};

use super::{json_string, style_references::StyleReferences};

pub(super) fn rewrite(json: &str, references: &StyleReferences) -> String {
    let source = format!("({json})");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::mjs()).parse();
    let mut replacements = Vec::new();
    for statement in &parsed.program.body {
        if let oxc_ast::ast::Statement::ExpressionStatement(statement) = statement {
            rules(&statement.expression, references, &mut replacements);
        }
    }
    let mut result = json.to_string();
    for (span, value) in replacements.into_iter().rev() {
        let start = Span::new(0, span.start).source_text(&source).len();
        let end = start + span.source_text(&source).len();
        result.replace_range(start - 1..end - 1, &value);
    }
    result
}

fn rules(
    expression: &Expression<'_>,
    references: &StyleReferences,
    replacements: &mut Vec<(Span, String)>,
) {
    let expression = crate::utils::unwrap_syntax_only(expression);
    let Expression::ObjectExpression(object) = expression else {
        return;
    };
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        let Some(name) = property.key.static_name() else {
            continue;
        };
        if name == "selectors" {
            if let Expression::ObjectExpression(selectors) = &property.value {
                for selector in &selectors.properties {
                    if let ObjectPropertyKind::ObjectProperty(selector) = selector {
                        if let Some(key) = selector.key.static_name() {
                            let resolved = references.selector(&key);
                            if resolved != key {
                                let span = selector.key.span();
                                replacements.push((span, json_string(&resolved)));
                            }
                        }
                        rules(&selector.value, references, replacements);
                    }
                }
            }
        } else if matches!(
            name.as_ref(),
            "@media" | "@supports" | "@container" | "@layer" | "@scope"
        ) {
            if let Expression::ObjectExpression(queries) = &property.value {
                for query in &queries.properties {
                    if let ObjectPropertyKind::ObjectProperty(query) = query {
                        rules(&query.value, references, replacements);
                    }
                }
            }
        } else if name.starts_with(':') || name.starts_with('_') || name == "@starting-style" {
            rules(&property.value, references, replacements);
        }
    }
}
