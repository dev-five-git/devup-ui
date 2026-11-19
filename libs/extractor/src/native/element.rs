//! Devup UI elements as React Native elements: the style props of each
//! element leave it for one `style`, and the element takes the React Native
//! component its import is now bound to.

use oxc_ast::ast::{
    Expression, IdentifierReference, JSXAttribute, JSXAttributeItem, JSXAttributeName,
    JSXAttributeValue, JSXChild, JSXElement, JSXElementName, JSXExpression, LogicalOperator,
    ObjectExpression, ObjectPropertyKind, Program, TSType, UnaryOperator,
};
use std::cell::Cell;

use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::errors::{Problem, RuntimeFeature};
use super::imports::{self, native_component};
use super::props::{declarations, dynamic_keys};
use super::sheet::Sheet;
use super::style::{Choice, style_code};
use super::values::{Literal, NativeValue, ROOT_FONT_SIZE};
use super::{NativePlatform, NativeTarget, REACT_NATIVE, Replacement};
use crate::component::ExportVariableKind;
use crate::fresh_name::fresh_name;

/// The changes compiling a file makes, or why it cannot compile
pub(super) struct Compiled {
    pub replacements: Vec<Replacement>,
    pub errors: Vec<(u32, String)>,
}

/// Compile the Devup UI elements of `program`
pub(super) fn compile(
    program: &Program<'_>,
    code: &str,
    package: &str,
    target: NativeTarget,
    scoping: &Scoping,
) -> Compiled {
    let imports = imports::read(program, code, package, target);
    let mut visitor = Elements {
        code,
        target,
        scoping,
        components: &imports.components,
        sheet: Sheet::new(fresh_name("__s", code)),
        units: fresh_name("__u", code),
        units_used: Cell::new(false),
        replacements: imports.replacements,
        errors: imports.errors,
    };
    if !visitor.components.is_empty() {
        visitor.visit_program(program);
    }
    let Elements {
        sheet,
        units,
        units_used,
        mut replacements,
        errors,
        ..
    } = visitor;
    let mut prelude = String::new();
    if !sheet.is_empty() {
        prelude.push_str(&sheet_prelude(&sheet, code));
    }
    if units_used.get() {
        prelude.push_str(&units_declaration(&units));
    }
    if !prelude.is_empty()
        && let Some(at) = imports.sheet_at
    {
        replacements.push((at, at, prelude));
    }
    Compiled {
        replacements,
        errors,
    }
}

/// What a prop of a Devup UI element is
#[derive(Debug, PartialEq)]
enum Prop {
    /// `as`, the component rendered instead
    As,
    /// `style`, given whole
    Style,
    /// A style prop
    Declares,
    /// `_ios`, `_android` or `_web`: styles for one platform
    Platform(NativePlatform),
    /// What needs the React Native runtime
    Runtime(RuntimeFeature),
    /// What React Native has no equivalent of
    Unsupported(Problem),
    /// A prop the element passes on to the React Native component
    Pass,
}

/// What the prop `name` of a Devup UI element is
fn classify(name: &str) -> Prop {
    match name {
        "as" => Prop::As,
        "style" => Prop::Style,
        "_ios" => Prop::Platform(NativePlatform::Ios),
        "_android" => Prop::Platform(NativePlatform::Android),
        "_web" => Prop::Platform(NativePlatform::Web),
        "_pressed" | "_active" => Prop::Runtime(RuntimeFeature::Pressed),
        "_portrait" | "_landscape" => Prop::Runtime(RuntimeFeature::Responsive),
        "className" => Prop::Unsupported(Problem::new(
            "React Native has no class names",
            "write the styles as props, or pass them in `style`",
        )),
        "typography" => Prop::Unsupported(Problem::new(
            "theme typography is not read on React Native yet",
            "write `fontSize`, `fontWeight`, `lineHeight` and `letterSpacing` themselves",
        )),
        "styleOrder" | "styleVars" => Prop::Unsupported(Problem::new(
            "React Native has no stylesheet order and no CSS variables",
            "remove it; on React Native the later style in `style` wins",
        )),
        name if name.starts_with("_theme") => Prop::Runtime(RuntimeFeature::Theme),
        name if name.starts_with('_') || name == "selectors" => Prop::Unsupported(Problem::new(
            "React Native has no selectors, pseudo-elements or media queries",
            "remove it, or write the style the element takes in every state",
        )),
        name if css::is_style_property(name) => Prop::Declares,
        _ => Prop::Pass,
    }
}

/// The styles a component declares before its props
fn defaults(kind: &ExportVariableKind) -> Vec<Choice> {
    let row = || {
        (
            "flexDirection".to_string(),
            NativeValue::Text("row".to_string()),
        )
    };
    let center = |key: &str| (key.to_string(), NativeValue::Text("center".to_string()));
    match kind {
        ExportVariableKind::Flex => vec![Choice::Static(vec![row()])],
        ExportVariableKind::Center => vec![Choice::Static(vec![
            row(),
            center("justifyContent"),
            center("alignItems"),
        ])],
        _ => vec![],
    }
}

/// Whether a React Native component renders no text of its own
fn holds_no_text(native: &str) -> bool {
    matches!(native, "View" | "Pressable")
}

struct Elements<'s> {
    code: &'s str,
    target: NativeTarget,
    scoping: &'s Scoping,
    components: &'s FxHashMap<SymbolId, (ExportVariableKind, String)>,
    sheet: Sheet,
    /// The name of the function converting values read at runtime
    units: String,
    /// Whether a value read at runtime is converted
    units_used: Cell<bool>,
    replacements: Vec<Replacement>,
    errors: Vec<(u32, String)>,
}

/// What the props of one element give its `style`
#[derive(Default)]
struct Styles {
    choices: Vec<Choice>,
    /// The code of each style given whole, in prop order
    given: Vec<String>,
    /// The span of each prop leaving the element once it takes `style`
    removed: Vec<Span>,
    /// The `style` prop, which leaves the element with the style props
    style: Option<Span>,
    /// A spread whose `style` cannot be read twice
    opaque_spread: Option<Span>,
}

impl<'a> Visit<'a> for Elements<'_> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'a>) {
        match self.component(&element.opening_element.name) {
            Some((kind, local)) => self.element(element, &kind, &local),
            None => walk::walk_jsx_element(self, element),
        }
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if let Some((_, local)) = self.binding(identifier) {
            self.errors.push((
                identifier.span.start,
                format!(
                    "`{local}` can only be rendered as a JSX element on React Native: the build compiles its props into styles only there. Fix: render `<{local} />`, or use a React Native component here"
                ),
            ));
        }
    }

    fn visit_ts_type(&mut self, _: &TSType<'a>) {}
}

impl<'a> Elements<'_> {
    /// The Devup UI component `identifier` reads
    fn binding(
        &self,
        identifier: &IdentifierReference<'_>,
    ) -> Option<(ExportVariableKind, String)> {
        let reference = identifier.reference_id.get()?;
        let symbol = self.scoping.get_reference(reference).symbol_id()?;
        self.components.get(&symbol).cloned()
    }

    /// The Devup UI component an element renders
    fn component(&self, name: &JSXElementName<'_>) -> Option<(ExportVariableKind, String)> {
        match name {
            JSXElementName::IdentifierReference(identifier) => self.binding(identifier),
            _ => None,
        }
    }

    fn error(&mut self, offset: u32, message: String) {
        self.errors.push((offset, message));
    }

    /// Compile one Devup UI element
    fn element(&mut self, element: &JSXElement<'a>, kind: &ExportVariableKind, local: &str) {
        let opening = &element.opening_element;
        let mut styles = Styles {
            choices: defaults(kind),
            ..Styles::default()
        };
        let mut tag = None;
        let mut previous_end = opening
            .type_arguments
            .as_ref()
            .map_or_else(|| opening.name.span().end, |arguments| arguments.span.end);
        for item in &opening.attributes {
            let span = Span::new(previous_end, item.span().end);
            previous_end = item.span().end;
            match item {
                JSXAttributeItem::SpreadAttribute(spread) => {
                    match spread_style(&spread.argument, self.code) {
                        Some(style) => styles.given.push(style),
                        None => styles.opaque_spread = Some(spread.span),
                    }
                    self.visit_expression(&spread.argument);
                }
                JSXAttributeItem::Attribute(attribute) => {
                    if let Some(name) = self.attribute(attribute, span, kind, local, &mut styles) {
                        tag = Some(name);
                    }
                }
            }
        }
        let retagged = tag.is_some();
        self.finish(element, styles, tag, previous_end);
        self.children(element, kind, local, retagged);
    }

    /// Read one prop; the code of the component it renders instead when it
    /// is `as`
    fn attribute(
        &mut self,
        attribute: &JSXAttribute<'a>,
        span: Span,
        kind: &ExportVariableKind,
        local: &str,
        styles: &mut Styles,
    ) -> Option<String> {
        let JSXAttributeName::Identifier(name) = &attribute.name else {
            self.pass(attribute);
            return None;
        };
        let name = name.name.as_str();
        if name == "src" && *kind == ExportVariableKind::Image {
            self.image_source(attribute, local);
            return None;
        }
        match classify(name) {
            Prop::As => {
                styles.removed.push(span);
                return self.tag(attribute, local);
            }
            Prop::Style => self.given_style(attribute, span, local, styles),
            Prop::Declares => {
                styles.removed.push(span);
                match self.value_choice(name, attribute.value.as_ref()) {
                    Ok(choice) => styles.choices.push(choice),
                    Err(problem) => self.error(attribute.span.start, problem.on(local, name)),
                }
            }
            Prop::Platform(platform) => {
                styles.removed.push(span);
                let choices = self.platform(
                    attribute_expression(attribute),
                    name,
                    attribute.span.start,
                    local,
                );
                if platform == self.target.platform {
                    styles.choices.extend(choices);
                }
            }
            Prop::Runtime(feature) => self.error(
                attribute.span.start,
                feature.problem(self.target).on(local, name),
            ),
            Prop::Unsupported(problem) => self.error(attribute.span.start, problem.on(local, name)),
            Prop::Pass => self.pass(attribute),
        }
        None
    }

    /// Visit a prop the element passes on, for the elements in its value
    fn pass(&mut self, attribute: &JSXAttribute<'a>) {
        if let Some(value) = &attribute.value {
            self.visit_jsx_attribute_value(value);
        }
    }

    /// `style`, applied after the style props as an inline style is on the web
    fn given_style(
        &mut self,
        attribute: &JSXAttribute<'a>,
        span: Span,
        local: &str,
        styles: &mut Styles,
    ) {
        match attribute_expression(attribute) {
            Some(expression) => {
                styles
                    .given
                    .push(expression.span().source_text(self.code).to_string());
                styles.style = Some(span);
                self.visit_expression(expression);
            }
            None => self.error(
                attribute.span.start,
                Problem::new(
                    "React Native takes a style object, not CSS text",
                    "pass an object, such as `style={{ opacity: 0.5 }}`",
                )
                .on(local, "style"),
            ),
        }
    }

    /// The code of the component `as` renders
    fn tag(&mut self, attribute: &JSXAttribute<'a>, local: &str) -> Option<String> {
        let tag = attribute_expression(attribute).filter(|expression| component_name(expression));
        if tag.is_none() {
            self.error(
                attribute.span.start,
                Problem::new(
                    "React Native renders components, not HTML tags or values chosen at runtime",
                    "pass a capitalized component, such as `as={Pressable}`",
                )
                .on(local, "as"),
            );
        }
        tag.map(|expression| expression.span().source_text(self.code).to_string())
    }

    /// `src` of an `Image` as React Native's `source`
    fn image_source(&mut self, attribute: &JSXAttribute<'a>, local: &str) {
        let source = match &attribute.value {
            Some(JSXAttributeValue::StringLiteral(literal)) => {
                Some(uri_source(literal.span, self.code))
            }
            _ => attribute_expression(attribute).and_then(|expression| {
                image_source_code(expression.without_parentheses(), self.code)
            }),
        };
        match source {
            Some(source) => self.replacements.push((
                attribute.span.start as usize,
                attribute.span.end as usize,
                format!("source={{{source}}}"),
            )),
            None => self.error(
                attribute.span.start,
                Problem::new(
                    "the build turns only a literal, a name or a member `src` into a `source`",
                    "assign the value to a variable first, or pass React Native's `source`, such as `source={require('./image.png')}`",
                )
                .on(local, "src"),
            ),
        }
    }

    /// The styles of a platform block, such as `_ios={{ boxShadow: '...' }}`
    fn platform(
        &mut self,
        value: Option<&Expression<'a>>,
        name: &str,
        offset: u32,
        local: &str,
    ) -> Vec<Choice> {
        if let Some(Expression::ObjectExpression(object)) =
            value.map(Expression::without_parentheses)
        {
            return self.object_choices(object, local);
        }
        self.error(
            offset,
            Problem::new(
                "a platform block takes an object of styles",
                format!("write the styles in an object, such as `{name}={{{{ p: 2 }}}}`"),
            )
            .on(local, name),
        );
        Vec::new()
    }

    /// The styles of an object of style props
    fn object_choices(&mut self, object: &ObjectExpression<'a>, local: &str) -> Vec<Choice> {
        let mut choices = Vec::new();
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                self.error(
                    property.span().start,
                    Problem::new(
                        "the build reads only the styles written in a platform block",
                        "write each style in the block",
                    )
                    .on(local, "..."),
                );
                continue;
            };
            let Some(key) = property
                .key
                .static_name()
                .filter(|_| !property.computed)
                .and_then(|key| key.as_str().map(str::to_string))
            else {
                let key = property.key.span().source_text(self.code);
                self.error(
                    property.span.start,
                    Problem::new(
                        "the build reads only keys written in a platform block",
                        "write the key itself",
                    )
                    .on(local, &format!("[{key}]")),
                );
                continue;
            };
            choices.extend(self.object_property(&key, &property.value, property.span.start, local));
        }
        choices
    }

    /// The styles one key of a platform block declares
    fn object_property(
        &mut self,
        key: &str,
        value: &Expression<'a>,
        offset: u32,
        local: &str,
    ) -> Vec<Choice> {
        match classify(key) {
            Prop::Declares => match self.expression_choice(key, value) {
                Ok(choice) => return vec![choice],
                Err(problem) => self.error(offset, problem.on(local, key)),
            },
            Prop::Platform(platform) => {
                let choices = self.platform(Some(value), key, offset, local);
                if platform == self.target.platform {
                    return choices;
                }
            }
            Prop::Runtime(feature) => {
                self.error(offset, feature.problem(self.target).on(local, key));
            }
            Prop::Unsupported(problem) => self.error(offset, problem.on(local, key)),
            Prop::As | Prop::Style | Prop::Pass => self.error(
                offset,
                Problem::new(
                    "a platform block holds only style props",
                    "pass it to the element itself",
                )
                .on(local, key),
            ),
        }
        Vec::new()
    }

    /// The styles a style prop declares with the value it is written with
    fn value_choice(
        &self,
        name: &str,
        value: Option<&JSXAttributeValue<'a>>,
    ) -> Result<Choice, Problem> {
        match value {
            Some(JSXAttributeValue::StringLiteral(literal)) => literal.value.as_str().map_or_else(
                || self.dynamic_choice(name, literal.span.source_text(self.code)),
                |text| static_choice(name, &Literal::Text(text.to_string())),
            ),
            Some(JSXAttributeValue::ExpressionContainer(container)) => container
                .expression
                .as_expression()
                .map_or(Ok(Choice::Empty), |expression| {
                    self.expression_choice(name, expression)
                }),
            _ => Err(Problem::new(
                "a style prop takes a value",
                "give the style a value, such as `p={2}`",
            )),
        }
    }

    /// The styles a style prop declares with `expression`
    fn expression_choice(
        &self,
        name: &str,
        expression: &Expression<'a>,
    ) -> Result<Choice, Problem> {
        if let Some(literal) = literal(expression) {
            return static_choice(name, &literal);
        }
        match expression.without_parentheses() {
            Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => Ok(Choice::Empty),
            Expression::Identifier(identifier) if identifier.name == "undefined" => {
                Ok(Choice::Empty)
            }
            Expression::ConditionalExpression(condition) => Ok(Choice::Condition {
                test: self.source(&condition.test),
                consequent: Box::new(self.expression_choice(name, &condition.consequent)?),
                alternate: Box::new(self.expression_choice(name, &condition.alternate)?),
            }),
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                Ok(Choice::Condition {
                    test: self.source(&logical.left),
                    consequent: Box::new(self.expression_choice(name, &logical.right)?),
                    alternate: Box::new(Choice::Empty),
                })
            }
            Expression::ArrayExpression(_) => Err(RuntimeFeature::Responsive.problem(self.target)),
            Expression::ObjectExpression(_) => Err(Problem::new(
                "a style prop takes one value, not an object",
                "give one value, or use a platform block such as `_ios`",
            )),
            expression => self.dynamic_choice(name, &self.source(expression)),
        }
    }

    /// The styles a style prop declares with the value `code` the element
    /// reads at runtime
    fn dynamic_choice(&self, name: &str, code: &str) -> Result<Choice, Problem> {
        dynamic_keys(name).map(|keys| {
            Choice::Dynamic(
                keys.into_iter()
                    .map(|(key, scale)| (key.to_string(), self.converted(code, scale)))
                    .collect(),
            )
        })
    }

    /// `code`, converted at runtime when its numbers take `scale`
    fn converted(&self, code: &str, scale: Option<u32>) -> String {
        scale.map_or_else(
            || code.to_string(),
            |scale| {
                self.units_used.set(true);
                format!("{}({code}, {scale})", self.units)
            },
        )
    }

    /// The code of `expression`
    fn source(&self, expression: &Expression<'_>) -> String {
        let code = expression.span().source_text(self.code);
        if matches!(
            expression.without_parentheses(),
            Expression::SequenceExpression(_)
        ) {
            format!("({code})")
        } else {
            code.to_string()
        }
    }

    /// Check the children of an element that renders no text, and visit them
    fn children(
        &mut self,
        element: &JSXElement<'a>,
        kind: &ExportVariableKind,
        local: &str,
        retagged: bool,
    ) {
        if !retagged && holds_no_text(native_component(kind)) {
            for child in &element.children {
                if let Some(span) = text_child(child) {
                    self.error(
                        span.start,
                        format!(
                            "`<{local}>` cannot hold text on React Native: it renders a `{}`, and React Native renders text only inside a `Text`. Fix: wrap the text in `<Text>`",
                            native_component(kind)
                        ),
                    );
                }
            }
        }
        self.visit_jsx_children(&element.children);
    }

    /// Give the element its `style` and component
    fn finish(
        &mut self,
        element: &JSXElement<'a>,
        styles: Styles,
        tag: Option<String>,
        attributes_end: u32,
    ) {
        if let Some(tag) = &tag {
            self.rename(element, tag);
        }
        let Styles {
            choices,
            given,
            mut removed,
            style,
            opaque_spread,
        } = styles;
        let Some(code) = style_code(choices, given, &mut self.sheet) else {
            self.remove(&removed);
            return;
        };
        if let Some(spread) = opaque_spread {
            self.error(
                spread.start,
                "A spread on a Devup UI element must be a name or a member on React Native: the element merges the `style` it gives with its own. Fix: assign the spread object to a variable first".to_string(),
            );
        }
        removed.extend(style);
        self.remove(&removed);
        self.replacements.push((
            attributes_end as usize,
            attributes_end as usize,
            format!(" style={{{code}}}"),
        ));
    }

    /// Render the component `as` gives
    fn rename(&mut self, element: &JSXElement<'a>, tag: &str) {
        let names = std::iter::once(element.opening_element.name.span()).chain(
            element
                .closing_element
                .as_ref()
                .map(|closing| closing.name.span()),
        );
        for span in names {
            self.replacements
                .push((span.start as usize, span.end as usize, tag.to_string()));
        }
    }

    fn remove(&mut self, spans: &[Span]) {
        self.replacements.extend(
            spans
                .iter()
                .map(|span| (span.start as usize, span.end as usize, String::new())),
        );
    }
}

/// The `source` of a URL written at `span`
fn uri_source(span: Span, code: &str) -> String {
    format!("{{ uri: {} }}", span.source_text(code))
}

/// The `source` of an `Image` whose `src` is `expression`: a literal is a
/// URL, and a name or a member may hold a URL or, imported through Metro, an
/// asset number React Native takes as it is
fn image_source_code(expression: &Expression<'_>, code: &str) -> Option<String> {
    match expression {
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => {
            Some(uri_source(expression.span(), code))
        }
        Expression::Identifier(_) | Expression::StaticMemberExpression(_) => {
            let value = expression.span().source_text(code);
            Some(format!(
                "typeof {value} === \"number\" ? {value} : {{ uri: {value} }}"
            ))
        }
        _ => None,
    }
}

/// The expression a prop is written with
fn attribute_expression<'b, 'a>(attribute: &'b JSXAttribute<'a>) -> Option<&'b Expression<'a>> {
    match &attribute.value {
        Some(JSXAttributeValue::ExpressionContainer(container)) => {
            container.expression.as_expression()
        }
        _ => None,
    }
}

/// Whether `expression` names a component: a capitalized name, or a member
/// of names such as `Animated.View`
fn component_name(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::Identifier(identifier) => identifier
            .name
            .chars()
            .next()
            .is_some_and(char::is_uppercase),
        Expression::StaticMemberExpression(member) => member_of_names(&member.object),
        _ => false,
    }
}

fn member_of_names(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::Identifier(_) => true,
        Expression::StaticMemberExpression(member) => member_of_names(&member.object),
        _ => false,
    }
}

/// The code of the `style` a spread gives, when its object can be read twice
fn spread_style(argument: &Expression<'_>, code: &str) -> Option<String> {
    member_of_names(argument).then(|| format!("{}.style", argument.span().source_text(code)))
}

/// The literal `expression` writes, if it writes one
fn literal(expression: &Expression<'_>) -> Option<Literal> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal
            .value
            .as_str()
            .map(|text| Literal::Text(text.to_string())),
        Expression::NumericLiteral(literal) => Some(Literal::Number(literal.value)),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::UnaryNegation => {
            match unary.argument.without_parentheses() {
                Expression::NumericLiteral(literal) => Some(Literal::Number(-literal.value)),
                _ => None,
            }
        }
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked?.as_str())
            .map(|cooked| Literal::Text(cooked.to_string())),
        _ => None,
    }
}

/// The styles a style prop declares with a literal
fn static_choice(name: &str, literal: &Literal) -> Result<Choice, Problem> {
    declarations(name, literal).map(Choice::Static)
}

/// The styles a style prop declares with a value read at runtime
/// The import of `StyleSheet` and the declaration of `sheet`
fn sheet_prelude(sheet: &Sheet, code: &str) -> String {
    let style_sheet = fresh_name("StyleSheet", code);
    let import = if style_sheet == "StyleSheet" {
        style_sheet.clone()
    } else {
        format!("StyleSheet as {style_sheet}")
    };
    format!(
        "import {{ {import} }} from \"{REACT_NATIVE}\";
{}
",
        sheet.declaration(&style_sheet)
    )
}

/// The declaration of the function converting a value read at runtime as the
/// build converts a literal: a number or a number in a string is multiplied by
/// the scale `s`, a pixel length is its number, a `rem` or `em` length is
/// in pixels, and any other value stays as it is
pub(super) fn units_declaration(name: &str) -> String {
    format!(
        r#"const {name} = (v, s) => {{ if (typeof v === "number") return v * s; const m = typeof v === "string" && /^\s*(-?(?:\d+\.?\d*|\.\d+))(px|r?em)?\s*$/.exec(v); return m ? Number(m[1]) * (m[2] ? (m[2] === "px" ? 1 : {ROOT_FONT_SIZE}) : s) : v; }};"#
    ) + "\n"
}

/// The span of a child writing text, if it writes any
fn text_child(child: &JSXChild<'_>) -> Option<Span> {
    match child {
        JSXChild::Text(text) if !text.value.trim().is_empty() => Some(text.span),
        JSXChild::ExpressionContainer(container) => match &container.expression {
            JSXExpression::StringLiteral(literal) => Some(literal.span),
            JSXExpression::TemplateLiteral(template) => Some(template.span),
            _ => None,
        },
        _ => None,
    }
}
