//! Emotion's `cx` and `merge`: class names in, one class string out.
//!
//! Both are rewritten into a single array of class parts, which the `css()`
//! composition then joins like any other classes. A string is always a class
//! and an object is always a class map, never rules.

use crate::gen_class_name::merge_expression_for_class_name;
use crate::utils::{
    build_time_error, get_str_by_property_key, get_string_by_literal_expression, readable_argument,
    spread_error, unwrap_syntax_only,
};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, Expression, ObjectExpression, ObjectPropertyKind,
    PropertyKind, Str, TemplateElement, TemplateElementValue, TemplateLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use oxc_syntax::operator::LogicalOperator;

/// Which Emotion function a call is
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassCall {
    Cx,
    Merge,
}

impl ClassCall {
    const fn api(self) -> &'static str {
        match self {
            ClassCall::Cx => "cx",
            ClassCall::Merge => "merge",
        }
    }
}

/// Whether a literal is truthy, `None` for what only the running code knows
fn literal_truth(expression: &Expression<'_>) -> Option<bool> {
    match unwrap_syntax_only(expression) {
        Expression::BooleanLiteral(literal) => Some(literal.value),
        Expression::NullLiteral(_) => Some(false),
        Expression::NumericLiteral(number) => Some(number.value != 0.0),
        Expression::StringLiteral(text) => Some(!text.value.is_empty()),
        Expression::Identifier(name) if name.name == "undefined" => Some(false),
        _ => None,
    }
}

const MAP_ENTRY: &str =
    "a class map entry must be written `name: condition`, not as a getter, setter or method";
const MERGE_ARGUMENT: &str = "it takes one class string";

/// The parts of `arguments` as one array expression of classes, each a string,
/// a template literal or `test ? class : class`; what the build cannot read is
/// recorded in `errors`
pub(super) fn class_arguments<'a>(
    ast: &AstBuilder<'a>,
    call: ClassCall,
    offset: u32,
    arguments: &[Argument<'a>],
    errors: &mut Vec<(u32, String)>,
) -> Expression<'a> {
    let mut reader = ClassParts { ast, call, errors };
    if call == ClassCall::Merge && arguments.len() != 1 {
        let code: Vec<String> = arguments.iter().map(readable_argument).collect();
        reader.errors.push((
            offset,
            build_time_error(call.api(), &code.join(", "), MERGE_ARGUMENT),
        ));
    }
    let mut parts = Vec::new();
    for argument in arguments {
        match argument {
            Argument::SpreadElement(spread) => reader.errors.push(spread_error(call.api(), spread)),
            argument => reader.add(argument.to_expression(), &mut parts),
        }
    }
    Expression::new_array_expression(
        SPAN,
        oxc_allocator::Vec::from_iter_in(parts.into_iter().map(ArrayExpressionElement::from), ast),
        ast,
    )
}

struct ClassParts<'r, 'a> {
    ast: &'r AstBuilder<'a>,
    call: ClassCall,

    errors: &'r mut Vec<(u32, String)>,
}

impl<'a> ClassParts<'_, 'a> {
    fn string(&self, value: &str) -> Expression<'a> {
        Expression::new_string_literal(
            SPAN,
            Str::from_in(value, self.ast.allocator()),
            None,
            self.ast,
        )
    }

    fn clone(&self, expression: &Expression<'a>) -> Expression<'a> {
        expression.clone_in(self.ast.allocator())
    }

    /// `${value}`
    fn interpolation(&self, value: Expression<'a>) -> Expression<'a> {
        let element = |tail| {
            TemplateElement::new(
                SPAN,
                TemplateElementValue {
                    raw: Str::from_in("", self.ast.allocator()),
                    cooked: None,
                },
                tail,
                self.ast,
            )
        };
        Expression::new_template_literal(
            SPAN,
            oxc_allocator::Vec::from_array_in([element(false), element(true)], self.ast),
            oxc_allocator::Vec::from_array_in([value], self.ast),
            self.ast,
        )
    }

    /// `branch` as a single class: what `test ? class : class` may hold
    fn class(&mut self, expression: &Expression<'a>) -> Expression<'a> {
        let mut parts = Vec::new();
        self.add(expression, &mut parts);
        match merge_expression_for_class_name(self.ast, parts) {
            None => self.string(""),
            Some(class @ (Expression::StringLiteral(_) | Expression::TemplateLiteral(_))) => class,
            Some(other) => self.interpolation(other),
        }
    }

    /// `test ? when_true : when_false`, or the side a literal test picks
    fn choose(
        &self,
        test: &Expression<'a>,
        when_true: Expression<'a>,
        when_false: Expression<'a>,
        out: &mut Vec<Expression<'a>>,
    ) {
        out.push(match literal_truth(test) {
            Some(true) => when_true,
            Some(false) => when_false,
            None => Expression::new_conditional_expression(
                SPAN,
                self.clone(test),
                when_true,
                when_false,
                self.ast,
            ),
        });
    }

    /// A value only the running code knows, as a class
    fn runtime(&self, value: Expression<'a>, out: &mut Vec<Expression<'a>>) {
        out.push(if self.call == ClassCall::Cx {
            self.without_falsy(value)
        } else {
            self.as_written(value)
        });
    }

    /// `${value || ''}`: a falsy value is no class
    fn without_falsy(&self, value: Expression<'a>) -> Expression<'a> {
        let none = self.string("");
        let skipped =
            Expression::new_logical_expression(SPAN, value, LogicalOperator::Or, none, self.ast);
        self.interpolation(skipped)
    }

    /// `value` itself where it reads as a class, `${value}` otherwise
    fn as_written(&self, value: Expression<'a>) -> Expression<'a> {
        if matches!(
            value,
            Expression::Identifier(_)
                | Expression::StaticMemberExpression(_)
                | Expression::ComputedMemberExpression(_)
        ) {
            value
        } else {
            self.interpolation(value)
        }
    }

    fn text(&self, text: &str, out: &mut Vec<Expression<'a>>) {
        out.extend(text.split_whitespace().map(|class| self.string(class)));
    }

    fn add(&mut self, expression: &Expression<'a>, out: &mut Vec<Expression<'a>>) {
        match unwrap_syntax_only(expression) {
            Expression::StringLiteral(text) => self.text(&text.value, out),
            Expression::TemplateLiteral(template) => self.template(template, out),
            Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => {}
            Expression::Identifier(name) if name.name == "undefined" => {}
            Expression::NumericLiteral(number) => {
                if number.value != 0.0
                    && let Some(text) = get_string_by_literal_expression(expression)
                {
                    out.push(self.string(&text));
                }
            }
            Expression::ArrayExpression(array) if self.call == ClassCall::Cx => {
                for element in &array.elements {
                    match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            self.errors.push(spread_error(self.call.api(), spread));
                        }
                        element => {
                            if let Some(element) = element.as_expression() {
                                self.add(element, out);
                            }
                        }
                    }
                }
            }
            Expression::ObjectExpression(map) if self.call == ClassCall::Cx => {
                self.class_map(map, out);
            }
            Expression::ConditionalExpression(conditional) => {
                let when_true = self.class(&conditional.consequent);
                let when_false = self.class(&conditional.alternate);
                self.choose(&conditional.test, when_true, when_false, out);
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                let class = self.class(&logical.right);
                let none = self.string("");
                self.choose(&logical.left, class, none, out);
            }
            Expression::LogicalExpression(logical) => {
                let fallback = self.class(&logical.right);
                let chosen = Expression::new_logical_expression(
                    SPAN,
                    self.clone(&logical.left),
                    logical.operator,
                    fallback,
                    self.ast,
                );
                self.runtime(chosen, out);
            }
            other => {
                let value = self.clone(other);
                self.runtime(value, out);
            }
        }
    }

    /// A template literal split into its classes when every interpolation is
    /// set apart by spaces, and kept whole when one is glued to a class
    fn template(&mut self, template: &TemplateLiteral<'a>, out: &mut Vec<Expression<'a>>) {
        let texts: Vec<String> = template
            .quasis
            .iter()
            .map(|quasi| {
                quasi
                    .value
                    .cooked
                    .as_ref()
                    .unwrap_or(&quasi.value.raw)
                    .to_string()
            })
            .collect();
        let glued = texts.windows(2).any(|pair| {
            pair[0]
                .chars()
                .next_back()
                .is_some_and(|c| !c.is_whitespace())
                || pair[1].chars().next().is_some_and(|c| !c.is_whitespace())
        });
        if glued {
            out.push(Expression::TemplateLiteral(oxc_allocator::Box::new_in(
                template.clone_in(self.ast.allocator()),
                self.ast,
            )));
            return;
        }
        for (index, text) in texts.iter().enumerate() {
            self.text(text, out);
            if let Some(expression) = template.expressions.get(index) {
                self.add(expression, out);
            }
        }
    }

    /// `{ name: condition }`: `name` while `condition` holds
    fn class_map(&mut self, map: &ObjectExpression<'a>, out: &mut Vec<Expression<'a>>) {
        for property in &map.properties {
            let property = match property {
                ObjectPropertyKind::ObjectProperty(property) => property,
                ObjectPropertyKind::SpreadProperty(spread) => {
                    self.errors.push(spread_error(self.call.api(), spread));
                    continue;
                }
            };
            if property.kind != PropertyKind::Init || property.method {
                let accessor = match property.kind {
                    PropertyKind::Get => "get ",
                    PropertyKind::Set => "set ",
                    PropertyKind::Init => "",
                };
                let name = get_str_by_property_key(&property.key).unwrap_or_default();
                self.errors.push((
                    property.span.start,
                    build_time_error(self.call.api(), &format!("{accessor}{name}()"), MAP_ENTRY),
                ));
                continue;
            }
            let name = get_str_by_property_key(&property.key)
                .map(|name| self.string(&name))
                .or_else(|| {
                    property
                        .key
                        .as_expression()
                        .map(|key| self.interpolation(self.clone(key)))
                });
            if let Some(name) = name {
                let none = self.string("");
                self.choose(&property.value, name, none, out);
            }
        }
    }
}

/// A class string whose interpolations hold templates, those spliced into it
pub(super) fn flatten_classes<'a>(
    ast: &AstBuilder<'a>,
    expression: Expression<'a>,
) -> Expression<'a> {
    let Expression::TemplateLiteral(template) = &expression else {
        return expression;
    };
    let mut flat = Flat::default();
    flat.splice(ast, template);
    flat.texts.push(std::mem::take(&mut flat.current));
    let last = flat.texts.len();
    let elements = flat.texts.iter().enumerate().map(|(index, text)| {
        TemplateElement::new(
            SPAN,
            TemplateElementValue {
                raw: Str::from_in(text.as_str(), ast.allocator()),
                cooked: None,
            },
            index + 1 == last,
            ast,
        )
    });
    Expression::new_template_literal(
        SPAN,
        oxc_allocator::Vec::from_iter_in(elements, ast),
        oxc_allocator::Vec::from_iter_in(flat.expressions, ast),
        ast,
    )
}

#[derive(Default)]
struct Flat<'a> {
    current: String,
    texts: Vec<String>,
    expressions: Vec<Expression<'a>>,
}

impl<'a> Flat<'a> {
    fn splice(&mut self, ast: &AstBuilder<'a>, template: &TemplateLiteral<'a>) {
        for (index, quasi) in template.quasis.iter().enumerate() {
            self.current.push_str(&quasi.value.raw);
            match template.expressions.get(index) {
                Some(Expression::TemplateLiteral(inner)) => self.splice(ast, inner),
                Some(other) => {
                    self.texts.push(std::mem::take(&mut self.current));
                    self.expressions.push(other.clone_in(ast.allocator()));
                }
                None => {}
            }
        }
    }
}
