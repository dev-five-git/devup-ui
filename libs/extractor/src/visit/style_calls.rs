use super::DevupVisitor;
use super::capture::Captured;
use crate::utils::unwrap_syntax_only;
use oxc_allocator::GetAllocator;
use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKind};
use oxc_span::GetSpan;

#[derive(Clone, Copy)]
enum OrderScope {
    Declarations,
    Global,
    DeclarationRecord,
    GlobalRecord,
}

impl<'a> DevupVisitor<'a> {
    pub(super) fn capture_styled_base(
        &mut self,
        expression: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
    ) {
        let Expression::CallExpression(call) = expression else {
            return;
        };
        let direct = call.arguments.len() == 2;
        let value = match &mut call.callee {
            Expression::Identifier(_) if direct => call.arguments[0].as_expression_mut(),
            Expression::CallExpression(factory) if factory.arguments.len() == 1 => {
                factory.arguments[0].as_expression_mut()
            }
            _ => None,
        };
        if let Some(value) = value
            && self
                .bindings
                .kind(crate::utils::unwrap_syntax_only(value))
                .is_none()
            && super::order::reach(&self.bindings, value) != super::order::Reach::Constant
        {
            let name = self.names.fresh("__devupBase");
            captures.push(self.capture_as(name, value));
        }
    }

    pub(super) fn check_style_orders(&mut self, expression: &Expression<'a>, static_only: bool) {
        self.check_order_scope(
            expression,
            if static_only {
                OrderScope::Global
            } else {
                OrderScope::Declarations
            },
            static_only,
        );
    }

    fn check_order_scope(
        &mut self,
        expression: &Expression<'a>,
        scope: OrderScope,
        static_only: bool,
    ) {
        match unwrap_syntax_only(expression) {
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            let key = property.key.static_name();
                            if matches!(
                                scope,
                                OrderScope::DeclarationRecord | OrderScope::GlobalRecord
                            ) {
                                self.check_order_scope(
                                    &property.value,
                                    if matches!(scope, OrderScope::GlobalRecord) {
                                        OrderScope::Global
                                    } else {
                                        OrderScope::Declarations
                                    },
                                    static_only,
                                );
                                continue;
                            }
                            if static_only
                                && property
                                    .key
                                    .static_name()
                                    .is_some_and(|name| name == "fontFaces")
                            {
                                crate::style_order::reject(
                                    &property.value,
                                    "fontFaces",
                                    &mut self.errors,
                                );
                                continue;
                            }
                            if property
                                .key
                                .static_name()
                                .is_some_and(|name| crate::style_order::reserved(&name))
                            {
                                let parsed = crate::style_order::parse(
                                    &property.value,
                                    self.ast.allocator(),
                                );
                                let error = match parsed {
                                    Err(error) => Some(error),
                                    Ok(_)
                                        if property.kind != PropertyKind::Init
                                            || property.method =>
                                    {
                                        Some((
                                            property.span.start,
                                            crate::utils::build_time_error(
                                                "styleOrder",
                                                "styleOrder",
                                                "a plain value, not an accessor or method",
                                            ),
                                        ))
                                    }
                                    Ok(crate::style_order::Order::Conditional { .. })
                                        if static_only =>
                                    {
                                        Some((
                                            property.value.span().start,
                                            crate::utils::build_time_error(
                                                "styleOrder",
                                                &crate::utils::readable_code(&property.value),
                                                "global styles require a static order; they have no runtime class selection",
                                            ),
                                        ))
                                    }
                                    Ok(_) => None,
                                };
                                if let Some(error) = error {
                                    self.errors.push(error);
                                }
                            } else if let Some(key) = key {
                                let record = matches!(
                                    key.as_ref(),
                                    "selectors"
                                        | "_media"
                                        | "_supports"
                                        | "_container"
                                        | "@media"
                                        | "@supports"
                                        | "@container"
                                        | "@layer"
                                );
                                let data = matches!(
                                    key.as_ref(),
                                    "props" | "styleVars" | "vars" | "imports" | "fontFaces"
                                );
                                if data {
                                    continue;
                                }
                                if record {
                                    self.check_order_scope(
                                        &property.value,
                                        if matches!(scope, OrderScope::Global) && key != "selectors"
                                        {
                                            OrderScope::GlobalRecord
                                        } else {
                                            OrderScope::DeclarationRecord
                                        },
                                        static_only,
                                    );
                                } else if matches!(scope, OrderScope::Global) {
                                    self.check_order_scope(
                                        &property.value,
                                        if key.starts_with('@') || key.starts_with('_') {
                                            OrderScope::Global
                                        } else {
                                            OrderScope::Declarations
                                        },
                                        static_only,
                                    );
                                } else if key.starts_with('_')
                                    || key.starts_with('@')
                                    || key.contains('&')
                                {
                                    self.check_order_scope(
                                        &property.value,
                                        OrderScope::Declarations,
                                        static_only,
                                    );
                                }
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.check_order_scope(&spread.argument, scope, static_only);
                        }
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.check_order_scope(&conditional.consequent, scope, static_only);
                self.check_order_scope(&conditional.alternate, scope, static_only);
            }
            Expression::LogicalExpression(logical) => {
                self.check_order_scope(&logical.right, scope, static_only);
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(value) = element.as_expression() {
                        self.check_order_scope(value, scope, static_only);
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn capture_style_argument(
        &mut self,
        expression: &mut Expression<'a>,
        values: &mut Vec<Captured<'a>>,
    ) {
        crate::css_utils::literal_tree::lower(
            &self.ast,
            expression,
            crate::css_utils::literal_tree::Scope {
                source: self.source,
                global: false,
            },
        );
        self.check_style_orders(expression, false);
        if crate::css_utils::literal::is_rule_text(expression)
            && let Expression::TemplateLiteral(template) = expression
        {
            for value in &mut template.expressions {
                self.capture_shape(value, values);
            }
        } else {
            self.capture_shape(expression, values);
        }
    }
}
