use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind};
use oxc_span::GetSpan;

#[derive(Clone, Copy)]
enum Scope {
    Rules,
    Record,
    Declarations,
    Values,
}

struct Rejection<'a> {
    api: &'a str,
    errors: &'a mut Vec<(u32, String)>,
}

pub(super) fn reject(expression: &Expression<'_>, api: &str, errors: &mut Vec<(u32, String)>) {
    Rejection { api, errors }.walk(
        expression,
        if api == "fontFaces" {
            Scope::Declarations
        } else {
            Scope::Rules
        },
    );
}

pub(super) fn reject_object(
    object: &ObjectExpression<'_>,
    api: &str,
    errors: &mut Vec<(u32, String)>,
) {
    Rejection { api, errors }.object(object, Scope::Rules);
}

impl Rejection<'_> {
    fn walk(&mut self, expression: &Expression<'_>, scope: Scope) {
        match crate::utils::unwrap_syntax_only(expression) {
            Expression::ObjectExpression(object) => self.object(object, scope),
            Expression::ArrayExpression(array) => {
                for value in array
                    .elements
                    .iter()
                    .filter_map(|element| element.as_expression())
                {
                    self.walk(value, scope);
                }
            }
            Expression::ConditionalExpression(value) => {
                self.walk(&value.consequent, scope);
                self.walk(&value.alternate, scope);
            }
            Expression::LogicalExpression(value) => self.walk(&value.right, scope),
            Expression::ArrowFunctionExpression(arrow) => {
                if let Some(value) = arrow.body.as_expression() {
                    self.walk(value, scope);
                }
            }
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
                if !self.api.starts_with("stylex") && !matches!(scope, Scope::Values) =>
            {
                let allocator = Allocator::default();
                let ast = oxc_ast::builder::AstBuilder::new(&allocator);
                if let Some(text) = crate::css_utils::literal::CssText::new(&ast, expression) {
                    self.errors.extend(crate::css_utils::global::reject(
                        &text,
                        0..text.text.len(),
                        self.api,
                    ));
                }
            }
            _ => {}
        }
    }

    fn object(&mut self, object: &ObjectExpression<'_>, scope: Scope) {
        for property in &object.properties {
            match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    let key = property.key.static_name();
                    if !matches!(scope, Scope::Record)
                        && key.as_ref().is_some_and(|key| super::reserved(key))
                    {
                        self.errors
                            .push(super::no_effect(self.api, property.key.span().start));
                        continue;
                    }
                    let nested = key.as_ref().is_some_and(|key| {
                        key.starts_with('_') || key.starts_with('@') || key.contains('&')
                    });
                    let record = key.as_ref().is_some_and(|key| {
                        matches!(
                            key.as_ref(),
                            "selectors"
                                | "_media"
                                | "_supports"
                                | "_container"
                                | "@media"
                                | "@supports"
                                | "@container"
                                | "@layer"
                        )
                    });
                    let child = match scope {
                        Scope::Rules | Scope::Record => Scope::Declarations,
                        Scope::Declarations if record => Scope::Record,
                        Scope::Declarations if nested => Scope::Declarations,
                        Scope::Declarations | Scope::Values => Scope::Values,
                    };
                    self.walk(&property.value, child);
                }
                ObjectPropertyKind::SpreadProperty(spread) => self.walk(&spread.argument, scope),
            }
        }
    }
}
