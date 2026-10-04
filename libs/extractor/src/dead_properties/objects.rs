use super::authored_requirement;
use crate::utils::{
    RESPONSIVE_ARRAY, get_string_by_property_key, readable_code, unwrap_syntax_only,
};
use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_span::GetSpan;

#[derive(Clone, Copy)]
pub(crate) enum ObjectKind {
    Styles,
    Records,
    Selectors,
    Globals,
}

pub(crate) fn authored_errors(
    expression: &Expression<'_>,
    kind: ObjectKind,
) -> Vec<(u32, String, &'static str)> {
    let mut declarations = Declarations::default();
    declarations.collect(expression, kind);
    declarations.errors
}

#[derive(Default)]
struct Declarations {
    path: Vec<String>,
    errors: Vec<(u32, String, &'static str)>,
    evaluated: bool,
    responsive_entry: bool,
}

impl Declarations {
    fn collect(&mut self, expression: &Expression<'_>, kind: ObjectKind) {
        match unwrap_syntax_only(expression) {
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            let Some(name) = get_string_by_property_key(&property.key) else {
                                continue;
                            };
                            self.path.push(name.clone());
                            match kind {
                                ObjectKind::Styles => {
                                    if let Some(requirement) = authored_requirement(&name) {
                                        self.errors.push((
                                            property.key.span().start,
                                            self.path.join(" -> "),
                                            requirement,
                                        ));
                                    } else if !matches!(
                                        name.as_str(),
                                        "vars" | "props" | "styleVars" | "params" | "as"
                                    ) && !css::is_special_property::is_special_property(
                                        &name,
                                    ) {
                                        let nested = if matches!(
                                            name.as_str(),
                                            "selectors"
                                                | "@media"
                                                | "@supports"
                                                | "@container"
                                                | "_media"
                                                | "_supports"
                                                | "_container"
                                                | "@layer"
                                        ) {
                                            ObjectKind::Selectors
                                        } else {
                                            ObjectKind::Styles
                                        };
                                        self.responsive(&property.value, nested);
                                    }
                                }
                                ObjectKind::Records => {
                                    self.collect(&property.value, ObjectKind::Styles);
                                }
                                ObjectKind::Selectors => {
                                    self.responsive(&property.value, ObjectKind::Styles);
                                }
                                ObjectKind::Globals => {
                                    if name != "imports" {
                                        self.collect(
                                            &property.value,
                                            if name.starts_with('@') || name.starts_with('_') {
                                                ObjectKind::Globals
                                            } else {
                                                ObjectKind::Styles
                                            },
                                        );
                                    }
                                }
                            }
                            self.path.pop();
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.collect(&spread.argument, kind);
                        }
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                if self.responsive_entry {
                    self.responsive(expression, kind);
                    return;
                }
                for (index, item) in array.elements.iter().enumerate() {
                    if let Some(expression) = item.as_expression() {
                        self.path.push(index.to_string());
                        self.collect(expression, kind);
                        self.path.pop();
                    }
                }
            }
            _ => {}
        }
    }

    fn responsive(&mut self, expression: &Expression<'_>, kind: ObjectKind) {
        match unwrap_syntax_only(expression) {
            Expression::ArrayExpression(array) if self.responsive_entry => {
                self.errors.push((
                    array.span.start,
                    if self.evaluated {
                        self.path.join(" -> ")
                    } else {
                        readable_code(unwrap_syntax_only(expression))
                    },
                    RESPONSIVE_ARRAY,
                ));
            }
            Expression::ArrayExpression(array) => {
                self.responsive_entry = true;
                for (index, item) in array.elements.iter().enumerate() {
                    if let Some(expression) = item.as_expression() {
                        self.path.push(index.to_string());
                        self.responsive(expression, kind);
                        self.path.pop();
                    }
                }
                self.responsive_entry = false;
            }
            _ => self.collect(expression, kind),
        }
    }
}

pub(crate) fn evaluated_errors(json: &str, kind: ObjectKind) -> Vec<(String, &'static str)> {
    let allocator = oxc_allocator::Allocator::default();
    let source = format!("({json})");
    let parsed = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
    match parsed.program.body.first() {
        Some(oxc_ast::ast::Statement::ExpressionStatement(statement)) => {
            let mut declarations = Declarations {
                evaluated: true,
                ..Declarations::default()
            };
            declarations.collect(&statement.expression, kind);
            declarations
                .errors
                .into_iter()
                .map(|(_, path, requirement)| (path, requirement))
                .collect()
        }
        _ => Vec::new(),
    }
}
