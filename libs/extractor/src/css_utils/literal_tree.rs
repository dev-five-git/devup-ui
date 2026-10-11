use oxc_ast::ast::Expression;
use oxc_ast::builder::AstBuilder;

#[derive(Clone, Copy)]
pub(crate) struct Scope<'s> {
    pub source: Option<&'s str>,
    pub global: bool,
}

pub(crate) fn lower<'a>(
    ast: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    scope: Scope<'_>,
) -> bool {
    let Scope { source, global } = scope;
    let expression = crate::utils::unwrap_syntax_only_mut(expression);
    if !global && super::literal::lower_with_source(ast, expression, source) {
        return true;
    }
    let mut changed = false;
    match expression {
        Expression::ObjectExpression(object) => {
            for property in &mut object.properties {
                match property {
                    oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                        changed |= lower(ast, &mut spread.argument, scope);
                    }
                    oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) => {
                        let Some(key) = property.key.static_name() else {
                            continue;
                        };
                        if matches!(key.as_ref(), "props" | "vars" | "styleVars" | "imports") {
                            continue;
                        }
                        if key == "fontFaces" {
                            changed |= lower(
                                ast,
                                &mut property.value,
                                Scope {
                                    global: false,
                                    ..scope
                                },
                            );
                        } else if matches!(
                            key.as_ref(),
                            "selectors"
                                | "_media"
                                | "_supports"
                                | "_container"
                                | "@media"
                                | "@supports"
                                | "@container"
                                | "@layer"
                        ) {
                            if let Expression::ObjectExpression(record) = &mut property.value {
                                for entry in &mut record.properties {
                                    if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(entry) =
                                        entry
                                    {
                                        changed |= lower(
                                            ast,
                                            &mut entry.value,
                                            Scope {
                                                global: global && key != "selectors",
                                                ..scope
                                            },
                                        );
                                    }
                                }
                            }
                        } else if global
                            || key.starts_with('_')
                            || key.starts_with('@')
                            || key.contains('&')
                        {
                            changed |= lower(
                                ast,
                                &mut property.value,
                                Scope {
                                    global: global && key.starts_with('@'),
                                    ..scope
                                },
                            );
                        }
                    }
                }
            }
        }
        Expression::ArrayExpression(array) => {
            for element in &mut array.elements {
                if let Some(element) = element.as_expression_mut() {
                    changed |= lower(ast, element, scope);
                }
            }
        }
        Expression::ConditionalExpression(conditional) => {
            changed |= lower(ast, &mut conditional.consequent, scope);
            changed |= lower(ast, &mut conditional.alternate, scope);
        }
        Expression::LogicalExpression(logical) => changed |= lower(ast, &mut logical.right, scope),
        _ => {}
    }
    changed
}
