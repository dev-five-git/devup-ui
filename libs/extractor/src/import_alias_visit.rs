//! Import alias transformation
//!
//! Transforms imports from aliased packages to the target package.
//!
//! Examples:
//! - `import styled from '@emotion/styled'` → `import { styled } from '@devup-ui/react'`
//! - `import styledA from '@emotion/styled'` → `import { styled as styledA } from '@devup-ui/react'`
//! - `import { style } from '@vanilla-extract/css'` → `import { style } from '@devup-ui/react'`

use crate::ImportAlias;
use crate::css_prop::{
    CssProp, EMOTION_REACT, builds_jsx_with_emotion, class_names_child, is_emotion, is_jsx_file,
    is_jsx_function, react_runtime, returned,
};
use crate::utils::{
    get_str_by_property_key, is_vanilla_extract_file, js_number_literal, keeps_bare_number,
};
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrowFunctionBody, CallExpression, Expression, ImportDeclarationSpecifier,
    JSXAttributeItem, JSXAttributeValue, JSXElement, JSXElementName, JSXOpeningElement,
    LogicalOperator, ModuleExportName, ObjectPropertyKind, Statement,
};
use oxc_ast_visit::{
    Visit,
    walk::{walk_call_expression, walk_jsx_element, walk_jsx_opening_element},
};
use oxc_parser::Parser;
use oxc_span::GetSpan;
#[cfg(test)]
use oxc_span::SourceType;
use std::borrow::Cow;
use std::collections::HashMap;

/// Where a style function takes its rules
#[derive(Clone, Copy)]
enum RulesAt {
    Argument(usize),
    EveryArgument,
}

/// Numbers in rules written against vanilla-extract, Emotion or styled-components
/// outside a stylesheet. Those calls become Devup UI's, which read a number as its
/// spacing scale, so it is rewritten to the `px` string the library makes of it.
#[derive(Default)]
struct LibraryNumbers<'n> {
    /// Local name of each style function and where its rules are
    calls: Vec<(&'n str, RulesAt)>,
    /// Local names that build styled components
    styled: Vec<&'n str>,
    /// Local names of components taking rules in `styles` (Emotion's `Global`)
    components: Vec<&'n str>,
    /// Which elements take the `css` prop, whose rules are Emotion's
    css_prop: CssProp,
    /// Local names of Devup UI's exports, which take the `css` prop as tags do
    devup: Vec<&'n str>,
    /// Local names of the functions building elements from a type and props
    jsx: Vec<&'n str>,
    /// Local names of Emotion's `ClassNames`, whose child function takes a
    /// `css` taking rules
    class_names: Vec<&'n str>,
    class_names_calls: Vec<oxc_syntax::symbol::SymbolId>,
    scoping: Option<oxc_semantic::Scoping>,
    replacements: Vec<(usize, usize, String)>,
}

impl LibraryNumbers<'_> {
    /// `styled.div`, `styled(tag)`, `styled(tag, options)` and their
    /// `.attrs()` / `.withConfig()`: whatever a call of it passes are rules
    fn is_styled_factory(&self, callee: &Expression) -> bool {
        match callee {
            Expression::StaticMemberExpression(member) => {
                matches!(&member.object, Expression::Identifier(root) if self.styled.contains(&root.name.as_str()))
            }
            Expression::CallExpression(call) => match &call.callee {
                Expression::Identifier(root) => self.styled.contains(&root.name.as_str()),
                Expression::StaticMemberExpression(member) => {
                    matches!(member.property.name.as_str(), "attrs" | "withConfig")
                        && self.is_styled_factory(&member.object)
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn pixelify(&mut self, rules: &Expression) {
        match rules {
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(element) = element.as_expression() {
                        self.pixelify(element);
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && let Some(key) = get_str_by_property_key(&property.key)
                    {
                        self.pixelify_value(&key, &property.value);
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.pixelify(&conditional.consequent);
                self.pixelify(&conditional.alternate);
            }
            Expression::LogicalExpression(logical) => self.pixelify(&logical.right),
            Expression::ParenthesizedExpression(inner) => self.pixelify(&inner.expression),
            _ => {}
        }
    }

    /// The rules a `css` prop composes: a function of the theme gives them
    fn pixelify_css(&mut self, value: &Expression) {
        match value {
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(element) = element.as_expression() {
                        self.pixelify_css(element);
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.pixelify_css(&conditional.consequent);
                self.pixelify_css(&conditional.alternate);
            }
            Expression::LogicalExpression(logical) => {
                if logical.operator != LogicalOperator::And {
                    self.pixelify_css(&logical.left);
                }
                self.pixelify_css(&logical.right);
            }
            Expression::ParenthesizedExpression(inner) => self.pixelify_css(&inner.expression),
            Expression::ArrowFunctionExpression(arrow) => {
                let rules = match &arrow.body {
                    ArrowFunctionBody::FunctionBody(body) => returned(body),
                    body => body.as_expression(),
                };
                if let Some(rules) = rules {
                    self.pixelify_css(rules);
                }
            }
            Expression::FunctionExpression(function) => {
                if let Some(rules) = function.body.as_deref().and_then(returned) {
                    self.pixelify_css(rules);
                }
            }
            rules => self.pixelify(rules),
        }
    }

    /// The value of the attribute `name` on `element`, pixelified as `css`
    /// prop rules when `css`
    fn pixelify_attribute(&mut self, element: &JSXOpeningElement, name: &str) {
        for attribute in &element.attributes {
            if let JSXAttributeItem::Attribute(attribute) = attribute
                && attribute
                    .name
                    .as_identifier()
                    .is_some_and(|attribute| attribute.name == name)
                && let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value
                && let Some(rules) = container.expression.as_expression()
            {
                if name == "css" {
                    self.pixelify_css(rules);
                } else {
                    self.pixelify(rules);
                }
            }
        }
    }

    fn pixelify_value(&mut self, key: &str, value: &Expression) {
        if let Some(number) = js_number_literal(value) {
            if number != 0.0 && !keeps_bare_number(key) {
                let span = value.span();
                self.replacements.push((
                    span.start as usize,
                    span.end as usize,
                    format!("\"{number}px\""),
                ));
            }
            return;
        }
        match value {
            Expression::ArrayExpression(array) => self.fallbacks(key, array),
            Expression::ObjectExpression(_) => {
                if key != "vars" {
                    self.pixelify(value);
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.pixelify_value(key, &conditional.consequent);
                self.pixelify_value(key, &conditional.alternate);
            }
            Expression::LogicalExpression(logical) => self.pixelify_value(key, &logical.right),
            Expression::ParenthesizedExpression(inner) => {
                self.pixelify_value(key, &inner.expression);
            }
            _ => {}
        }
    }
}

impl LibraryNumbers<'_> {
    /// `key: [a, b]` in these libraries declares `key` once per value, later
    /// ones as fallbacks a browser keeps when it understands them, where Devup
    /// UI reads an array as breakpoints. Literal values become one value
    /// declaring each in order; others stay as written.
    fn fallbacks(&mut self, key: &str, array: &oxc_ast::ast::ArrayExpression) {
        let values: Option<Vec<String>> = array
            .elements
            .iter()
            .map(|element| {
                let element = element.as_expression()?;
                if let Some(number) = js_number_literal(element) {
                    return Some(if number == 0.0 || keeps_bare_number(key) {
                        number.to_string()
                    } else {
                        format!("{number}px")
                    });
                }
                match element {
                    Expression::StringLiteral(literal) => Some(literal.value.to_string()),
                    _ => None,
                }
            })
            .collect();
        let Some(values) = values.filter(|values| values.len() > 1) else {
            return;
        };
        let property = kebab_property(key);
        let joined = values.join(&format!(";{property}:"));
        let span = array.span;
        self.replacements.push((
            span.start as usize,
            span.end as usize,
            format!("{joined:?}"),
        ));
    }
}

/// `backgroundColor` as the CSS property `background-color`, `WebkitBoxShadow`
/// as `-webkit-box-shadow`
fn kebab_property(key: &str) -> String {
    let mut property = String::with_capacity(key.len() + 4);
    for (index, c) in key.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if index > 0
                || matches!(key, k if k.starts_with("Webkit") || k.starts_with("Moz") || k.starts_with("Ms"))
            {
                property.push('-');
            }
            property.push(c.to_ascii_lowercase());
        } else {
            property.push(c);
        }
    }
    property
}

impl<'a> Visit<'a> for LibraryNumbers<'a> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'a>) {
        let css = match &element.opening_element.name {
            JSXElementName::IdentifierReference(name)
                if self.class_names.contains(&name.name.as_str())
                    && self.scoping.as_ref().is_some_and(|scoping| {
                        crate::css_prop::binding_of(scoping, name).is_some_and(|symbol| {
                            scoping.get_root_binding(name.name.as_str().into()) == Some(symbol)
                        })
                    }) =>
            {
                class_names_child(element).and_then(|names| names.css)
            }
            _ => None,
        };
        if let Some(css) = css {
            self.class_names_calls.push(css);
        }
        walk_jsx_element(self, element);
        if css.is_some() {
            self.class_names_calls.pop();
        }
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let rules_at = match &call.callee {
            Expression::Identifier(callee) => {
                let symbol = self
                    .scoping
                    .as_ref()
                    .and_then(|scoping| crate::css_prop::binding_of(scoping, callee));
                if symbol.is_some_and(|symbol| self.class_names_calls.contains(&symbol)) {
                    Some(RulesAt::EveryArgument)
                } else {
                    self.calls
                        .iter()
                        .find(|(name, _)| {
                            *name == callee.name.as_str()
                                && self.scoping.as_ref().is_none_or(|scoping| {
                                    symbol.is_some()
                                        && scoping.get_root_binding(callee.name.as_str().into())
                                            == symbol
                                })
                        })
                        .map(|(_, rules_at)| *rules_at)
                }
            }
            callee => self
                .is_styled_factory(callee)
                .then_some(RulesAt::EveryArgument),
        };
        let rules: Vec<&Expression<'a>> = match rules_at {
            Some(RulesAt::Argument(index)) => call
                .arguments
                .get(index)
                .and_then(Argument::as_expression)
                .into_iter()
                .collect(),
            Some(RulesAt::EveryArgument) => call
                .arguments
                .iter()
                .filter_map(Argument::as_expression)
                .collect(),
            None => vec![],
        };
        for rules in rules {
            self.pixelify(rules);
        }
        if let Expression::Identifier(callee) = &call.callee
            && self.jsx.contains(&callee.name.as_str())
            && let [element, Argument::ObjectExpression(props), ..] = call.arguments.as_slice()
            && let Some(element) = element.as_expression()
            && self
                .css_prop
                .takes_type(element, |root| self.devup.contains(&root))
        {
            for property in &props.properties {
                if let ObjectPropertyKind::ObjectProperty(property) = property
                    && !property.computed
                    && property.key.static_name().is_some_and(|key| key == "css")
                {
                    self.pixelify_css(&property.value);
                }
            }
        }
        walk_call_expression(self, call);
    }

    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'a>) {
        if let JSXElementName::IdentifierReference(name) = &element.name
            && self.components.contains(&name.name.as_str())
        {
            self.pixelify_attribute(element, "styles");
        }
        if self
            .css_prop
            .takes(&element.name, |root| self.devup.contains(&root))
        {
            self.pixelify_attribute(element, "css");
        }
        walk_jsx_opening_element(self, element);
    }
}

/// Map an aliased package's export onto the `@devup-ui/react` export that implements the
/// same behaviour, so the extractor consumes the call and drops the import entirely — no
/// dependency on either package survives.
///
/// `None` means the name has no devup-ui counterpart. Redirecting it anyway would produce
/// an ESM "does not provide an export" error, so the specifier stays on its own package
/// and the source library remains a real dependency.
/// Where a redirected specifier lands.
///
/// `Main` names are genuine Devup UI APIs. `Compat` names only exist to absorb another
/// library, so they live in the `<package>/compat` entry and never widen what a project
/// using Devup UI directly sees — which also lets them keep their original spelling
/// (`useTheme` there cannot collide with Devup UI's own `useTheme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DevupTarget<'n> {
    Main(&'n str),
    Compat(&'n str),
}

fn devup_equivalent(source: &str, imported: &str) -> Option<DevupTarget<'static>> {
    match (source, imported) {
        // `style({...})` and `css({...})` both hand back a class name for a style object,
        // and `globalStyle(selector, rules)` is `globalCss` with the selector split out.
        ("@vanilla-extract/css", "style") | (_, "css") => Some(DevupTarget::Main("css")),
        // `cx` and `merge` take class names rather than styles, so they are no `css`
        ("@emotion/css", "cx") => Some(DevupTarget::Compat("cx")),
        ("@emotion/css", "merge") => Some(DevupTarget::Compat("merge")),
        ("@vanilla-extract/css", "globalStyle") | ("@emotion/css", "injectGlobal") => {
            Some(DevupTarget::Main("globalCss"))
        }
        (_, "keyframes") => Some(DevupTarget::Main("keyframes")),
        (_, "styled") => Some(DevupTarget::Main("styled")),
        // The `css` props it builds compile away, leaving React's own element
        ("@emotion/react", "jsx" | "createElement") => Some(DevupTarget::Compat("jsx")),
        ("@emotion/react", "ClassNames") => Some(DevupTarget::Compat("ClassNames")),
        (_, "createGlobalStyle") => Some(DevupTarget::Compat("createGlobalStyle")),
        (_, "Global") => Some(DevupTarget::Compat("Global")),
        (_, "ThemeProvider") => Some(DevupTarget::Compat("ThemeProvider")),
        (_, "ServerStyleSheet") => Some(DevupTarget::Compat("ServerStyleSheet")),
        (_, "StyleSheetManager") => Some(DevupTarget::Compat("StyleSheetManager")),
        (_, "isStyledComponent") => Some(DevupTarget::Compat("isStyledComponent")),
        (_, "withTheme") => Some(DevupTarget::Compat("withTheme")),
        (_, "useTheme") => Some(DevupTarget::Compat("useTheme")),
        ("@emotion/react", "CacheProvider") => Some(DevupTarget::Compat("CacheProvider")),
        ("styled-components", "ThemeConsumer") => Some(DevupTarget::Compat("ThemeConsumer")),
        ("styled-components", "createTheme") => Some(DevupTarget::Compat("createTheme")),
        _ => None,
    }
}

#[cfg(test)]
pub fn transform_import_aliases<'a>(
    code: &'a str,
    filename: &str,
    package: &str,
    import_aliases: &HashMap<String, ImportAlias>,
) -> Cow<'a, str> {
    transform_import_aliases_with_edits(code, filename, package, import_aliases).code
}

/// What rewriting aliased imports gives
pub struct Aliased<'a> {
    /// The code, or the original code when nothing was rewritten
    pub code: Cow<'a, str>,
    /// The replacements made in order, so a position in the code maps back to
    /// the source
    pub edits: Vec<Edit>,
    /// Which elements take Emotion's `css` prop
    pub css_prop: CssProp,
}

/// Packages generating CSS on top of vanilla-extract's, which Devup UI does not
/// compile
const VANILLA_EXTRACT_COMPANIONS: [&str; 2] =
    ["@vanilla-extract/recipes", "@vanilla-extract/sprinkles"];

/// The value imports of vanilla-extract's companion packages, as errors at
/// their offsets: while `@vanilla-extract/css` compiles to Devup UI, the
/// rules they generate would never reach the stylesheet
pub fn companion_errors(code: &str, filename: &str) -> Vec<(u32, String)> {
    if !VANILLA_EXTRACT_COMPANIONS
        .iter()
        .any(|package| code.contains(package))
    {
        return Vec::new();
    }
    let allocator = Allocator::default();
    let source_type = crate::parsing::source_type(filename).unwrap_or_default();
    let program = Parser::new(&allocator, code, source_type).parse().program;
    program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::ImportDeclaration(import)
                if !import.import_kind.is_type()
                    && VANILLA_EXTRACT_COMPANIONS.contains(&import.source.value.as_str()) =>
            {
                Some((
                    import.span.start,
                    format!(
                        "`{}` generates CSS that Devup UI does not compile, so with the `@vanilla-extract/css` alias its rules would never reach the stylesheet. Fix: write the styles with `css()`, or set `importAliases: {{ '@vanilla-extract/css': false }}` and build with the vanilla-extract plugin",
                        import.source.value
                    ),
                ))
            }
            _ => None,
        })
        .collect()
}

/// A replacement of `code[start..end]` by text of `length` bytes
pub type Edit = (usize, usize, usize);

/// Transform source code by rewriting aliased imports to the target package
///
/// # Arguments
/// * `code` - The source code to transform
/// * `filename` - The filename (used for source type detection)
/// * `package` - The target package (e.g., "@devup-ui/react")
/// * `import_aliases` - Map of source package → alias configuration
///
/// # Returns
/// The transformed source code, or the original code if no transformations were
/// needed, the replacements made in order, so a position in the result maps
/// back to the source, and which elements take Emotion's `css` prop
pub fn transform_import_aliases_with_edits<'a>(
    code: &'a str,
    filename: &str,
    package: &str,
    import_aliases: &HashMap<String, ImportAlias>,
) -> Aliased<'a> {
    let unchanged = |css_prop| Aliased {
        code: Cow::Borrowed(code),
        edits: Vec::new(),
        css_prop,
    };
    let emotion_aliased = import_aliases.contains_key(EMOTION_REACT);
    let emotion_jsx = builds_jsx_with_emotion(import_aliases);
    // Quick check: if no aliases match, no element takes a `css` prop and the
    // project builds no JSX with Emotion, return original code
    if !(emotion_jsx && is_jsx_file(filename))
        && CssProp::of(import_aliases, code, false) == CssProp::Off
        && (import_aliases.is_empty() || !import_aliases.keys().any(|alias| code.contains(alias)))
    {
        return unchanged(CssProp::Off);
    }

    let allocator = Allocator::default();
    let source_type = crate::parsing::source_type(filename).unwrap_or_default();

    // Parse the code
    let parser_ret = Parser::new(&allocator, code, source_type).parse();
    let program = parser_ret.program;

    let redirect_every_name = is_vanilla_extract_file(filename);

    // Collect import transformations
    let mut transformations: Vec<(usize, usize, String)> = Vec::new();
    if emotion_aliased {
        transformations.extend(crate::jsx_import_source::emotion_replacements(&program));
    }
    let mut numbers = LibraryNumbers::default();
    let mut uses_emotion = emotion_jsx || !transformations.is_empty();
    let compat = format!("{package}/compat");
    if emotion_jsx
        && is_jsx_file(filename)
        && let Some(replacement) = crate::jsx_import_source::project_replacement(&program)
    {
        transformations.push(replacement);
    }

    for stmt in &program.body {
        if let Statement::ImportDeclaration(import_decl) = stmt {
            let source_value = import_decl.source.value.as_str();
            uses_emotion |= is_emotion(source_value);
            for specifier in import_decl.specifiers.iter().flatten() {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(spec)
                        if is_jsx_function(
                            source_value,
                            imported_name(&spec.imported),
                            &compat,
                        ) =>
                    {
                        numbers.jsx.push(spec.local.name.as_str());
                    }
                    specifier if source_value == package => {
                        numbers.devup.push(specifier.local().name.as_str());
                    }
                    _ => {}
                }
            }
            if emotion_aliased && let Some(runtime) = react_runtime(source_value) {
                let span = import_decl.source.span;
                transformations.push((
                    span.start as usize,
                    span.end as usize,
                    format!("\"{runtime}\""),
                ));
            } else if let Some(alias) = import_aliases.get(source_value)
                && !import_decl.import_kind.is_type()
            {
                let span = import_decl.span;
                let new_import =
                    generate_transformed_import(import_decl, alias, package, redirect_every_name);
                transformations.push((span.start as usize, span.end as usize, new_import));
                if !redirect_every_name {
                    for specifier in import_decl.specifiers.iter().flatten() {
                        match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                                let local = spec.local.name.as_str();
                                match (source_value, imported_name(&spec.imported)) {
                                    ("@vanilla-extract/css", "style" | "keyframes") => {
                                        numbers.calls.push((local, RulesAt::Argument(0)));
                                    }
                                    ("@vanilla-extract/css", "globalStyle") => {
                                        numbers.calls.push((local, RulesAt::Argument(1)));
                                    }
                                    (
                                        "@emotion/react" | "styled-components",
                                        "css" | "keyframes",
                                    )
                                    | ("@emotion/css", "css" | "keyframes" | "injectGlobal") => {
                                        numbers.calls.push((local, RulesAt::EveryArgument));
                                    }
                                    ("@emotion/react", "Global") => numbers.components.push(local),
                                    ("@emotion/react", "ClassNames") => {
                                        numbers.class_names.push(local);
                                    }
                                    _ => {}
                                }
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(spec)
                                if matches!(
                                    source_value,
                                    "@emotion/styled" | "styled-components"
                                ) =>
                            {
                                numbers.styled.push(spec.local.name.as_str());
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec)
                                if matches!(
                                    source_value,
                                    "@emotion/styled" | "styled-components"
                                ) =>
                            {
                                numbers.styled.push(spec.local.name.as_str());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    numbers.css_prop = CssProp::of(import_aliases, code, uses_emotion);
    if !numbers.class_names.is_empty()
        && (code.bytes().any(|byte| byte.is_ascii_digit()) || code.contains('['))
    {
        numbers.scoping = Some(
            oxc_semantic::SemanticBuilder::new()
                .build(&program)
                .semantic
                .into_scoping(),
        );
    }
    if !(numbers.calls.is_empty()
        && numbers.styled.is_empty()
        && numbers.components.is_empty()
        && numbers.class_names.is_empty())
        || numbers.css_prop != CssProp::Off
    {
        numbers.visit_program(&program);
        transformations.append(&mut numbers.replacements);
    }
    // An insertion comes before a replacement starting where it does
    transformations.sort_unstable_by_key(|(start, end, _)| (*start, *end));

    // Apply transformations in reverse order to preserve positions
    if transformations.is_empty() {
        return unchanged(numbers.css_prop);
    }

    let edits = transformations
        .iter()
        .map(|(start, end, replacement)| (*start, *end, replacement.len()))
        .collect();
    let mut result = code.to_string();
    for (start, end, replacement) in transformations.into_iter().rev() {
        result.replace_range(start..end, &replacement);
    }

    Aliased {
        code: Cow::Owned(result),
        edits,
        css_prop: numbers.css_prop,
    }
}

/// Reapply Emotion units to literals produced after the initial alias pass.
pub(super) fn pixelify_emotion_values<'a>(
    code: &'a str,
    filename: &str,
    original: &str,
) -> (Cow<'a, str>, Vec<Edit>) {
    let allocator = Allocator::default();
    let source_type = crate::parsing::source_type(filename).unwrap_or_default();
    let original_program = Parser::new(&allocator, original, source_type)
        .parse()
        .program;
    let mut numbers = LibraryNumbers::default();
    for statement in &original_program.body {
        if let Statement::ImportDeclaration(import) = statement
            && import.source.value == "@emotion/css"
        {
            for specifier in import.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(named) = specifier
                    && matches!(
                        named.imported.name().as_str(),
                        "css" | "keyframes" | "injectGlobal"
                    )
                {
                    numbers
                        .calls
                        .push((named.local.name.as_str(), RulesAt::EveryArgument));
                }
            }
        }
    }
    let program = Parser::new(&allocator, code, source_type).parse().program;
    numbers.visit_program(&program);
    numbers.replacements.sort_by_key(|(start, ..)| *start);
    let edits = numbers
        .replacements
        .iter()
        .map(|(start, end, replacement)| (*start, *end, replacement.len()))
        .collect();
    if numbers.replacements.is_empty() {
        return (Cow::Borrowed(code), edits);
    }
    let mut output = code.to_string();
    for (start, end, replacement) in numbers.replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    (Cow::Owned(output), edits)
}

/// The source offset of `offset` in code `edits` made; an offset inside a
/// replacement maps to where the replaced text began
#[must_use]
pub fn source_offset(edits: &[Edit], offset: usize) -> usize {
    let (mut added, mut removed) = (0, 0);
    for &(start, end, length) in edits {
        let replaced_at = start + added - removed;
        if offset < replaced_at {
            break;
        }
        if offset < replaced_at + length {
            return start;
        }
        added += length;
        removed += end - start;
    }
    offset + removed - added
}

/// Pick the name a specifier should import from the target package, or `None` to leave it
/// on its own package. A vanilla-extract stylesheet bypasses the mapping because
/// `execute_vanilla_extract` destructures its mock namespace by vanilla-extract's own names.
fn redirect_target<'n>(
    source: &str,
    imported: &'n str,
    redirect_every_name: bool,
) -> Option<DevupTarget<'n>> {
    if redirect_every_name {
        Some(DevupTarget::Main(imported))
    } else {
        devup_equivalent(source, imported)
    }
}

fn split_target<'n>(target: DevupTarget<'n>, package: &str) -> (&'n str, String) {
    match target {
        DevupTarget::Main(name) => (name, package.to_string()),
        DevupTarget::Compat(name) => (name, format!("{package}/compat")),
    }
}

fn push_redirect(
    redirected: &mut String,
    compat: &mut String,
    target: DevupTarget<'_>,
    local: &str,
) {
    match target {
        DevupTarget::Main(name) => push_specifier(redirected, name, local),
        DevupTarget::Compat(name) => push_specifier(compat, name, local),
    }
}

fn push_specifier(parts: &mut String, imported: &str, local: &str) {
    if !parts.is_empty() {
        parts.push_str(", ");
    }
    parts.push_str(imported);
    if imported != local {
        parts.push_str(" as ");
        parts.push_str(local);
    }
}

/// The logical exported name: `import { 'css' as c }` and `import { css as c }` both
/// export `css`, so the mapping tables match on this.
fn imported_name<'a>(imported: &'a ModuleExportName) -> &'a str {
    match imported {
        ModuleExportName::IdentifierName(id) => id.name.as_str(),
        ModuleExportName::IdentifierReference(id) => id.name.as_str(),
        ModuleExportName::StringLiteral(literal) => literal.value.as_str(),
    }
}

/// The exported name as it must be written back into an import clause; only the
/// rare string-literal export name (`import { "x" as y }`) needs an owned, quoted
/// `String`.
fn exported_text<'a>(imported: &'a ModuleExportName) -> Cow<'a, str> {
    match imported {
        ModuleExportName::StringLiteral(_) => Cow::Owned(imported.to_string()),
        ModuleExportName::IdentifierName(_) | ModuleExportName::IdentifierReference(_) => {
            Cow::Borrowed(imported_name(imported))
        }
    }
}

/// Generate the transformed import statement
fn generate_transformed_import(
    import_decl: &oxc_ast::ast::ImportDeclaration,
    alias: &ImportAlias,
    package: &str,
    redirect_every_name: bool,
) -> String {
    let source = import_decl.source.value.as_str();
    let specifiers = match &import_decl.specifiers {
        Some(specs) => specs,
        None => return format!("import '{package}';"),
    };

    // Classify specifiers in a single pass: capture the first namespace and the
    // first default specifier (first-seen wins, matching the prior find_map
    // semantics). Named specifiers are still iterated separately below.
    let mut namespace = None;
    let mut default_spec = None;
    for s in specifiers {
        match s {
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => {
                if namespace.is_none() {
                    namespace = Some(ns);
                }
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(ds) => {
                if default_spec.is_none() {
                    default_spec = Some(ds);
                }
            }
            ImportDeclarationSpecifier::ImportSpecifier(_) => {}
        }
    }

    if let Some(ns_spec) = namespace {
        let local = ns_spec.local.name.as_str();
        // A `DefaultToNamed` package exports a single callable, so its namespace binding
        // *is* that value — bind it straight to the devup-ui export and member calls such
        // as `Emotion.div` keep resolving, with no dependency left behind.
        if let ImportAlias::DefaultToNamed(named_export) = alias
            && let Some(target) = redirect_target(source, named_export, redirect_every_name)
        {
            let (devup_name, entry) = split_target(target, package);
            let mut parts = String::new();
            push_specifier(&mut parts, devup_name, local);
            return format!("import {{ {parts} }} from '{entry}';");
        }
        // Otherwise the namespace stands for many named exports whose devup-ui
        // counterparts can be renamed (`style` -> `css`), which a namespace access
        // cannot express. Leave it on its own package rather than break the members.
        let target = if redirect_every_name { package } else { source };
        return format!("import * as {local} from '{target}';");
    }

    let mut redirected = String::new();
    let mut compat = String::new();
    let mut retained = String::new();
    let mut types = String::new();
    let mut retained_default = None;

    // Handle default specifier first (at most one in valid JS); only its
    // rendering differs between the alias variants.
    if let Some(default_spec) = default_spec {
        let local_name = default_spec.local.name.as_str();
        match alias {
            // `import foo from 'pkg'` → `import { named as foo } from 'target'`
            ImportAlias::DefaultToNamed(named_export) => {
                match redirect_target(source, named_export, redirect_every_name) {
                    Some(target) => push_redirect(&mut redirected, &mut compat, target, local_name),
                    None => retained_default = Some(local_name),
                }
            }
            // `import foo from 'pkg'` → `import { default as foo } from 'target'`
            ImportAlias::NamedToNamed => {
                if redirect_every_name {
                    push_redirect(
                        &mut redirected,
                        &mut compat,
                        DevupTarget::Main("default"),
                        local_name,
                    );
                } else {
                    retained_default = Some(local_name);
                }
            }
        }
    }

    // Handle named specifiers (kept as-is for both variants)
    for specifier in specifiers {
        if let ImportDeclarationSpecifier::ImportSpecifier(spec) = specifier {
            let local = spec.local.name.as_str();
            let written = exported_text(&spec.imported);
            // Types are erased, so they stay on the package without making it a
            // runtime dependency
            if spec.import_kind.is_type() {
                push_specifier(&mut types, &written, local);
                continue;
            }
            // A stylesheet passes every export through as written
            let name = if redirect_every_name {
                written.as_ref()
            } else {
                imported_name(&spec.imported)
            };
            match redirect_target(source, name, redirect_every_name) {
                Some(target) => push_redirect(&mut redirected, &mut compat, target, local),
                None => push_specifier(&mut retained, &written, local),
            }
        }
    }

    let mut result = String::new();
    for (parts, entry) in [
        (&redirected, package.to_string()),
        (&compat, format!("{package}/compat")),
    ] {
        if parts.is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str("import { ");
        result.push_str(parts);
        result.push_str(" } from '");
        result.push_str(&entry);
        result.push_str("';");
    }
    if retained_default.is_some() || !retained.is_empty() {
        eprintln!(
            "[devup-ui] WARNING: '{source}' keeps {} because devup-ui has no equivalent export, so the package stays a runtime dependency.",
            retained_default.map_or_else(|| retained.clone(), ToString::to_string)
        );
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str("import ");
        if let Some(local) = retained_default {
            result.push_str(local);
            if !retained.is_empty() {
                result.push_str(", ");
            }
        }
        if !retained.is_empty() {
            result.push_str("{ ");
            result.push_str(&retained);
            result.push_str(" }");
        }
        result.push_str(" from '");
        result.push_str(source);
        result.push_str("';");
    }
    if !types.is_empty() {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str("import type { ");
        result.push_str(&types);
        result.push_str(" } from '");
        result.push_str(source);
        result.push_str("';");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use oxc_ast::builder::AstBuilder;
    use oxc_span::SPAN;

    fn emotion_alias() -> HashMap<String, ImportAlias> {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases
    }

    fn vanilla_extract_alias() -> HashMap<String, ImportAlias> {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );
        aliases
    }

    #[test]
    fn test_source_offset() {
        // `ab` at 2 became 5 bytes and `cdef` at 10 became 1 byte
        let edits = [(2, 4, 5), (10, 14, 1)];
        for (offset, source) in [
            (0, 0),
            (1, 1),
            (2, 2),
            (6, 2),
            (7, 4),
            (12, 9),
            (13, 10),
            (14, 14),
            (20, 20),
        ] {
            assert_eq!(source_offset(&edits, offset), source, "{offset}");
        }
        assert_eq!(source_offset(&[], 7), 7);
    }

    fn styled_components_alias() -> HashMap<String, ImportAlias> {
        let mut aliases = HashMap::new();
        aliases.insert(
            "styled-components".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases
    }

    fn combined_aliases() -> HashMap<String, ImportAlias> {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );
        aliases
    }

    #[test]
    fn test_default_to_named_same_name() {
        assert_snapshot!(transform_import_aliases(
            r"import styled from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_different_name() {
        assert_snapshot!(transform_import_aliases(
            r"import styledA from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_named_to_named() {
        assert_snapshot!(transform_import_aliases(
            r"import { style, globalStyle } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_default_specifier_in_vanilla_extract_stylesheet() {
        assert_snapshot!(transform_import_aliases(
            r"import veDefault from '@vanilla-extract/css'",
            "styles.css.ts",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_default_and_named_both_retained_on_source() {
        assert_snapshot!(transform_import_aliases(
            r"import veDefault, { styleVariants } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_default_export_without_devup_equivalent_stays_on_source() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "some-lib".to_string(),
            ImportAlias::DefaultToNamed("someUnmappedExport".to_string()),
        );
        assert_snapshot!(transform_import_aliases(
            r"import sheet from 'some-lib'",
            "test.tsx",
            "@devup-ui/react",
            &aliases
        ));
    }

    #[test]
    fn test_namespace_import_of_a_compat_only_default() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "some-lib".to_string(),
            ImportAlias::DefaultToNamed("ThemeProvider".to_string()),
        );
        assert_snapshot!(transform_import_aliases(
            r"import * as Sheet from 'some-lib'",
            "test.tsx",
            "@devup-ui/react",
            &aliases
        ));
    }

    #[test]
    fn test_vanilla_extract_names_map_onto_devup_equivalents() {
        assert_snapshot!(transform_import_aliases(
            r"import { style, globalStyle, styleVariants } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_named_imports_without_devup_export_stay_on_source() {
        assert_snapshot!(transform_import_aliases(
            r"import styled, { css, keyframes, createGlobalStyle, ThemeProvider } from 'styled-components'",
            "test.tsx",
            "@devup-ui/react",
            &styled_components_alias()
        ));
    }

    // A property's array of values declares fallbacks in these libraries
    #[test]
    fn test_property_arrays_become_fallback_declarations() {
        let code = transform_import_aliases(
            "import { css } from '@emotion/react'
css({ background: ['red', 'linear-gradient(red, blue)'], WebkitBoxShadow: ['none', '0 0 1px red'], margin: [4, 0], zIndex: [1, 2], color: [x, 'red'], top: ['1px'] })",
            "test.tsx",
            "@devup-ui/react",
            &HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        );
        for expected in [
            r#"background: "red;background:linear-gradient(red, blue)""#,
            r#"WebkitBoxShadow: "none;-webkit-box-shadow:0 0 1px red""#,
            r#"margin: "4px;margin:0""#,
            r#"zIndex: "1;z-index:2""#,
            "color: [x, 'red']",
            "top: ['1px']",
        ] {
            assert!(code.contains(expected), "{expected}\n{code}");
        }
        assert_eq!(kebab_property("MozAppearance"), "-moz-appearance");
        assert_eq!(kebab_property("msFlex"), "ms-flex");
    }

    // Types are erased, so a type-only import never becomes a runtime import
    #[test]
    fn test_type_only_imports_stay_type_only() {
        let code = transform_import_aliases(
            "import styled, { type DefaultTheme, ThemeProvider, type CSSObject as O } from 'styled-components'\nimport type { CSSProperties } from 'styled-components'",
            "test.tsx",
            "@devup-ui/react",
            &styled_components_alias(),
        );
        for expected in [
            "import type { DefaultTheme, CSSObject as O } from 'styled-components';",
            "import type { CSSProperties } from 'styled-components'",
            "import { ThemeProvider } from '@devup-ui/react/compat';",
        ] {
            assert!(code.contains(expected), "{expected}\n{code}");
        }
        assert!(!code.contains("import { DefaultTheme"), "{code}");
    }

    #[test]
    fn test_styled_components_theme_apis_redirect_to_compat() {
        let code = transform_import_aliases(
            "import { ThemeConsumer, createTheme } from 'styled-components'",
            "test.tsx",
            "@devup-ui/react",
            &styled_components_alias(),
        );
        assert!(
            code.contains("import { ThemeConsumer, createTheme } from '@devup-ui/react/compat';"),
            "{code}"
        );
        assert!(!code.contains("from 'styled-components'"), "{code}");
    }

    #[test]
    fn test_vanilla_extract_companions_are_reported() {
        let errors = companion_errors(
            "import type { RecipeVariants } from '@vanilla-extract/recipes'
import { recipe } from '@vanilla-extract/recipes'
import { style } from '@vanilla-extract/css'
import { createSprinkles } from '@vanilla-extract/sprinkles'",
            "a.css.ts",
        );
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(
            errors[0]
                .1
                .starts_with("`@vanilla-extract/recipes` generates CSS")
        );
        assert!(
            errors[1]
                .1
                .starts_with("`@vanilla-extract/sprinkles` generates CSS")
        );
        assert_eq!(
            companion_errors("import { style } from '@vanilla-extract/css'", "a.css.ts"),
            vec![]
        );
    }

    #[test]
    fn test_emotion_cache_provider_redirects_to_compat() {
        let code = transform_import_aliases(
            "import { CacheProvider, Global } from '@emotion/react'",
            "test.tsx",
            "@devup-ui/react",
            &HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        );
        assert!(
            code.contains("import { CacheProvider, Global } from '@devup-ui/react/compat';"),
            "{code}"
        );
        assert!(!code.contains("from '@emotion/react'"), "{code}");
    }

    #[test]
    fn test_emotion_css_maps_onto_devup_equivalents() {
        let code = transform_import_aliases(
            "import { css, cx, keyframes, injectGlobal, merge, flush } from '@emotion/css'",
            "test.tsx",
            "@devup-ui/react",
            &HashMap::from([("@emotion/css".to_string(), ImportAlias::NamedToNamed)]),
        );
        assert!(
            code.contains(
                "import { css, keyframes, globalCss as injectGlobal } from '@devup-ui/react';"
            ),
            "{code}"
        );
        assert!(
            code.contains("import { cx, merge } from '@devup-ui/react/compat';"),
            "{code}"
        );
        assert!(
            code.contains("import { flush } from '@emotion/css';"),
            "{code}"
        );
    }

    #[test]
    fn test_no_matching_alias() {
        assert_snapshot!(transform_import_aliases(
            r"import { useState } from 'react'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_empty_aliases() {
        assert_snapshot!(transform_import_aliases(
            r"import styled from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &HashMap::new()
        ));
    }

    #[test]
    fn test_styled_components() {
        assert_snapshot!(transform_import_aliases(
            r"import styled from 'styled-components'",
            "test.tsx",
            "@devup-ui/react",
            &styled_components_alias()
        ));
    }

    #[test]
    fn test_css_ts_file_vanilla_extract() {
        assert_snapshot!(transform_import_aliases(
            r"import { style } from '@vanilla-extract/css'
export const container = style({ background: 'red' })",
            "styles.css.ts",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_emotion_numbers_become_px() {
        let code = r"import styled from '@emotion/styled'
import { Global, ThemeProvider } from '@emotion/react'
foo.bar.baz({ top: 1 })
make()()({ top: 1 })
;(() => 1)()
styled.div({ top: 1, p: 2 })
styled.div(cond && { top: 1 }, (flag ? { left: 2 } : { right: 3 }), { bottom: cond ? 4 : (5), left: (cond ? 7 : 8), vars: flag && { x: 6 } })
export const A = () => <><div styles={{ top: 1 }} /><Global {...props} styles={{ top: 1 }} key={1} /></>";
        let mut aliases = combined_aliases();
        aliases.insert("@emotion/react".to_string(), ImportAlias::NamedToNamed);
        assert_eq!(
            transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases),
            r#"import { styled } from '@devup-ui/react';
import { Global, ThemeProvider } from '@devup-ui/react/compat';
foo.bar.baz({ top: 1 })
make()()({ top: 1 })
;(() => 1)()
styled.div({ top: "1px", p: 2 })
styled.div(cond && { top: "1px" }, (flag ? { left: "2px" } : { right: "3px" }), { bottom: cond ? "4px" : "5px", left: (cond ? "7px" : "8px"), vars: flag && { x: 6 } })
export const A = () => <><div styles={{ top: 1 }} /><Global {...props} styles={{ top: "1px" }} key={1} /></>"#
        );
    }

    #[test]
    fn test_vanilla_extract_numbers_become_px_outside_stylesheets() {
        let code = r"import { style as s, globalStyle, keyframes, styleVariants } from '@vanilla-extract/css'
export const a = s({ padding: 8, top: -2, left: (1.5), width: 0, lineHeight: 1.5, vars: { '--x': 4 }, [key]: 5, ...rest, selectors: { '&:hover': { margin: 4 } } })
export const b = s([a, { right: 3 }], 'debug')
globalStyle('body', { margin: 2 })
keyframes({ from: { width: 10, opacity: 0 }, '50%': { width: 20 } })
styleVariants({ small: { padding: 1 } })
other({ padding: 8 })
s()";
        assert_eq!(
            transform_import_aliases(
                code,
                "test.tsx",
                "@devup-ui/react",
                &vanilla_extract_alias()
            ),
            r#"import { css as s, globalCss as globalStyle, keyframes } from '@devup-ui/react'; import { styleVariants } from '@vanilla-extract/css';
export const a = s({ padding: "8px", top: "-2px", left: "1.5px", width: 0, lineHeight: 1.5, vars: { '--x': 4 }, [key]: 5, ...rest, selectors: { '&:hover': { margin: "4px" } } })
export const b = s([a, { right: "3px" }], 'debug')
globalStyle('body', { margin: "2px" })
keyframes({ from: { width: "10px", opacity: 0 }, '50%': { width: "20px" } })
styleVariants({ small: { padding: 1 } })
other({ padding: 8 })
s()"#
        );
        let stylesheet =
            "import { style } from '@vanilla-extract/css'\nexport const a = style({ padding: 8 })";
        assert!(
            transform_import_aliases(
                stylesheet,
                "a.css.ts",
                "@devup-ui/react",
                &vanilla_extract_alias()
            )
            .ends_with("style({ padding: 8 })")
        );
    }

    #[test]
    fn test_multiple_imports_same_file() {
        assert_snapshot!(transform_import_aliases(
            r"import styled from '@emotion/styled'
import { style } from '@vanilla-extract/css'
import { useState } from 'react'",
            "test.tsx",
            "@devup-ui/react",
            &combined_aliases()
        ));
    }

    #[test]
    fn test_preserves_code_after_import() {
        assert_snapshot!(transform_import_aliases(
            r"import { style } from '@vanilla-extract/css'

export const button = style({
    background: 'blue',
    padding: '8px',
});",
            "test.css.ts",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_named_import_with_alias() {
        assert_snapshot!(transform_import_aliases(
            r"import { style as myStyle } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_side_effect_import_no_specifiers() {
        assert_snapshot!(transform_import_aliases(
            r"import '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_alias_in_comment_not_transformed() {
        assert_snapshot!(transform_import_aliases(
            r"// This uses @emotion/styled but doesn't import it
const x = 1;",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_with_additional_named_imports() {
        assert_snapshot!(transform_import_aliases(
            r"import styled, { css, keyframes } from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_with_aliased_named_import() {
        assert_snapshot!(transform_import_aliases(
            r"import styled, { css as emotionCss } from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_namespace_import() {
        assert_snapshot!(transform_import_aliases(
            r"import * as Emotion from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_named_to_named_with_default_specifier() {
        assert_snapshot!(transform_import_aliases(
            r"import vanillaDefault, { style } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_named_to_named_namespace_import() {
        assert_snapshot!(transform_import_aliases(
            r"import * as VE from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_identifier_reference_imported_name_with_and_without_alias() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);

        for (code, local, expected) in [
            (
                "import { imported } from 'source'",
                "imported",
                "import { imported } from '@devup-ui/react';",
            ),
            (
                "import { imported as local } from 'source'",
                "local",
                "import { imported as local } from '@devup-ui/react';",
            ),
        ] {
            let mut parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
            let oxc_ast::ast::Statement::ImportDeclaration(import_decl) =
                &mut parsed.program.body[0]
            else {
                panic!("expected import declaration");
            };
            let Some(specifiers) = import_decl.specifiers.as_mut() else {
                panic!("expected import specifiers");
            };
            let ImportDeclarationSpecifier::ImportSpecifier(spec) = &mut specifiers[0] else {
                panic!("expected import specifier");
            };
            spec.imported = ModuleExportName::new_identifier_reference(SPAN, "imported", &builder);
            assert_eq!(spec.local.name.as_str(), local);
            assert_eq!(
                generate_transformed_import(
                    import_decl,
                    &ImportAlias::NamedToNamed,
                    "@devup-ui/react",
                    true
                ),
                expected
            );
        }
    }

    #[test]
    fn test_string_literal_imported_name_with_and_without_alias() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);

        for (code, expected) in [
            (
                "import { 'imported' as local } from 'source'",
                "import { \"imported\" as local } from '@devup-ui/react';",
            ),
            (
                "import { imported } from 'source'",
                "import { \"imported\" } from '@devup-ui/react';",
            ),
        ] {
            let mut parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
            let oxc_ast::ast::Statement::ImportDeclaration(import_decl) =
                &mut parsed.program.body[0]
            else {
                panic!("expected import declaration");
            };
            let Some(specifiers) = import_decl.specifiers.as_mut() else {
                panic!("expected import specifiers");
            };
            let ImportDeclarationSpecifier::ImportSpecifier(spec) = &mut specifiers[0] else {
                panic!("expected import specifier");
            };
            if spec.local.name == "imported" {
                spec.imported =
                    ModuleExportName::new_string_literal(SPAN, "imported", None, &builder);
                spec.local.name = "\"imported\"".into();
            }
            assert_eq!(
                generate_transformed_import(
                    import_decl,
                    &ImportAlias::NamedToNamed,
                    "@devup-ui/react",
                    true
                ),
                expected
            );
        }
    }

    #[test]
    fn test_css_prop_numbers_runtimes_and_pragmas() {
        let mut aliases = emotion_alias();
        aliases.insert(EMOTION_REACT.to_string(), ImportAlias::NamedToNamed);
        let outputs: Vec<(String, CssProp)> = [
            "export const a = <div css={{ padding: 8, lineHeight: 2, '&:hover': { margin: 4 } }} />;",
            "export const a = <div css={[{ top: 1 }, c ? { left: 2 } : { right: 3 }, c && { bottom: 4 }, d || { width: 5 }, ({ height: 6 })]} />;",
            "export const a = <div css={[t => ({ top: 1 }), t => { return { left: 2 }; }, t => { f(); return { right: 3 }; }, function (t) { return { bottom: 4 }; }, function (t) { f(); }]} />;",
            "import { Box } from '@devup-ui/react';\nexport const a = <><Box p={2} css={{ padding: 2 }} /><Custom css={{ padding: 2 }} /></>;",
            "import styled from '@emotion/styled';\nexport const a = <Custom css={{ padding: 2 }} />;",
            "/** @jsxImportSource @emotion/react */\nexport const a = <Custom css={{ padding: 2 }} />;",
            "import { jsx as _jsx } from '@emotion/react/jsx-runtime';\nimport { jsxDEV } from '@emotion/react/jsx-dev-runtime';\nexport const a = [_jsx('div', { css: { padding: 2 }, id: 1, ['css']: { top: 1 } }), jsxDEV(Custom, { css: { top: 3 } }), _jsx('div', props), f('div', { css: { top: 4 } })];",
            "import { jsx, createElement } from '@emotion/react';\nexport const a = [jsx('div', { css: { padding: 2 } }), createElement('div', null)];",
            "import { jsx as _jsx } from 'react/jsx-runtime';\nexport const a = [_jsx('div', { css: { padding: 2 } }), _jsx(Custom, { css: { padding: 2 } })];",
            "export const a = { css: 1 };",
        ]
        .iter()
        .map(|code| {
            let aliased =
                transform_import_aliases_with_edits(code, "test.tsx", "@devup-ui/react", &aliases);
            (aliased.code.into_owned(), aliased.css_prop)
        })
        .collect();
        insta::assert_debug_snapshot!(outputs);

        let without_css_prop = transform_import_aliases_with_edits(
            "export const a = <div css={{ padding: 8 }} />;",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias(),
        );
        assert_eq!(without_css_prop.css_prop, CssProp::Off);
        assert!(matches!(without_css_prop.code, Cow::Borrowed(_)));
    }

    #[test]
    fn test_project_building_jsx_with_emotion() {
        let mut aliases = emotion_alias();
        aliases.insert(EMOTION_REACT.to_string(), ImportAlias::NamedToNamed);
        aliases.insert(
            crate::css_prop::EMOTION_JSX_RUNTIME.to_string(),
            ImportAlias::NamedToNamed,
        );
        let outputs: Vec<(String, CssProp, Vec<Edit>)> = [
            ("test.tsx", "export const a = <Custom css={{ padding: 2 }} />;"),
            (
                "test.jsx",
                "import styled from '@emotion/styled';\nexport const a = <div />;",
            ),
            (
                "test.tsx",
                "/** @jsxImportSource @emotion/react */\nexport const a = <div />;",
            ),
            ("test.tsx", "/** @jsx h */\nexport const a = <div />;"),
            ("test.ts", "export const a = 1;"),
            (
                "test.ts",
                "import { jsx as _jsx } from '@emotion/react/jsx-runtime';\nexport const a = _jsx(Custom, { css: { top: 1 } });",
            ),
        ]
        .iter()
        .map(|(filename, code)| {
            let aliased =
                transform_import_aliases_with_edits(code, filename, "@devup-ui/react", &aliases);
            (aliased.code.into_owned(), aliased.css_prop, aliased.edits)
        })
        .collect();
        insta::assert_debug_snapshot!(outputs);

        let runtime_alone = transform_import_aliases_with_edits(
            "export const a = <div />;",
            "test.tsx",
            "@devup-ui/react",
            &HashMap::from([(
                crate::css_prop::EMOTION_JSX_RUNTIME.to_string(),
                ImportAlias::NamedToNamed,
            )]),
        );
        assert_eq!(runtime_alone.css_prop, CssProp::Off);
        assert!(matches!(runtime_alone.code, Cow::Borrowed(_)));
    }
}
