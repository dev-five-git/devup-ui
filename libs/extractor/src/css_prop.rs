//! Emotion's `css` prop: styles an element takes in place of a class name.
//! The build composes them like `css()` and gives the element the classes and
//! CSS variables they compile to, so no runtime reads the prop.

use std::collections::HashMap;

use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{
    Argument, ArrowFunctionBody, BinaryOperator, BindingPattern, CallExpression, Expression,
    FormalParameters, FunctionBody, ImportDeclarationSpecifier, JSXAttributeItem, JSXChild,
    JSXElement, JSXElementName, ObjectPropertyKind, Program, Statement, Str, TemplateElement,
    TemplateElementValue, TemplateLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::SPAN;
use rustc_hash::FxHashSet;

use crate::ImportAlias;
use crate::css_utils::{Place, interpolation_place};
use crate::utils::{binding_root, get_string_by_literal_expression, unwrap_syntax_only};

/// The package whose alias makes elements take the `css` prop
pub(crate) const EMOTION_REACT: &str = "@emotion/react";

/// Emotion's JSX runtime, whose alias tells that the project builds JSX with
/// it, as a `jsxImportSource` of `@emotion/react` in tsconfig does
pub(crate) const EMOTION_JSX_RUNTIME: &str = "@emotion/react/jsx-runtime";

/// The pragma building a file's JSX with React, as its `css` props compile
pub(crate) const REACT_JSX_PRAGMA: &str = "/** @jsxImportSource react */\n";

/// Whether the project builds JSX with Emotion's runtime, which then builds it
/// with React's as every `css` prop compiles
pub(crate) fn builds_jsx_with_emotion(import_aliases: &HashMap<String, ImportAlias>) -> bool {
    import_aliases.contains_key(EMOTION_REACT) && import_aliases.contains_key(EMOTION_JSX_RUNTIME)
}

/// Whether `filename` is written in JSX, which the project's JSX settings build
pub(crate) fn is_jsx_file(filename: &str) -> bool {
    filename
        .rsplit_once('.')
        .is_some_and(|(_, extension)| matches!(extension, "tsx" | "jsx"))
}

/// Which elements of a file take Emotion's `css` prop
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CssProp {
    /// No element: Emotion is not aliased, or the file writes no `css` prop
    #[default]
    Off,
    /// Tags and Devup UI components, which take no `css` prop of their own
    Elements,
    /// Every element, as the file shows it uses Emotion
    Everywhere,
}

impl CssProp {
    /// Which elements of `code` take the prop, with the aliases the build
    /// applies, and whether the file shows it uses Emotion
    pub(crate) fn of(
        import_aliases: &HashMap<String, ImportAlias>,
        code: &str,
        uses_emotion: bool,
    ) -> Self {
        if !import_aliases.contains_key(EMOTION_REACT) || !may_take_css_prop(code) {
            Self::Off
        } else if uses_emotion {
            Self::Everywhere
        } else {
            Self::Elements
        }
    }

    /// Whether the element `name` takes the prop; `devup` tells a Devup UI
    /// component
    pub(crate) fn takes(self, name: &JSXElementName<'_>, devup: impl Fn(&str) -> bool) -> bool {
        match self {
            Self::Off => false,
            Self::Everywhere => true,
            Self::Elements => match name {
                JSXElementName::Identifier(_) | JSXElementName::NamespacedName(_) => true,
                name => crate::imported_constants::jsx_root(name).is_some_and(devup),
            },
        }
    }

    /// Whether the element a `jsx()` call builds from `element` takes the prop
    pub(crate) fn takes_type(self, element: &Expression<'_>, devup: impl Fn(&str) -> bool) -> bool {
        match self {
            Self::Off => false,
            Self::Everywhere => true,
            Self::Elements => match unwrap_syntax_only(element) {
                Expression::StringLiteral(_) => true,
                element => binding_root(element).is_some_and(devup),
            },
        }
    }
}

/// What in a program takes the `css` prop: the elements `css_prop` tells, and
/// the `jsx()` calls building them
pub(crate) struct CssTakers<'s> {
    pub css_prop: CssProp,
    /// Local names of the functions building elements from a type and props
    jsx: FxHashSet<&'s str>,
    /// Local names of Emotion's `ClassNames`
    class_names: FxHashSet<&'s str>,
}

impl<'s> CssTakers<'s> {
    /// The elements `program` gives a `css` prop, as `css_prop` tells;
    /// `compat` is the entry absorbing Emotion's own `jsx`
    pub(crate) fn new(program: &Program<'s>, css_prop: CssProp, compat: &str) -> Self {
        let mut jsx = FxHashSet::default();
        let mut class_names = FxHashSet::default();
        for statement in &program.body {
            if let Statement::ImportDeclaration(import) = statement {
                let source = import.source.value.as_str();
                for specifier in import.specifiers.iter().flatten() {
                    let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                        continue;
                    };
                    let imported = specifier.imported.name();
                    if is_jsx_function(source, &imported, compat) {
                        jsx.insert(specifier.local.name.as_str());
                    } else if imported == "ClassNames"
                        && (source == compat || source == EMOTION_REACT)
                    {
                        class_names.insert(specifier.local.name.as_str());
                    }
                }
            }
        }
        Self {
            css_prop,
            jsx,
            class_names,
        }
    }

    /// The names a `<ClassNames>` child function `element` gives takes `css`
    /// and `cx` by; empty for any other element
    pub(crate) fn class_names_calls(&self, element: &JSXElement<'_>) -> Vec<String> {
        match &element.opening_element.name {
            JSXElementName::IdentifierReference(name)
                if self.class_names.contains(name.name.as_str()) =>
            {
                class_names_calls(element)
            }
            _ => Vec::new(),
        }
    }

    /// Whether `attribute` of the element `name` is a `css` prop it takes;
    /// `devup` tells a Devup UI component
    pub(crate) fn attribute(
        &self,
        name: &JSXElementName<'_>,
        attribute: &JSXAttributeItem<'_>,
        devup: impl Fn(&str) -> bool,
    ) -> bool {
        matches!(attribute, JSXAttributeItem::Attribute(attribute)
            if attribute.name.as_identifier().is_some_and(|attribute| attribute.name == "css"))
            && self.css_prop.takes(name, devup)
    }

    /// Where among the props a `jsx()` call gives is the `css` prop the
    /// element takes
    pub(crate) fn property(
        &self,
        call: &CallExpression<'_>,
        devup: impl Fn(&str) -> bool,
    ) -> Option<usize> {
        let Expression::Identifier(callee) = &call.callee else {
            return None;
        };
        let [element, Argument::ObjectExpression(props), ..] = call.arguments.as_slice() else {
            return None;
        };
        (self.jsx.contains(callee.name.as_str())
            && self.css_prop.takes_type(element.as_expression()?, devup))
        .then(|| {
            props.properties.iter().rposition(|property| {
                matches!(property, ObjectPropertyKind::ObjectProperty(property)
                    if !property.computed && property.key.static_name().is_some_and(|key| key == "css"))
            })
        })
        .flatten()
    }
}

/// Whether `code` may give an element a `css` prop: `css` written as a JSX
/// attribute or as the key of an object
pub(crate) fn may_take_css_prop(code: &str) -> bool {
    code.match_indices("css").any(|(index, _)| {
        let before = code[..index].chars().next_back();
        let after = code[index + 3..].trim_start();
        let attribute = before.is_some_and(char::is_whitespace)
            && after.starts_with('=')
            && !after[1..].starts_with('=');
        let key = before.is_some_and(|c| c.is_whitespace() || matches!(c, '{' | ',' | '"' | '\''))
            && after
                .strip_prefix(['"', '\''])
                .unwrap_or(after)
                .trim_start()
                .starts_with(':');
        attribute || key
    })
}

/// Whether importing `source` shows a file uses Emotion
pub(crate) fn is_emotion(source: &str) -> bool {
    [EMOTION_REACT, "@emotion/styled"].iter().any(|package| {
        source
            .strip_prefix(package)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

/// Where the JSX pragma in the comment text `comment` names `@emotion/react`
/// as the module JSX is built with
pub(crate) fn emotion_pragma(comment: &str) -> Option<usize> {
    let at = comment.find("@jsxImportSource")? + "@jsxImportSource".len();
    let value = comment[at..].trim_start();
    let rest = value.strip_prefix(EMOTION_REACT)?;
    rest.chars()
        .next()
        .is_none_or(|c| c.is_whitespace() || c == '*')
        .then(|| comment.len() - value.len())
}

/// The React module an Emotion JSX runtime stands in for
pub(crate) fn react_runtime(source: &str) -> Option<&'static str> {
    match source {
        "@emotion/react/jsx-runtime" => Some("react/jsx-runtime"),
        "@emotion/react/jsx-dev-runtime" => Some("react/jsx-dev-runtime"),
        _ => None,
    }
}

/// Whether `imported` from `source` builds an element from a type and props;
/// `compat` is the entry absorbing Emotion's own `jsx`
pub(crate) fn is_jsx_function(source: &str, imported: &str, compat: &str) -> bool {
    match source {
        "react/jsx-runtime"
        | "react/jsx-dev-runtime"
        | "@emotion/react/jsx-runtime"
        | "@emotion/react/jsx-dev-runtime" => matches!(imported, "jsx" | "jsxs" | "jsxDEV"),
        source => {
            (source == EMOTION_REACT || source == compat)
                && matches!(imported, "jsx" | "createElement")
        }
    }
}

/// What a function of the theme requires to give rules at build time
pub(crate) const THEME_FUNCTION: &str =
    "a function of the theme must give its rules at once, as `theme => ({ ... })`";

/// What a function of the theme requires of its reads of the theme
pub(crate) const THEME_READ: &str = "it may read the theme only as `theme.a.b` in a value, which becomes the CSS variable `var(--a-b)` the `ThemeProvider` sets";

/// What CSS text requires of an interpolation it cannot place
pub(crate) const UNPLACED: &str =
    "an interpolation in CSS text must be a value, or a mixin standing where a declaration would";

/// What CSS text requires of a mixin
pub(crate) const NESTED_MIXIN: &str =
    "a mixin must stand outside nested rules, where the parts it composes with can be split";

/// `function`, a function of the theme, as the rules it gives, each read of
/// the theme written as the CSS variable the `ThemeProvider` sets for it.
/// `Err` holds the code the build cannot write so, with what it requires.
pub(crate) fn theme_rules<'a>(
    ast: &AstBuilder<'a>,
    function: &Expression<'a>,
) -> Result<Expression<'a>, (Expression<'a>, &'static str)> {
    let unsupported = || (function.clone_in(ast.allocator()), THEME_FUNCTION);
    let (params, body) = match function {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            let body = match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => returned(body),
                body => body.as_expression(),
            };
            (&arrow.params, body)
        }
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => (
            &function.params,
            function.body.as_deref().and_then(returned),
        ),
        _ => return Err(unsupported()),
    };
    let (Some(roots), Some(body)) = (theme_roots(params), body) else {
        return Err(unsupported());
    };
    let mut rules = body.clone_in_with_semantic_ids(ast.allocator());
    let mut reads = ThemeReads {
        ast,
        roots: &roots,
        unread: None,
    };
    reads.visit(&mut rules, false);
    match reads.unread {
        Some(unread) => Err((unread, THEME_READ)),
        None => Ok(rules),
    }
}

/// What a function body gives when it is a single `return`
pub(crate) fn returned<'b, 'a>(body: &'b FunctionBody<'a>) -> Option<&'b Expression<'a>> {
    match body.statements.as_slice() {
        [Statement::ReturnStatement(statement)] => statement.argument.as_ref(),
        _ => None,
    }
}

fn returned_mut<'b, 'a>(body: &'b mut FunctionBody<'a>) -> Option<&'b mut Expression<'a>> {
    match body.statements.as_mut_slice() {
        [Statement::ReturnStatement(statement)] => statement.argument.as_mut(),
        _ => None,
    }
}

/// The parameters of `function` and what it gives at once, when it is a
/// plain function giving one value
pub(crate) fn render_function<'b, 'a>(
    function: &'b mut Expression<'a>,
) -> Option<(&'b FormalParameters<'a>, &'b mut Expression<'a>)> {
    match function {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            let arrow = &mut **arrow;
            let body = match &mut arrow.body {
                ArrowFunctionBody::FunctionBody(body) => returned_mut(body)?,
                body => body.as_expression_mut()?,
            };
            Some((&arrow.params, body))
        }
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => {
            let function = &mut **function;
            Some((
                &function.params,
                returned_mut(function.body.as_deref_mut()?)?,
            ))
        }
        _ => None,
    }
}

/// The names Emotion's `<ClassNames>` child function takes `css`, `cx` and
/// the theme by
#[derive(Default, Clone, Copy)]
pub(crate) struct ClassNamesParams<'a> {
    pub css: Option<&'a str>,
    pub cx: Option<&'a str>,
    pub theme: Option<&'a str>,
}

/// What `params` take of `{ css, cx, theme }`; `None` when they take it in a
/// way the build cannot follow
pub(crate) fn class_names_params<'a>(
    params: &FormalParameters<'a>,
) -> Option<ClassNamesParams<'a>> {
    let mut names = ClassNamesParams::default();
    if params.rest.is_some() {
        return None;
    }
    match params.items.as_slice() {
        [] => {}
        [param] if param.initializer.is_none() => {
            let BindingPattern::ObjectPattern(object) = &param.pattern else {
                return None;
            };
            if object.rest.is_some() {
                return None;
            }
            for property in &object.properties {
                let (BindingPattern::BindingIdentifier(local), Some(key), false) = (
                    &property.value,
                    property.key.static_name(),
                    property.computed,
                ) else {
                    return None;
                };
                let slot = match key.as_ref() {
                    "css" => &mut names.css,
                    "cx" => &mut names.cx,
                    "theme" => &mut names.theme,
                    _ => return None,
                };
                *slot = Some(local.name.as_str());
            }
        }
        _ => return None,
    }
    Some(names)
}

/// What the child function of the `<ClassNames>` element `element` takes
pub(crate) fn class_names_child<'a>(element: &JSXElement<'a>) -> Option<ClassNamesParams<'a>> {
    let mut children = element
        .children
        .iter()
        .filter(|child| !matches!(child, JSXChild::Text(text) if text.value.trim().is_empty()));
    let (Some(JSXChild::ExpressionContainer(container)), None) = (children.next(), children.next())
    else {
        return None;
    };
    let params = match container.expression.as_expression()? {
        Expression::ArrowFunctionExpression(arrow) => &arrow.params,
        Expression::FunctionExpression(function) => &function.params,
        _ => return None,
    };
    class_names_params(params)
}

/// The names the child function of the `<ClassNames>` element `element`
/// takes `css` and `cx` by, which read styles
pub(crate) fn class_names_calls(element: &JSXElement<'_>) -> Vec<String> {
    class_names_child(element)
        .map(|names| {
            [names.css, names.cx]
                .into_iter()
                .flatten()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Write the reads of the theme bound to `theme` in `expression`, a CSS value
/// when `value`, as the CSS variables the `ThemeProvider` sets; `Err` holds
/// the first read the build cannot write so
pub(crate) fn read_theme<'a>(
    ast: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    theme: &'a str,
    value: bool,
) -> Result<(), Expression<'a>> {
    let roots = [(theme, None)];
    let mut reads = ThemeReads {
        ast,
        roots: &roots,
        unread: None,
    };
    reads.visit(expression, value);
    reads.unread.map_or(Ok(()), Err)
}

/// The names a function of the theme reads it by, each with the key of the
/// theme it holds when destructured; `None` for parameters the build cannot
/// follow
fn theme_roots<'a>(params: &FormalParameters<'a>) -> Option<Vec<(&'a str, Option<String>)>> {
    if params.rest.is_some() {
        return None;
    }
    match params.items.as_slice() {
        [] => Some(Vec::new()),
        [param] if param.initializer.is_none() => match &param.pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                Some(vec![(identifier.name.as_str(), None)])
            }
            BindingPattern::ObjectPattern(object) if object.rest.is_none() => object
                .properties
                .iter()
                .map(
                    |property| match (&property.value, property.key.static_name()) {
                        (BindingPattern::BindingIdentifier(local), Some(key))
                            if !property.computed =>
                        {
                            Some((local.name.as_str(), Some(key.to_string())))
                        }
                        _ => None,
                    },
                )
                .collect(),
            _ => None,
        },
        _ => None,
    }
}

/// Writes each read of the theme in a value as the CSS variable it becomes,
/// keeping the first read that is not one
struct ThemeReads<'r, 'a> {
    ast: &'r AstBuilder<'a>,
    roots: &'r [(&'a str, Option<String>)],
    unread: Option<Expression<'a>>,
}

/// How an expression reads the theme
enum ThemeRead {
    /// By the path of keys to a value
    Path(Vec<String>),
    /// Otherwise, as a whole or through a call or a key only the runtime gives
    Other,
}

impl<'a> ThemeReads<'_, 'a> {
    /// How `expression` reads the theme, when it does
    fn path(&self, expression: &Expression<'a>) -> Option<ThemeRead> {
        let mut path = Vec::new();
        let mut exact = true;
        let mut cursor = expression;
        loop {
            match cursor {
                Expression::StaticMemberExpression(member) => {
                    path.push(member.property.name.to_string());
                    cursor = &member.object;
                }
                Expression::ComputedMemberExpression(member) => {
                    match get_string_by_literal_expression(&member.expression) {
                        Some(key) => path.push(key.into_owned()),
                        None => exact = false,
                    }
                    cursor = &member.object;
                }
                Expression::CallExpression(call) => {
                    exact = false;
                    cursor = &call.callee;
                }
                Expression::ParenthesizedExpression(inner) => cursor = &inner.expression,
                Expression::Identifier(identifier) => {
                    let (_, key) = self
                        .roots
                        .iter()
                        .find(|(local, _)| *local == identifier.name.as_str())?;
                    path.extend(key.clone());
                    path.reverse();
                    return Some(if exact && !path.is_empty() {
                        ThemeRead::Path(path)
                    } else {
                        ThemeRead::Other
                    });
                }
                _ => return None,
            }
        }
    }

    /// Visit `expression`, which stands where a CSS value does when `value`
    fn visit(&mut self, expression: &mut Expression<'a>, value: bool) {
        match self.path(expression) {
            Some(ThemeRead::Path(path)) if value => {
                let variable = format!("var(--{})", path.join("-"));
                *expression = Expression::new_string_literal(
                    SPAN,
                    Str::from_in(variable.as_str(), self.ast.allocator()),
                    None,
                    self.ast,
                );
                return;
            }
            Some(_) => {
                if self.unread.is_none() {
                    self.unread = Some(expression.clone_in(self.ast.allocator()));
                }
                return;
            }
            None => {}
        }
        match expression {
            Expression::ObjectExpression(object) => {
                for property in &mut object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if let Some(key) = property.key.as_expression_mut() {
                                self.visit(key, false);
                            }
                            self.visit(&mut property.value, true);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.visit(&mut spread.argument, false);
                        }
                    }
                }
            }
            Expression::TemplateLiteral(template) => {
                for expression in &mut template.expressions {
                    self.visit(expression, true);
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &mut array.elements {
                    if let Some(element) = element.as_expression_mut() {
                        self.visit(element, value);
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.visit(&mut conditional.test, false);
                self.visit(&mut conditional.consequent, value);
                self.visit(&mut conditional.alternate, value);
            }
            Expression::LogicalExpression(logical) => {
                self.visit(&mut logical.left, false);
                self.visit(&mut logical.right, value);
            }
            // Text joined to a string stays text
            Expression::BinaryExpression(binary)
                if binary.operator == BinaryOperator::Addition
                    && [&binary.left, &binary.right].into_iter().any(|side| {
                        matches!(
                            side,
                            Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
                        )
                    }) =>
            {
                self.visit(&mut binary.left, value);
                self.visit(&mut binary.right, value);
            }
            Expression::ParenthesizedExpression(inner) => self.visit(&mut inner.expression, value),
            expression => walk_mut::walk_expression(self, expression),
        }
    }
}

impl<'a> VisitMut<'a> for ThemeReads<'_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        self.visit(expression, false);
    }
}

/// CSS text as the parts of a `css` prop: when `mixins`, an interpolation
/// standing where a declaration would is a mixin composed there, splitting the
/// text around it. `Err` holds an interpolation the parts cannot place, with
/// what the text requires of it.
pub(crate) fn template_parts<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    mixins: bool,
) -> Result<Vec<Expression<'a>>, (Expression<'a>, &'static str)> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut depth = 0usize;
    let mut from = 0;
    for (index, expression) in template.expressions.iter().enumerate() {
        let quasi = template.quasis[index].value.raw.as_str();
        text.push_str(quasi);
        depth = (depth + quasi.matches('{').count()).saturating_sub(quasi.matches('}').count());
        let place = interpolation_place(&text, &template.quasis[index + 1..]);
        if matches!(place, Place::Value) || get_string_by_literal_expression(expression).is_some() {
            continue;
        }
        let requirement = match place {
            Place::Statement if mixins && depth == 0 => {
                parts.extend(segment(ast, template, from, index));
                parts.push(expression.clone_in_with_semantic_ids(ast.allocator()));
                from = index + 1;
                continue;
            }
            Place::Statement if mixins => NESTED_MIXIN,
            _ => UNPLACED,
        };
        return Err((expression.clone_in(ast.allocator()), requirement));
    }
    parts.extend(segment(ast, template, from, template.expressions.len()));
    Ok(parts)
}

/// The text of `template` from the quasi `from` to the quasi `to`, with the
/// values between them; `None` when it holds nothing
fn segment<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    from: usize,
    to: usize,
) -> Option<Expression<'a>> {
    let quasis = &template.quasis[from..=to];
    if from == to && quasis[0].value.raw.trim().is_empty() {
        return None;
    }
    let allocator = ast.allocator();
    let quasis = quasis.iter().enumerate().map(|(index, quasi)| {
        TemplateElement::new(
            SPAN,
            TemplateElementValue {
                raw: quasi.value.raw,
                cooked: quasi.value.cooked,
            },
            index == to - from,
            ast,
        )
    });
    let expressions = template.expressions[from..to]
        .iter()
        .map(|expression| expression.clone_in_with_semantic_ids(allocator));
    Some(Expression::new_template_literal(
        SPAN,
        oxc_allocator::Vec::from_iter_in(quasis, ast),
        oxc_allocator::Vec::from_iter_in(expressions, ast),
        ast,
    ))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::literal_string_with_formatting_args
)]
mod tests {
    use super::*;
    use crate::utils::expression_to_code;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    fn parsed<'a>(allocator: &'a Allocator, code: &'a str) -> Expression<'a> {
        Parser::new(allocator, code, SourceType::tsx())
            .parse_expression()
            .unwrap()
    }

    #[test]
    fn test_css_prop_needles() {
        for code in [
            "<div css={{}} />",
            "<div\n\tcss = {a} />",
            "jsx('div', { css: a })",
            "jsx('div', {css:a})",
            "jsx('div', {\"css\": a})",
            "jsx('div', {'css' : a})",
            "f(a,css:b)",
        ] {
            assert!(may_take_css_prop(code), "{code}");
        }
        for code in [
            "import './a.css'",
            "a.css = b",
            "if (a css == b) {}",
            "css(a)",
            "<div className=\"css\" />",
            "css",
        ] {
            assert!(!may_take_css_prop(code), "{code}");
        }
    }

    #[test]
    fn test_css_prop_of_aliases() {
        let mut aliases = HashMap::new();
        assert_eq!(CssProp::of(&aliases, "<div css={a} />", true), CssProp::Off);
        aliases.insert(EMOTION_REACT.to_string(), ImportAlias::NamedToNamed);
        assert_eq!(CssProp::of(&aliases, "<div />", true), CssProp::Off);
        assert_eq!(
            CssProp::of(&aliases, "<div css={a} />", false),
            CssProp::Elements
        );
        assert_eq!(
            CssProp::of(&aliases, "<div css={a} />", true),
            CssProp::Everywhere
        );
    }

    #[test]
    fn test_emotion_sources_and_pragmas() {
        assert!(is_emotion("@emotion/react"));
        assert!(is_emotion("@emotion/react/jsx-runtime"));
        assert!(is_emotion("@emotion/styled"));
        assert!(!is_emotion("@emotion/reactive"));
        assert!(!is_emotion("@emotion/css"));

        assert_eq!(
            emotion_pragma("* @jsxImportSource @emotion/react "),
            Some(19)
        );
        assert_eq!(emotion_pragma("@jsxImportSource @emotion/react*"), Some(17));
        assert_eq!(
            emotion_pragma(" @jsxImportSource\t@emotion/react"),
            Some(18)
        );
        assert_eq!(emotion_pragma("@jsxImportSource react"), None);
        assert_eq!(emotion_pragma("@jsxImportSource @emotion/reactive"), None);
        assert_eq!(emotion_pragma("@jsx jsx"), None);

        assert_eq!(
            react_runtime("@emotion/react/jsx-runtime"),
            Some("react/jsx-runtime")
        );
        assert_eq!(
            react_runtime("@emotion/react/jsx-dev-runtime"),
            Some("react/jsx-dev-runtime")
        );
        assert_eq!(react_runtime("@emotion/react"), None);

        assert!(is_jsx_file("src/App.tsx"));
        assert!(is_jsx_file("App.jsx"));
        assert!(!is_jsx_file("App.ts"));
        assert!(!is_jsx_file("Makefile"));

        let mut aliases =
            HashMap::from([(EMOTION_JSX_RUNTIME.to_string(), ImportAlias::NamedToNamed)]);
        assert!(!builds_jsx_with_emotion(&aliases));
        aliases.insert(EMOTION_REACT.to_string(), ImportAlias::NamedToNamed);
        assert!(builds_jsx_with_emotion(&aliases));

        let compat = "@devup-ui/react/compat";
        assert!(is_jsx_function("react/jsx-runtime", "jsxs", compat));
        assert!(is_jsx_function(
            "@emotion/react/jsx-dev-runtime",
            "jsxDEV",
            compat
        ));
        assert!(!is_jsx_function("react/jsx-runtime", "Fragment", compat));
        assert!(is_jsx_function("@emotion/react", "createElement", compat));
        assert!(is_jsx_function(compat, "jsx", compat));
        assert!(!is_jsx_function("react", "createElement", compat));
        assert!(!is_jsx_function(EMOTION_REACT, "css", compat));
    }

    #[test]
    fn test_takes() {
        let allocator = Allocator::default();
        let devup = |name: &str| name == "Box";
        for (code, off, elements, everywhere) in [
            ("<div />", false, true, true),
            ("<svg:use />", false, true, true),
            ("<Box />", false, true, true),
            ("<Custom />", false, false, true),
            ("<this.Custom />", false, false, true),
        ] {
            let Expression::JSXElement(element) = parsed(&allocator, code) else {
                panic!("{code}");
            };
            let name = &element.opening_element.name;
            assert_eq!(CssProp::Off.takes(name, devup), off, "{code}");
            assert_eq!(CssProp::Elements.takes(name, devup), elements, "{code}");
            assert_eq!(CssProp::Everywhere.takes(name, devup), everywhere, "{code}");
        }
        for (code, elements) in [
            ("'div'", true),
            ("Box", true),
            ("Custom", false),
            ("f()", false),
        ] {
            let element = parsed(&allocator, code);
            assert!(!CssProp::Off.takes_type(&element, devup), "{code}");
            assert_eq!(
                CssProp::Elements.takes_type(&element, devup),
                elements,
                "{code}"
            );
            assert!(CssProp::Everywhere.takes_type(&element, devup), "{code}");
        }
    }

    #[test]
    fn test_theme_rules() {
        let allocator = Allocator::default();
        let ast = AstBuilder::new(&allocator);
        for (code, expected) in [
            (
                "theme => ({ color: theme.colors.primary })",
                "({color:`var(--colors-primary)`});",
            ),
            (
                "({ colors, space: s }) => ({ color: colors.text, margin: s[2] })",
                "({color:`var(--colors-text)`,margin:`var(--space-2)`});",
            ),
            (
                "function (t) { return [{ border: `1px solid ${t.line}` }, a && { color: '#' + t.a }]; }",
                "[{border:`1px solid ${`var(--line)`}`},a&&{color:`#`+`var(--a)`}];",
            ),
            (
                "t => c ? { color: (t.a) } : { [k]: t['b'], ...rest }",
                "c?{color:`var(--a)`}:{[k]:`var(--b)`,...rest};",
            ),
            ("() => ({ color: 'red' })", "({color:`red`});"),
        ] {
            let rules = theme_rules(&ast, &parsed(&allocator, code)).unwrap();
            assert_eq!(expression_to_code(&rules), expected, "{code}");
        }
        for (code, unread) in [
            ("t => ({ color: t })", "t"),
            ("t => ({ ...t.mixins })", "t.mixins"),
            ("t => ({ margin: t.spacing(2) })", "t.spacing(2)"),
            ("t => ({ color: t.colors[name] })", "t.colors[name]"),
            ("t => t.dark ? {} : {}", "t.dark"),
            ("t => ({ width: t.a * 2 })", "t.a"),
            ("t => ({ color: t.a || 'red' })", "t.a"),
            ("t => ({ color: f(t.a), border: t.b() })", "t.a"),
        ] {
            let (read, requirement) = theme_rules(&ast, &parsed(&allocator, code)).unwrap_err();
            assert_eq!(requirement, THEME_READ, "{code}");
            assert_eq!(expression_to_code(&read), format!("{unread};"), "{code}");
        }
        for code in [
            "async t => ({})",
            "function* (t) { return {}; }",
            "async function (t) { return {}; }",
            "(t, u) => ({})",
            "(...t) => ({})",
            "(t = {}) => ({})",
            "([t]) => ({})",
            "({ a: { b } }) => ({})",
            "({ ...t }) => ({})",
            "({ [k]: t }) => ({})",
            "t => { const a = 1; return {}; }",
            "t => {}",
            "function (t) {}",
            "a",
        ] {
            let (_, requirement) = theme_rules(&ast, &parsed(&allocator, code)).unwrap_err();
            assert_eq!(requirement, THEME_FUNCTION, "{code}");
        }
    }

    #[test]
    fn test_template_parts() {
        let allocator = Allocator::default();
        let ast = AstBuilder::new(&allocator);
        let parts = |code: &'static str, mixins: bool| {
            let Expression::TemplateLiteral(template) = parsed(&allocator, code) else {
                panic!("{code}");
            };
            template_parts(&ast, &template, mixins)
                .map(|parts| parts.iter().map(expression_to_code).collect::<String>())
        };
        assert_eq!(
            parts("`color: ${c}; ${'margin: 0'};`", false).unwrap(),
            "`color: ${c}; ${`margin: 0`};`;"
        );
        assert_eq!(
            parts("`${base}; color: ${c}; ${other}`", true).unwrap(),
            "base;`; color: ${c}; `;other;"
        );
        assert_eq!(parts("``", true).unwrap(), "");
        let (_, requirement) = parts("`${base}; color: red;`", false).unwrap_err();
        assert_eq!(requirement, UNPLACED);
        let (_, requirement) = parts("`&:hover { ${base}; }`", true).unwrap_err();
        assert_eq!(requirement, NESTED_MIXIN);
        let (code, requirement) = parts("`${selector} { color: red; }`", true).unwrap_err();
        assert_eq!(requirement, UNPLACED);
        assert_eq!(expression_to_code(&code), "selector;");
    }
}
