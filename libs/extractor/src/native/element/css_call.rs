//! `css()` on React Native: a call compiles to the style it describes, the
//! code an element's `style` takes, and `className` passes that style on.

use oxc_ast::ast::{
    CallExpression, Expression, IdentifierReference, JSXAttribute, LogicalOperator,
    ObjectExpression, TaggedTemplateExpression, TemplateLiteral,
};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};
use std::fmt::Write;

use super::styled::{gives_style_props, is_function};
use super::{Elements, Prop, Styles, attribute_expression, classify};
use crate::native::errors::Problem;
use crate::native::style::{Choice, State, combined, style_code};
use crate::native::values::Literal;

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
        let compat = self.is_compat(&call.callee);
        let outer = std::mem::replace(&mut self.compat, compat);
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
                    "a press, hover or focus style applies while the element is in that state, which `css()` cannot tell",
                    format!("give `{name}` to the element"),
                )
                .on(CSS, &name),
            );
        }
        if let Some(animation) = self.animation.take() {
            // The element given the style animates when it is animated
            self.reanimated(&animation, CSS);
        }
        self.css_code(call.span, choices);
        self.compat = outer;
    }

    /// Compile `css` tagged with CSS text, such as `` css`padding: 8px;` ``,
    /// to the style it describes
    pub(super) fn css_template(&mut self, template: &TaggedTemplateExpression<'a>) {
        let compat = self.is_compat(&template.tag);
        let outer = std::mem::replace(&mut self.compat, compat);
        let choices = self.css_text(&template.quasi, template.span.start);
        self.compat = outer;
        self.css_code(template.span, choices);
    }

    /// Put the style `choices` describe in place of the `css` at `span`
    fn css_code(&mut self, span: Span, choices: Vec<Choice>) {
        if choices.iter().any(|choice| choice.holds(State::Disabled)) {
            self.error(
                span.start,
                Problem::new(
                    "`_disabled` applies while the element is disabled, which `css()` cannot tell",
                    "give `_disabled` to the element",
                )
                .call(CSS),
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
                    self.error(span.start, outside_component_problem());
                }
                style.code
            },
        );
        self.replacements
            .push((span.start as usize, span.end as usize, code));
    }

    /// The styles CSS text declares, each value written in it or given whole
    /// by an interpolation
    pub(super) fn css_text(&mut self, template: &TemplateLiteral<'a>, offset: u32) -> Vec<Choice> {
        let text = marked_text(template);
        if text.contains(['{', '}']) {
            self.error(
                offset,
                Problem::new(
                    "CSS text on React Native holds only declarations: React Native has no selectors, nesting or at-rules",
                    "write the declarations alone, and the styles of other states as props such as `_active`",
                )
                .call(CSS),
            );
            return Vec::new();
        }
        let mut choices = Vec::new();
        for declaration in text
            .split(';')
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            if let Some((name, _)) = declaration.split_once(':') {
                self.note_animation(&camel_case(name.trim()), offset);
            }
            match self.css_declaration(declaration, &template.expressions) {
                Ok(choice) => choices.push(choice),
                Err((name, problem)) => self.error(offset, problem.on(CSS, &name)),
            }
        }
        choices
    }

    /// The styles one declaration of CSS text declares
    fn css_declaration(
        &self,
        declaration: &str,
        expressions: &[Expression<'a>],
    ) -> Result<Choice, (String, Problem)> {
        let Some((name, value)) = declaration.split_once(':') else {
            return Err((
                unmarked(declaration),
                Problem::new(
                    "a declaration of CSS text is a property and a value",
                    "write it as `property: value`",
                ),
            ));
        };
        let name = camel_case(name.trim());
        let value = value.trim().trim_end_matches("!important").trim_end();
        if classify(&name) != Prop::Declares {
            return Err((
                name,
                Problem::new(
                    "React Native has no such style",
                    "remove it, or write a style React Native has",
                ),
            ));
        }
        let result = match marker_index(value) {
            Some(index) => self.expression_choice(&name, &expressions[index]),
            None if value.contains(MARKER) => Err(Problem::new(
                "the build reads an interpolation in CSS text only as a whole value",
                "compute the whole value in the interpolation, such as `${`${size}px`}`",
            )),
            None => self.static_choice(&name, &[(0, Literal::Text(value.to_string()))]),
        };
        result.map_err(|problem| (name, problem))
    }

    /// One argument of `css()`: an object of styles, a condition choosing
    /// objects, or a style it passes on whole, such as what another `css()`
    /// gives
    pub(super) fn css_part(&mut self, expression: &Expression<'a>, choices: &mut Vec<Choice>) {
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
            expression if self.in_styled && is_function(expression) => {
                // A compat package's function gives React Native styles
                if !self.compat && gives_style_props(expression) {
                    self.error(
                        expression.span().start,
                        Problem::new(
                            "a function of the props gives style props, which React Native cannot read at runtime",
                            "return what `css()` gives, or write each value as a function of the props",
                        )
                        .call("styled()"),
                    );
                    return;
                }
                let code = format!("({})({})", self.rewritten(expression), self.props);
                self.props_called.set(true);
                choices.push(Choice::Given(code));
            }
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
        // What visiting the expression changes lies in it
        let mut inner = self.replacements.split_off(mark);
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

/// What marks an interpolation in CSS text, around its index
const MARKER: char = char::MIN;

/// The text of `template`, each interpolation marked by its index
fn marked_text(template: &TemplateLiteral<'_>) -> String {
    let mut text = String::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        text.push_str(quasi.value.raw.as_str());
        if index < template.expressions.len() {
            let _ = write!(text, "{MARKER}{index}{MARKER}");
        }
    }
    strip_comments(&text)
}

/// `text` without its CSS comments
fn strip_comments(text: &str) -> String {
    let mut rest = text;
    let mut kept = String::new();
    while let Some(start) = rest.find("/*") {
        kept.push_str(&rest[..start]);
        rest = rest[start + 2..]
            .find("*/")
            .map_or("", |end| &rest[start + 2 + end + 2..]);
    }
    kept.push_str(rest);
    kept
}

/// `text` as it is written, each interpolation as `${…}`
fn unmarked(text: &str) -> String {
    text.split(MARKER)
        .enumerate()
        .map(|(at, part)| if at % 2 == 1 { "${…}" } else { part })
        .collect()
}

/// The index of the interpolation `value` is, when it is one whole
fn marker_index(value: &str) -> Option<usize> {
    value
        .strip_prefix(MARKER)?
        .strip_suffix(MARKER)?
        .parse()
        .ok()
}

/// A CSS property name as React Native names its style: `background-color`
/// as `backgroundColor`
fn camel_case(name: &str) -> String {
    let mut camel = String::with_capacity(name.len());
    let mut upper = false;
    for character in name.chars() {
        if character == '-' {
            upper = !camel.is_empty();
        } else if upper {
            camel.extend(character.to_uppercase());
            upper = false;
        } else {
            camel.push(character);
        }
    }
    camel
}

/// Whether `expression` is an object of styles
fn is_object(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::ObjectExpression(_)
    )
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
        "its styles differ by theme, breakpoint, orientation, motion or the window's size, which only a function component or a hook can read",
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
