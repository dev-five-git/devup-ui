//! `css()` on React Native: a call compiles to the style it describes, the
//! code an element's `style` takes, and `className` passes that style on.

use oxc_ast::ast::{
    CallExpression, Expression, IdentifierReference, JSXAttribute, LogicalOperator, ObjectExpression,
};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};

use super::{Elements, Styles, attribute_expression};
use crate::native::errors::Problem;
use crate::native::style::{Choice, combined, style_code};

/// The name `css()` errors call the call by
const CSS: &str = "css()";

impl<'a> Elements<'_> {
    /// Whether `callee` reads the `css` the file imports
    pub(super) fn is_css(&self, callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(identifier) => self.is_css_reference(identifier),
            _ => false,
        }
    }

    /// Whether `identifier` reads the `css` the file imports
    pub(super) fn is_css_reference(&self, identifier: &IdentifierReference<'_>) -> bool {
        identifier
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_some_and(|symbol| self.css.contains(&symbol))
    }

    /// Compile a call of `css` to the style it describes
    pub(super) fn css_call(&mut self, call: &CallExpression<'a>) {
        let mut choices = Vec::new();
        for argument in &call.arguments {
            match argument.as_expression() {
                Some(expression) => self.css_part(expression, &mut choices),
                None => self.error(argument.span().start, spread_problem()),
            }
        }
        if let Some((offset, name)) = self.press.take() {
            self.error(
                offset,
                Problem::new(
                    "a press style applies while an element is pressed, which `css()` cannot tell",
                    format!("give `{name}` to the element"),
                )
                .on(CSS, &name),
            );
        }
        let code = style_code(
            choices,
            (Vec::new(), Vec::new()),
            &mut self.sheet,
            &self.runtime.vars(),
        )
            .map_or_else(
                || "null".to_string(),
                |style| {
                    if style.reads.any() && !self.read_in_component(style.reads) {
                        self.error(call.span.start, outside_component_problem());
                    }
                    style.code
                },
            );
        self.replacements
            .push((call.span.start as usize, call.span.end as usize, code));
    }

    /// One argument of `css()`: an object of styles, a condition choosing
    /// objects, or a style it passes on whole, such as what another `css()`
    /// gives
    fn css_part(&mut self, expression: &Expression<'a>, choices: &mut Vec<Choice>) {
        match expression.without_parentheses() {
            Expression::ObjectExpression(object) => {
                let object_choices = self.object_choices(object, CSS);
                choices.extend(object_choices);
            }
            Expression::ConditionalExpression(condition)
                if is_object(&condition.consequent) || is_object(&condition.alternate) =>
            {
                let consequent = self.css_branch(&condition.consequent);
                let alternate = self.css_branch(&condition.alternate);
                choices.push(Choice::Condition {
                    test: self.rewritten(&condition.test),
                    consequent: Box::new(consequent),
                    alternate: Box::new(alternate),
                });
            }
            Expression::LogicalExpression(logical)
                if logical.operator == LogicalOperator::And && is_object(&logical.right) =>
            {
                let consequent = self.css_branch(&logical.right);
                choices.push(Choice::Condition {
                    test: self.rewritten(&logical.left),
                    consequent: Box::new(consequent),
                    alternate: Box::new(Choice::Empty),
                });
            }
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => {
                self.error(expression.span().start, class_problem(CSS, "a part"));
            }
            expression if is_nothing(expression) => {}
            expression => {
                let code = self.rewritten(expression);
                choices.push(Choice::Given(code));
            }
        }
    }

    /// The styles of a branch of a condition in `css()`: an object, or none
    fn css_branch(&mut self, expression: &Expression<'a>) -> Choice {
        match expression.without_parentheses() {
            Expression::ObjectExpression(object) => self.css_object(object),
            expression if is_nothing(expression) => Choice::Empty,
            expression => {
                self.error(
                    expression.span().start,
                    Problem::new(
                        "a condition in `css()` chooses between objects of styles",
                        "write each branch as an object, or as `null`",
                    )
                    .on(CSS, "?"),
                );
                Choice::Empty
            }
        }
    }

    fn css_object(&mut self, object: &ObjectExpression<'a>) -> Choice {
        let choices = self.object_choices(object, CSS);
        combined(choices)
    }

    /// `className` on an element: the style a `css()` gives, before the
    /// element's style props
    pub(super) fn class_name(
        &mut self,
        attribute: &JSXAttribute<'a>,
        span: Span,
        local: &str,
        styles: &mut Styles,
    ) {
        match attribute_expression(attribute).map(Expression::without_parentheses) {
            Some(Expression::StringLiteral(_) | Expression::TemplateLiteral(_)) | None => {
                self.error(attribute.span.start, class_problem(local, "className"));
            }
            Some(expression) => {
                let code = self.rewritten(expression);
                styles.classes.push(code);
                styles.removed.push(span);
            }
        }
    }

    /// The code of `expression` with what the build compiles in it compiled,
    /// such as a `css()` call or a Devup UI element
    pub(super) fn rewritten(&mut self, expression: &Expression<'a>) -> String {
        let span = expression.span();
        let mark = self.replacements.len();
        self.visit_expression(expression);
        let (start, end) = (span.start as usize, span.end as usize);
        let mut inner = Vec::new();
        let mut index = mark;
        while index < self.replacements.len() {
            let (from, to, _) = self.replacements[index];
            if start <= from && to <= end {
                inner.push(self.replacements.remove(index));
            } else {
                index += 1;
            }
        }
        inner.sort_by_key(|(from, to, _)| (*from, *to));
        let mut code = String::new();
        let mut at = start;
        for (from, to, text) in inner {
            code.push_str(&self.code[at..from]);
            code.push_str(&text);
            at = to;
        }
        code.push_str(&self.code[at..end]);
        if matches!(
            expression.without_parentheses(),
            Expression::SequenceExpression(_)
        ) {
            format!("({code})")
        } else {
            code
        }
    }
}

/// Whether `expression` is an object of styles
fn is_object(expression: &Expression<'_>) -> bool {
    matches!(expression.without_parentheses(), Expression::ObjectExpression(_))
}

/// Whether `expression` gives no style: `null`, `undefined` or `false`
fn is_nothing(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::NullLiteral(_) => true,
        Expression::BooleanLiteral(literal) => !literal.value,
        Expression::Identifier(identifier) => identifier.name == "undefined",
        _ => false,
    }
}

fn spread_problem() -> String {
    Problem::new(
        "the build reads only the parts written in the call",
        "pass each part to `css()` itself",
    )
    .call(CSS)
}

fn outside_component_problem() -> String {
    Problem::new(
        "its styles differ by theme, breakpoint or orientation, which only a function component or a hook can read",
        "call `css()` in the component that uses it",
    )
    .call(CSS)
}

/// The error for a class name, which React Native has none of
fn class_problem(subject: &str, prop: &str) -> String {
    Problem::new(
        "React Native has no class names",
        "pass what `css()` gives, or write the styles as props",
    )
    .on(subject, prop)
}
