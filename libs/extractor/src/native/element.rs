//! Devup UI elements as React Native elements: the style props of each
//! element leave it for one `style`, and the element takes the React Native
//! component its import is now bound to.

use oxc_ast::ast::{
    ArrayExpression, ArrayExpressionElement, ArrowFunctionExpression, CallExpression,
    ExportDefaultDeclaration, Expression, Function, FunctionBody, IdentifierReference,
    JSXAttribute, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild, JSXElement,
    JSXElementName, JSXExpression, LogicalOperator, ObjectExpression, ObjectPropertyKind, Program,
    TSType, TaggedTemplateExpression, UnaryOperator, VariableDeclarator,
};
use std::borrow::Cow;
use std::cell::Cell;
use std::fmt::Write;

use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::errors::{Problem, RuntimeFeature};
use super::imports::{self, native_component};
use super::props::{Declaration, Tokens, declarations, dynamic_keys, length_keys, literal_levels};
use super::sheet::Sheet;
use super::style::{
    Choice, INTERACTIONS, READS, Read, Reads, State, Vars, cascade, in_theme, interaction_keys,
    style_code, varied, when, without,
};
use super::theme::{DEFAULT_BREAKPOINTS, selector_theme};
use super::values::{
    Literal, NativeValue, ROOT_FONT_SIZE, TIMED, Viewport, number_code, viewport_length,
};
use super::{NativePlatform, NativeTarget, REACT_NATIVE, RUNTIME, Replacement};
use crate::component::ExportVariableKind;
use crate::extract_style::constant::is_maintain_value_property;
use crate::fresh_name::fresh_name;

mod animation;
mod css_call;
mod styled;

use animation::Animated;

use styled::is_function;

/// The changes compiling a file makes, or why it cannot compile
pub(super) struct Compiled {
    pub replacements: Vec<Replacement>,
    pub errors: Vec<(u32, String)>,
}

/// The names the file gives what it imports from the runtime, and whether
/// it uses each
struct Runtime {
    /// The variable holding the index of the theme the app shows
    index: String,
    /// The variable holding the breakpoint the window is at
    breakpoint: String,
    /// The variable holding whether the window is in landscape
    landscape: String,
    /// The variable holding whether the system reduces motion
    motion: String,
    /// The variables holding the window's width and height
    width: String,
    height: String,
    /// The variable holding whether the element is pressed, in the function
    /// `Pressable` calls for its style
    pressed: String,
    /// The variables holding whether the pointer is over the element and
    /// whether it has focus
    hovered: String,
    focused: String,
    /// The component telling a `TextInput` whether it has focus
    focused_wrapper: String,
    focused_used: bool,
    /// The hook giving the theme of a compat `ThemeProvider`
    styled_theme: String,
    styled_theme_used: bool,
    /// The hooks reading each, by what they read
    hooks: [String; READS],
    /// What the hooks the file calls read
    hooks_used: Reads,
    /// The component that reads them itself, for an element outside a
    /// function component
    wrapper: Import,
    /// The one that also reads the window's size, for such an element taking
    /// a viewport length
    sized_wrapper: Import,
    /// What the elements rendered through it read
    wrapper_reads: Reads,
    /// The function telling the runtime the theme names
    define: String,
    /// The function telling the runtime the breakpoints
    define_breakpoints: String,
}

/// A name the file imports from the runtime under, and whether it uses it
struct Import {
    name: String,
    used: bool,
}

impl Import {
    fn new(name: &str, code: &str) -> Self {
        Self {
            name: fresh_name(name, code),
            used: false,
        }
    }

    /// The name, which the file now uses
    fn use_name(&mut self) -> String {
        self.used = true;
        self.name.clone()
    }
}

/// The runtime hooks, in the order they are called, by the name they are
/// exported under: they read what each `Read` names
const HOOKS: [&str; READS] = [
    "useThemeIndex",
    "useBreakpoint",
    "useLandscape",
    "useReducedMotion",
    "useWindowWidth",
    "useWindowHeight",
];

impl Runtime {
    fn new(code: &str) -> Self {
        Self {
            index: fresh_name("__t", code),
            breakpoint: fresh_name("__b", code),
            landscape: fresh_name("__l", code),
            motion: fresh_name("__m", code),
            width: fresh_name("__vw", code),
            height: fresh_name("__vh", code),
            pressed: fresh_name("__p", code),
            hovered: fresh_name("__h", code),
            focused: fresh_name("__f", code),
            focused_wrapper: fresh_name("__Focused", code),
            focused_used: false,
            styled_theme: fresh_name("__useStyledTheme", code),
            styled_theme_used: false,
            hooks: HOOKS.map(|hook| fresh_name(&format!("__{hook}"), code)),
            hooks_used: Reads::default(),
            wrapper: Import::new("__Themed", code),
            sized_wrapper: Import::new("__Sized", code),
            wrapper_reads: Reads::default(),
            define: fresh_name("__defineThemes", code),
            define_breakpoints: fresh_name("__defineBreakpoints", code),
        }
    }

    /// The variables holding what the runtime tells
    fn vars(&self) -> Vars<'_> {
        Vars {
            theme: &self.index,
            breakpoint: &self.breakpoint,
            landscape: &self.landscape,
            motion: &self.motion,
            pressed: &self.pressed,
            hovered: &self.hovered,
            focused: &self.focused,
            disabled: "false",
        }
    }

    /// The variables of `reads`, in the order of the hooks
    fn variables(&self, reads: Reads) -> impl Iterator<Item = (usize, &str)> {
        self.runtime_variables()
            .into_iter()
            .enumerate()
            .filter(move |(at, _)| reads.flags()[*at])
            .map(|(at, name)| (at, name.as_str()))
    }

    /// The variables holding what the runtime tells, in the order of the
    /// hooks
    const fn runtime_variables(&self) -> [&String; READS] {
        [
            &self.index,
            &self.breakpoint,
            &self.landscape,
            &self.motion,
            &self.width,
            &self.height,
        ]
    }

    /// The import of the runtime and the calls telling it the theme names
    /// and the breakpoints, when the file uses it
    fn prelude(&self, target: &NativeTarget, theme_api: bool) -> Option<String> {
        if !(self.hooks_used.any()
            || self.wrapper.used
            || self.sized_wrapper.used
            || self.focused_used
            || self.styled_theme_used
            || theme_api)
        {
            return None;
        }
        let reads = self.hooks_used.union(self.wrapper_reads);
        let mut specifiers: Vec<String> = self
            .variables(self.hooks_used)
            .map(|(at, _)| format!("{} as {}", HOOKS[at], self.hooks[at]))
            .collect();
        for (export, import) in [("Themed", &self.wrapper), ("Sized", &self.sized_wrapper)] {
            if import.used {
                specifiers.push(format!("{export} as {}", import.name));
            }
        }
        if self.styled_theme_used {
            specifiers.push(format!("useStyledTheme as {}", self.styled_theme));
        }
        if self.focused_used {
            specifiers.push(format!("Focused as {}", self.focused_wrapper));
        }
        let theme = &target.theme;
        let mut calls = String::new();
        if reads.has(Read::Theme) || theme_api {
            specifiers.push(format!("defineThemes as {}", self.define));
            let names = serde_json::Value::from(theme.names.clone()).to_string();
            let dark = theme
                .dark
                .map_or_else(String::new, |dark| format!(", {dark}"));
            let _ = writeln!(calls, "{}({names}{dark});", self.define);
        }
        if reads.has(Read::Breakpoint) && theme.breakpoints != DEFAULT_BREAKPOINTS {
            specifiers.push(format!("defineBreakpoints as {}", self.define_breakpoints));
            let breakpoints = serde_json::Value::from(theme.breakpoints.clone()).to_string();
            let _ = writeln!(calls, "{}({breakpoints});", self.define_breakpoints);
        }
        Some(format!(
            "import {{ {} }} from \"{RUNTIME}\";\n{calls}",
            specifiers.join(", ")
        ))
    }
}

/// A function the elements are in
struct Frame {
    /// Whether it is a function component or a hook, where the hook reading
    /// the theme can be called first
    component: bool,
    /// What the elements in it read from the runtime
    reads: Reads,
    /// Where the hook is called
    hook_at: HookAt,
}

/// Where the hook reading the theme is called in a function
#[derive(Clone, Copy)]
enum HookAt {
    /// First in a body of statements, at this offset
    Body(usize),
    /// Before the expression an arrow function returns, between these offsets
    Expression(usize, usize),
}

/// Compile the Devup UI elements of `program`
pub(super) fn compile(
    program: &Program<'_>,
    code: &str,
    package: &str,
    target: &NativeTarget,
    scoping: &Scoping,
) -> Compiled {
    let imports = imports::read(program, code, package, target);
    let mut visitor = Elements {
        code,
        target,
        scoping,
        components: &imports.components,
        css: &imports.css,
        styled: &imports.styled,
        compat_bindings: &imports.compat,
        compat: false,
        props_called: Cell::new(false),
        natives: Vec::new(),
        keyframes: &imports.keyframes,
        animation: None,
        animated: Animated::new(code),
        props: fresh_name("__props", code),
        in_styled: false,
        sheet: Sheet::new(fresh_name("__s", code)),
        units: fresh_name("__u", code),
        units_used: Cell::new(false),
        pressable: fresh_name("__Pressable", code),
        pressable_used: false,
        press: None,
        runtime: Runtime::new(code),
        theme_scope: None,
        frames: Vec::new(),
        component_functions: FxHashSet::default(),
        replacements: imports.replacements,
        errors: imports.errors,
    };
    if !(visitor.components.is_empty()
        && visitor.css.is_empty()
        && visitor.styled.is_empty()
        && visitor.keyframes.is_empty())
    {
        visitor.visit_program(program);
    }
    let Elements {
        sheet,
        units,
        units_used,
        pressable,
        pressable_used,
        runtime,
        animated,
        natives,
        mut replacements,
        errors,
        ..
    } = visitor;
    let mut prelude = runtime
        .prelude(target, imports.theme_api)
        .unwrap_or_default();
    if pressable_used {
        let _ = writeln!(
            prelude,
            "import {{ Pressable as {pressable} }} from \"{REACT_NATIVE}\";"
        );
    }
    if !natives.is_empty() {
        let specifiers: Vec<String> = natives
            .iter()
            .map(|(name, local)| format!("{name} as {local}"))
            .collect();
        let _ = writeln!(
            prelude,
            "import {{ {} }} from \"{REACT_NATIVE}\";",
            specifiers.join(", ")
        );
    }
    prelude.push_str(&animated.prelude());
    if !sheet.is_empty() {
        prelude.push_str(&sheet_prelude(&sheet, code));
    }
    if units_used.get() {
        prelude.push_str(&units_declaration(&units));
    }
    if !prelude.is_empty() {
        let at = imports.sheet_at.unwrap_or(code.len());
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
    /// `className`, the styles `css()` gives
    ClassName,
    /// A style prop
    Declares,
    /// `_ios`, `_android` or `_web`: styles for one platform
    Platform(NativePlatform),
    /// `_themeDark` and the like: styles for the theme named
    Theme(String),
    /// `_landscape`, `_motionReduce`, `_pressed` and the like: styles
    /// applied while the window is in that orientation, the system reduces
    /// motion or the element is pressed
    When(State),
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
        "_portrait" => Prop::When(State::Landscape(false)),
        "_landscape" => Prop::When(State::Landscape(true)),
        "_motionReduce" => Prop::When(State::ReducedMotion(true)),
        "_motionSafe" => Prop::When(State::ReducedMotion(false)),
        "_pressed" | "_active" => Prop::When(State::Pressed),
        "_hover" => Prop::When(State::Hovered),
        "_focus" => Prop::When(State::Focused),
        "_disabled" => Prop::When(State::Disabled),
        "className" => Prop::ClassName,
        "typography" => Prop::Declares,
        "styleOrder" | "styleVars" => Prop::Unsupported(Problem::new(
            "React Native has no stylesheet order and no CSS variables",
            "remove it; on React Native the later style in `style` wins",
        )),
        name if name.starts_with("_theme") && name.len() > "_theme".len() => {
            Prop::Theme(selector_theme(name))
        }
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
    target: &'s NativeTarget,
    scoping: &'s Scoping,
    components: &'s FxHashMap<SymbolId, (ExportVariableKind, String)>,
    /// The bindings of `css`
    css: &'s FxHashSet<SymbolId>,
    /// The bindings of `styled`
    styled: &'s FxHashSet<SymbolId>,
    /// The bindings of `styled` and `css` imported from a compat package
    compat_bindings: &'s FxHashSet<SymbolId>,
    /// Whether the styles read are a compat package's, whose numbers are
    /// pixels as React Native reads them
    compat: bool,
    /// Whether a function was called with `props`, the props of the
    /// `styled()` component being read
    props_called: Cell<bool>,
    /// The React Native components `styled.Name` renders: each name and the
    /// local name it is imported under
    natives: Vec<(String, String)>,
    /// The bindings of `keyframes`
    keyframes: &'s FxHashSet<SymbolId>,
    /// The offset and the name of the first animation or transition of the
    /// element read
    animation: Option<(u32, String)>,
    animated: Animated,
    /// The parameter holding the props of a component `styled()` gives
    props: String,
    /// Whether the styles of a `styled()` component are being read, where a
    /// function is called with its props
    in_styled: bool,
    sheet: Sheet,
    /// The name of the function converting values read at runtime
    units: String,
    /// Whether a value read at runtime is converted
    units_used: Cell<bool>,
    /// The name React Native's `Pressable` is imported under, for an element
    /// that is a `View` but takes a press style
    pressable: String,
    pressable_used: bool,
    /// The offset and the name of the first press style of the element read
    press: Option<(u32, String)>,
    runtime: Runtime,
    /// The theme of the `_theme*` block being read, whose tokens resolve in
    /// that theme alone
    theme_scope: Option<usize>,
    /// The functions the visited code is in, innermost last
    frames: Vec<Frame>,
    /// Where each function a component or a hook is bound to starts
    component_functions: FxHashSet<u32>,
    replacements: Vec<Replacement>,
    errors: Vec<(u32, String)>,
}

/// What the props of one element give its `style`
#[derive(Default)]
struct Styles {
    /// The code of what `className` gives, before the style props
    classes: Vec<String>,
    choices: Vec<Choice>,
    /// The styles of the theme, orientation and press blocks, which apply
    /// after the other styles whatever the order of the props, as their
    /// selectors and media queries do on the web
    blocks: Vec<Choice>,
    /// The code of each style given whole, in prop order
    given: Vec<String>,
    /// The span of each prop leaving the element once it takes `style`
    removed: Vec<Span>,
    /// The `style` prop, which leaves the element with the style props
    style: Option<Span>,
    /// The code of the element's `disabled` prop, which `_disabled` reads
    disabled: Option<String>,
    /// Whether `Focused` renders the element, which tells whether it has
    /// focus: a `TextInput` taking a focus style
    focus_wrapped: bool,
    /// The `style` prop's value compiled, which it keeps when the element
    /// takes no other style
    kept: Option<Replacement>,
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
        } else if self.is_css_reference(identifier)
            || self.is_styled_reference(identifier)
            || self.is_keyframes_reference(identifier)
        {
            self.errors.push((
                identifier.span.start,
                format!(
                    "`{}` can only be called on React Native: the build compiles each call into styles. Fix: call it where the styles are used",
                    identifier.name
                ),
            ));
        }
    }

    fn visit_ts_type(&mut self, _: &TSType<'a>) {}

    fn visit_tagged_template_expression(&mut self, template: &TaggedTemplateExpression<'a>) {
        if self.is_css(&template.tag) {
            self.css_template(template);
        } else if let Some(target) = self.styled_target(&template.tag) {
            self.styled_template(template, target);
        } else {
            walk::walk_tagged_template_expression(self, template);
        }
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if self.is_css(&call.callee) {
            self.css_call(call);
        } else if let Some(target) = self.styled_target(&call.callee) {
            self.styled_call(call, target);
        } else if self.is_styled(&call.callee) {
            self.styled_direct(call);
        } else if self.is_keyframes(&call.callee) {
            self.keyframes_call(call);
        } else {
            walk::walk_call_expression(self, call);
        }
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let (Some(name), Some(init)) = (declarator.id.get_identifier_name(), &declarator.init)
            && is_component_name(&name)
        {
            self.component_functions.extend(function_start(init));
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_export_default_declaration(&mut self, declaration: &ExportDefaultDeclaration<'a>) {
        let start = declaration.declaration.as_expression().map_or_else(
            || Some(declaration.declaration.span().start),
            function_start,
        );
        self.component_functions.extend(start);
        walk::walk_export_default_declaration(self, declaration);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        let component = function
            .id
            .as_ref()
            .is_some_and(|id| is_component_name(&id.name))
            || self.component_functions.contains(&function.span.start);
        let body = function.body.as_deref().map(body_start);
        self.in_frame(component, body, |visitor| {
            walk::walk_function(visitor, function, flags);
        });
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let component = self.component_functions.contains(&arrow.span.start);
        let hook_at = arrow.get_expression().map_or_else(
            || arrow.get_function_body().map(body_start),
            |expression| {
                let span = expression.span();
                Some(HookAt::Expression(span.start as usize, span.end as usize))
            },
        );
        self.in_frame(component, hook_at, |visitor| {
            walk::walk_arrow_function_expression(visitor, arrow);
        });
    }
}

/// Where the hook is called first in `body`: after its directives
fn body_start(body: &FunctionBody<'_>) -> HookAt {
    HookAt::Body(
        body.directives
            .last()
            .map_or(body.span.start + 1, |directive| directive.span.end) as usize,
    )
}

/// Whether a function bound to `name` is a function component or a hook,
/// which may call a hook first: a capitalized name, or `use` and a capital
fn is_component_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_uppercase() => true,
        Some('u') => name
            .strip_prefix("use")
            .is_some_and(|rest| rest.chars().next().is_none_or(|c| c.is_ascii_uppercase())),
        _ => false,
    }
}

/// Where the function `expression` gives starts, through `memo()` and
/// `forwardRef()`, if it gives one
fn function_start(expression: &Expression<'_>) -> Option<u32> {
    match expression.get_inner_expression() {
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span.start),
        Expression::FunctionExpression(function) => Some(function.span.start),
        Expression::CallExpression(call) if is_component_wrapper(&call.callee) => call
            .arguments
            .first()
            .and_then(|argument| argument.as_expression())
            .and_then(function_start),
        _ => None,
    }
}

/// Whether `callee` wraps a component in a component: `memo` or `forwardRef`,
/// imported or read from `React`
fn is_component_wrapper(callee: &Expression<'_>) -> bool {
    let name = match callee.get_inner_expression() {
        Expression::Identifier(identifier) => identifier.name.as_str(),
        Expression::StaticMemberExpression(member) => member.property.name.as_str(),
        _ => return false,
    };
    matches!(name, "memo" | "forwardRef")
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

    /// Visit a function, through `visit`, as the innermost frame when it has
    /// a body, then call the hook reading the theme first in it when an
    /// element in it reads the theme
    fn in_frame(
        &mut self,
        component: bool,
        hook_at: Option<HookAt>,
        visit: impl FnOnce(&mut Self),
    ) {
        let Some(hook_at) = hook_at else {
            visit(self);
            return;
        };
        self.frames.push(Frame {
            component,
            reads: Reads::default(),
            hook_at,
        });
        visit(self);
        if let Some(frame) = self.frames.pop()
            && frame.reads.any()
        {
            self.call_hooks(frame.hook_at, frame.reads);
        }
    }

    /// Call the hooks reading what `reads` names at `at`
    fn call_hooks(&mut self, at: HookAt, reads: Reads) {
        let call = self.hook_calls(reads);
        match at {
            HookAt::Body(at) => self.replacements.push((at, at, format!("\n{call}"))),
            HookAt::Expression(start, end) => {
                self.replacements
                    .push((start, start, format!("{{ {call} return ")));
                self.replacements.push((end, end, "; }".to_string()));
            }
        }
    }

    /// The statements calling the hooks reading what `reads` names
    fn hook_calls(&mut self, reads: Reads) -> String {
        self.runtime.hooks_used = self.runtime.hooks_used.union(reads);
        let calls: Vec<String> = self
            .runtime
            .variables(reads)
            .map(|(hook, variable)| format!("const {variable} = {}();", self.runtime.hooks[hook]))
            .collect();
        calls.join(" ")
    }

    /// Whether the element being compiled is in a function component or a
    /// hook, which then calls the hooks reading what `reads` names first
    fn read_in_component(&mut self, reads: Reads) -> bool {
        match self.frames.iter_mut().rev().find(|frame| frame.component) {
            Some(frame) => {
                frame.reads = frame.reads.union(reads);
                true
            }
            None => false,
        }
    }

    /// The styles a style prop declares with literals, each from the
    /// breakpoint it is paired with on: one style object, or one a
    /// breakpoint and a theme where the styles differ by them
    fn static_choice(&self, name: &str, items: &[(usize, Literal)]) -> Result<Choice, Problem> {
        // A number of a compat package is pixels, as React Native reads it
        let items: Cow<[(usize, Literal)]> = if self.compat {
            Cow::Owned(in_pixels(name, items))
        } else {
            Cow::Borrowed(items)
        };
        if items
            .iter()
            .any(|(_, literal)| viewport_of(literal).is_some())
            && let Some(keys) = length_keys(name)
        {
            return self.viewport_choice(name, &keys, &items);
        }
        let theme = &self.target.theme;
        let themes = match self.theme_scope {
            Some(index) => index..index + 1,
            None if items.iter().any(|(_, literal)| names_tokens(name, literal)) => {
                0..theme.count()
            }
            None => 0..1,
        };
        let levels = items
            .iter()
            .map(|(from, literal)| (from + 1).max(literal_levels(name, literal, theme)))
            .fold(1, usize::max)
            .min(theme.levels());
        let mut grid = Vec::with_capacity(levels);
        for level in 0..levels {
            let mut row = Vec::with_capacity(themes.len());
            for index in themes.clone() {
                row.push(cell(
                    name,
                    &items,
                    &Tokens {
                        theme,
                        index,
                        level,
                    },
                )?);
            }
            grid.push(row);
        }
        let choice = varied(grid);
        self.check_runtime(&choice)?;
        Ok(choice)
    }

    /// The styles of the literals of the style prop `name`, which sets
    /// `keys`, where one at least is a viewport length: each from its
    /// breakpoint on, a viewport length computed from the window's size
    fn viewport_choice(
        &self,
        name: &str,
        keys: &[&str],
        items: &[(usize, Literal)],
    ) -> Result<Choice, Problem> {
        if !self.target.runtime {
            return Err(RuntimeFeature::Viewport.problem());
        }
        let mut choices = Vec::with_capacity(items.len());
        for (level, literal) in items {
            let choice = match viewport_of(literal) {
                Some(length) => self.computed(keys, length),
                None => self.static_choice(name, &[(0, literal.clone())])?,
            };
            choices.push(if *level == 0 {
                choice
            } else {
                when(State::AtLeast(*level), choice)
            });
        }
        Ok(if choices.len() == 1 {
            choices.remove(0)
        } else {
            Choice::All(choices)
        })
    }

    /// The styles `keys` take from a viewport length: the hundredths
    /// `amount` of what `viewport` measures, one multiplication of the size
    /// the runtime tells
    fn computed(&self, keys: &[&str], (amount, viewport): (f64, Viewport)) -> Choice {
        let (width, height) = (&self.runtime.width, &self.runtime.height);
        let both = Reads::default()
            .with(Read::Width, true)
            .with(Read::Height, true);
        let (side, reads) = match viewport {
            Viewport::Width => (width.clone(), Reads::default().with(Read::Width, true)),
            Viewport::Height => (height.clone(), Reads::default().with(Read::Height, true)),
            Viewport::Min => (format!("Math.min({width}, {height})"), both),
            Viewport::Max => (format!("Math.max({width}, {height})"), both),
        };
        let factor = number_code(amount / 100.0);
        let code = if factor == "1" {
            side
        } else {
            format!("{side} * {factor}")
        };
        Choice::Computed(
            keys.iter()
                .map(|key| ((*key).to_string(), code.clone()))
                .collect(),
            reads,
        )
    }

    /// Fail for a choice the runtime makes when it is turned off
    fn check_runtime(&self, choice: &Choice) -> Result<(), Problem> {
        let reads = choice.reads();
        let feature = if self.target.runtime {
            None
        } else if reads.has(Read::Breakpoint) {
            Some(RuntimeFeature::Responsive)
        } else if reads.has(Read::Theme) {
            Some(RuntimeFeature::Theme)
        } else {
            None
        };
        feature.map_or(Ok(()), |feature| Err(feature.problem()))
    }

    /// Compile one Devup UI element
    fn element(&mut self, element: &JSXElement<'a>, kind: &ExportVariableKind, local: &str) {
        let opening = &element.opening_element;
        // What an element in a prop notes is its own
        let outer = (self.press.take(), self.animation.take());
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
        let press = self.press.take();
        let held: Vec<State> = INTERACTIONS
            .into_iter()
            .filter(|state| styles.blocks.iter().any(|choice| choice.holds(*state)))
            .collect();
        if let Some(press) = press.filter(|_| !held.is_empty()) {
            if *kind == ExportVariableKind::Input && held == [State::Focused] && tag.is_none() {
                // A `TextInput` tells its focus only through its events
                styles.focus_wrapped = true;
            } else {
                tag = self.pressable(press, kind, local, tag);
            }
        }
        if let Some(animation) = self.animation.take() {
            let interactive = !held.is_empty() && !styles.focus_wrapped;
            tag = self.animated(&animation, (kind, local), local, tag, interactive);
        }
        (self.press, self.animation) = outer;
        self.finish(element, styles, (tag, local), previous_end);
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
        if name == "disabled" {
            styles.disabled = Some(
                attribute_expression(attribute)
                    .map_or_else(|| "true".to_string(), |value| self.source(value)),
            );
        }
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
            Prop::ClassName => self.class_name(attribute, span, local, styles),
            Prop::Declares => {
                styles.removed.push(span);
                self.note_animation(name, attribute.span.start);
                match self.value_choice(name, attribute.value.as_ref()) {
                    Ok(choice) => styles.choices.push(choice),
                    Err(problem) => self.error(attribute.span.start, problem.on(local, name)),
                }
            }
            Prop::Platform(platform) => {
                styles.removed.push(span);
                let choices = self.block(
                    attribute_expression(attribute),
                    name,
                    attribute.span.start,
                    local,
                );
                if platform == self.target.platform {
                    styles.choices.extend(choices);
                }
            }
            Prop::Theme(theme) => {
                styles.removed.push(span);
                let choices = self.theme_block(
                    attribute_expression(attribute),
                    (name, &theme),
                    attribute.span.start,
                    local,
                );
                styles.blocks.extend(choices);
            }
            Prop::When(state) => {
                styles.removed.push(span);
                let choices = self.state_block(
                    attribute_expression(attribute),
                    (name, state),
                    attribute.span.start,
                    local,
                );
                styles.blocks.extend(choices);
            }
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
                let code = self.rewritten(expression);
                let at = expression.span();
                styles.kept = Some((at.start as usize, at.end as usize, code.clone()));
                styles.given.push(code);
                styles.style = Some(span);
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

    /// The styles of a block, such as `_ios={{ boxShadow: '...' }}` or
    /// `_themeDark={{ bg: 'black' }}`
    fn block(
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
                format!("`{name}` takes an object of styles"),
                format!("write the styles in an object, such as `{name}={{{{ p: 2 }}}}`"),
            )
            .on(local, name),
        );
        Vec::new()
    }

    /// The styles of a theme block, such as `_themeDark={{ bg: 'black' }}`,
    /// `name` being the prop and `theme` the theme it names: they apply only
    /// while that theme shows
    fn theme_block(
        &mut self,
        value: Option<&Expression<'a>>,
        (name, theme): (&str, &str),
        offset: u32,
        local: &str,
    ) -> Vec<Choice> {
        let Some(index) = self.target.theme.index_of(theme) else {
            self.error(
                offset,
                Problem::new(
                    format!("devup.json defines no `{theme}` theme"),
                    "add the theme to devup.json, or remove the block",
                )
                .on(local, name),
            );
            return Vec::new();
        };
        let count = self.target.theme.count();
        if count > 1 && !self.target.runtime {
            self.error(offset, RuntimeFeature::Theme.problem().on(local, name));
            return Vec::new();
        }
        if self.theme_scope.is_some_and(|scope| scope != index) {
            return Vec::new();
        }
        let outer = self.theme_scope.replace(index);
        let choices = self.block(value, name, offset, local);
        self.theme_scope = outer;
        choices
            .into_iter()
            .map(|choice| in_theme(index, count, choice))
            .collect()
    }

    /// The styles of `_landscape`, `_portrait` or `_pressed`, `name` being
    /// the prop: they apply only while `state` holds
    fn state_block(
        &mut self,
        value: Option<&Expression<'a>>,
        (name, state): (&str, State),
        offset: u32,
        local: &str,
    ) -> Vec<Choice> {
        // `Pressable` tells whether it is pressed, hovered or focused, and
        // `disabled` whether it is disabled: only the orientation needs the
        // runtime
        if INTERACTIONS.contains(&state) {
            self.press.get_or_insert_with(|| (offset, name.to_string()));
        } else if !self.target.runtime && state != State::Disabled {
            let feature = if matches!(state, State::ReducedMotion(_)) {
                RuntimeFeature::Motion
            } else {
                RuntimeFeature::Orientation
            };
            self.error(offset, feature.problem().on(local, name));
            return Vec::new();
        }
        self.block(value, name, offset, local)
            .into_iter()
            .map(|choice| when(state, choice))
            .collect()
    }

    /// The styles of an object of style props
    fn object_choices(&mut self, object: &ObjectExpression<'a>, local: &str) -> Vec<Choice> {
        let mut choices = Vec::new();
        let mut blocks = Vec::new();
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                self.error(
                    property.span().start,
                    Problem::new(
                        "the build reads only the styles written in a block",
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
                        "the build reads only keys written in a block",
                        "write the key itself",
                    )
                    .on(local, &format!("[{key}]")),
                );
                continue;
            };
            let block = matches!(classify(&key), Prop::Theme(_) | Prop::When(_));
            let property_choices =
                self.object_property(&key, &property.value, property.span.start, local);
            if block { &mut blocks } else { &mut choices }.extend(property_choices);
        }
        choices.extend(blocks);
        choices
    }

    /// The styles one key of a block declares
    fn object_property(
        &mut self,
        key: &str,
        value: &Expression<'a>,
        offset: u32,
        local: &str,
    ) -> Vec<Choice> {
        match classify(key) {
            Prop::Declares => {
                self.note_animation(key, offset);
                match self.expression_choice(key, value) {
                    Ok(choice) => return vec![choice],
                    Err(problem) => self.error(offset, problem.on(local, key)),
                }
            }
            Prop::Platform(platform) => {
                let choices = self.block(Some(value), key, offset, local);
                if platform == self.target.platform {
                    return choices;
                }
            }
            Prop::Theme(theme) => {
                return self.theme_block(Some(value), (key, &theme), offset, local);
            }
            Prop::When(state) => {
                return self.state_block(Some(value), (key, state), offset, local);
            }
            Prop::Unsupported(problem) => self.error(offset, problem.on(local, key)),
            Prop::As | Prop::Style | Prop::ClassName | Prop::Pass => self.error(
                offset,
                Problem::new(
                    "a block holds only style props",
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
                |text| self.static_choice(name, &[(0, Literal::Text(text.to_string()))]),
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
            return self.static_choice(name, &[(0, literal)]);
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
            Expression::ArrayExpression(array) => self.responsive_choice(name, array),
            Expression::ObjectExpression(_) => Err(Problem::new(
                "a style prop takes one value, not an object",
                "give one value, or use a platform block such as `_ios`",
            )),
            expression => self.dynamic_choice(name, &self.source(expression)),
        }
    }

    /// The styles a responsive array declares: each value from the
    /// breakpoint at its index on, as on the web, where `null` and
    /// `undefined` keep the value before. Literals compile to one style
    /// object a breakpoint; any other value applies from its breakpoint on
    fn responsive_choice(
        &self,
        name: &str,
        array: &ArrayExpression<'a>,
    ) -> Result<Choice, Problem> {
        let last = self.target.theme.levels() - 1;
        let mut items = Vec::new();
        for (index, element) in array.elements.iter().enumerate() {
            match element.as_expression() {
                Some(expression) if !is_empty_value(expression) => {
                    items.push((index.min(last), expression));
                }
                None if matches!(element, ArrayExpressionElement::SpreadElement(_)) => {
                    return Err(Problem::new(
                        "the build reads only the values written in a responsive array",
                        "write each value in the array",
                    ));
                }
                _ => {}
            }
        }
        let literals: Option<Vec<(usize, Literal)>> = items
            .iter()
            .map(|(level, expression)| literal(expression).map(|literal| (*level, literal)))
            .collect();
        if let Some(literals) = literals {
            return self.static_choice(name, &literals);
        }
        if !self.target.runtime {
            return Err(RuntimeFeature::Responsive.problem());
        }
        let mut choices = Vec::with_capacity(items.len());
        for (level, expression) in items {
            if matches!(
                expression.without_parentheses(),
                Expression::ArrayExpression(_)
            ) {
                return Err(Problem::new(
                    "a responsive array holds one value a breakpoint",
                    "write each value itself, not an array",
                ));
            }
            let choice = self.expression_choice(name, expression)?;
            choices.push(if level == 0 {
                choice
            } else {
                when(State::AtLeast(level), choice)
            });
        }
        Ok(Choice::All(choices))
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
        // A number of a compat package is pixels, as React Native reads it
        let scale = scale.map(|scale| if self.compat { 1 } else { scale });
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
        if self.in_styled && is_function(expression) {
            // A function in the styles of a `styled()` component takes its props
            self.props_called.set(true);
            return format!("({code})({})", self.props);
        }
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
        (tag, local): (Option<String>, &str),
        attributes_end: u32,
    ) {
        let Styles {
            classes,
            mut choices,
            blocks,
            given,
            mut removed,
            style,
            kept,
            opaque_spread,
            disabled,
            focus_wrapped,
        } = styles;
        choices.extend(blocks);
        if disabled.is_none() {
            // `_disabled` applies only to an element given `disabled`
            choices = choices
                .into_iter()
                .map(|choice| without(choice, State::Disabled))
                .collect();
        }
        let disabled = disabled.unwrap_or_default();
        let vars = Vars {
            disabled: &disabled,
            ..self.runtime.vars()
        };
        let keys = interaction_keys(&choices, &vars);
        let mut style_code = style_code(choices, (classes, given), &mut self.sheet, &vars);
        let reads = style_code
            .as_ref()
            .map_or_else(Reads::default, |style| style.reads);
        let focus_wrapped = focus_wrapped && style_code.is_some();
        if let Some(style) = style_code
            .as_mut()
            .filter(|_| !keys.is_empty() && !focus_wrapped)
        {
            // `Pressable` calls a function for its style with its state
            style.code = format!("({{ {} }}) => {}", keys.join(", "), style.code);
        }
        let in_component = !reads.any() || self.read_in_component(reads);
        if !in_component {
            self.runtime.wrapper_reads = self.runtime.wrapper_reads.union(reads);
        }
        if focus_wrapped && !in_component && reads.sized() {
            self.error(
                element.span.start,
                Problem::new(
                    "an input with a focus style outside a function component cannot read the window's size, which a viewport length needs",
                    "render the input in a function component",
                )
                .on(local, "_focus"),
            );
        }
        let wrapper = self.wrapper(focus_wrapped, in_component, reads);
        if let Some(name) = wrapper.as_ref().or(tag.as_ref()) {
            self.rename(element, name);
        }
        let Some(style_code) = style_code else {
            self.remove(&removed);
            self.replacements.extend(kept);
            return;
        };
        let code = if wrapper.is_some() {
            // `Focused` gives whether the element has focus first; a wrapper
            // gives the theme, the breakpoint and the orientation, in this
            // order, to an element outside a component: the style takes them
            // up to the last it reads
            let focus = focus_wrapped.then_some(self.runtime.focused.as_str());
            let read = self.runtime.variables(reads).last().map(|(at, _)| at);
            let runtime = self.runtime.runtime_variables();
            let parameters = focus
                .into_iter()
                .chain(
                    runtime[..read.filter(|_| !in_component).map_or(0, |at| at + 1)]
                        .iter()
                        .map(|name| name.as_str()),
                )
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                " $c={{{}}} $s={{({parameters}) => ({})}}",
                tag.as_deref().unwrap_or(local),
                style_code.code
            )
        } else {
            format!(" style={{{}}}", style_code.code)
        };
        if let Some(spread) = opaque_spread {
            self.error(
                spread.start,
                "A spread on a Devup UI element must be a name or a member on React Native: the element merges the `style` it gives with its own. Fix: assign the spread object to a variable first".to_string(),
            );
        }
        removed.extend(style);
        self.remove(&removed);
        self.replacements
            .push((attributes_end as usize, attributes_end as usize, code));
    }

    /// The component rendering an element that reads `reads` from the
    /// runtime for it: `Focused` for an input with a focus style, none in a
    /// function component, else `Themed`, or `Sized` where it reads the
    /// window's size
    fn wrapper(&mut self, focus_wrapped: bool, in_component: bool, reads: Reads) -> Option<String> {
        let runtime = &mut self.runtime;
        if focus_wrapped {
            runtime.focused_used = true;
            Some(runtime.focused_wrapper.clone())
        } else if in_component {
            None
        } else if reads.sized() {
            Some(runtime.sized_wrapper.use_name())
        } else {
            Some(runtime.wrapper.use_name())
        }
    }

    /// The component an element taking a press style renders: React Native's
    /// `Pressable` for a `View`, which only it can call the style of with
    /// whether it is pressed, or the `Pressable` it already renders
    fn pressable(
        &mut self,
        (offset, name): (u32, String),
        kind: &ExportVariableKind,
        local: &str,
        tag: Option<String>,
    ) -> Option<String> {
        let problem = match (native_component(kind), &tag) {
            ("Pressable", None) => return None,
            ("View", None) => {
                self.pressable_used = true;
                return Some(self.pressable.clone());
            }
            (_, Some(_)) => Problem::new(
                "a press, hover or focus style makes the element React Native's `Pressable`, which cannot render the component `as` gives",
                "remove `as`, or pass a function reading `pressed` to the `style` of a `Pressable`",
            ),
            (native, None) => Problem::new(
                format!(
                    "a press, hover or focus style needs React Native's `Pressable`, and the element renders React Native's `{native}`"
                ),
                "wrap it in a `Box` or a `Button` with `_pressed`",
            ),
        };
        self.error(offset, problem.on(local, &name));
        tag
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

/// The styles the literals of `items` declare for the style prop `name` at
/// the breakpoint and in the theme of `tokens`: each literal from its
/// breakpoint on, a later one declared after the one before
fn cell(
    name: &str,
    items: &[(usize, Literal)],
    tokens: &Tokens<'_>,
) -> Result<Vec<Declaration>, Problem> {
    let mut declared = Vec::new();
    for (_, literal) in items.iter().filter(|(from, _)| *from <= tokens.level) {
        cascade(
            &mut declared,
            declarations(name, literal, tokens)?.unwrap_or_default(),
        );
    }
    Ok(declared)
}

/// Whether `expression` declares nothing, as `null`, `undefined` and
/// booleans do
fn is_empty_value(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => true,
        Expression::Identifier(identifier) => identifier.name == "undefined",
        _ => false,
    }
}

/// Whether the styles of `literal` for the style prop `name` may differ by
/// theme: a typography, or a value naming a `$token`
fn names_tokens(name: &str, literal: &Literal) -> bool {
    name == "typography" || matches!(literal, Literal::Text(text) if text.contains('$'))
}

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

/// The viewport length `literal` writes, if it writes one
fn viewport_of(literal: &Literal) -> Option<(f64, Viewport)> {
    match literal {
        Literal::Text(text) => viewport_length(text),
        Literal::Number(_) => None,
    }
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

/// `items`, the literals of the style prop `name`, with each number of a
/// length as pixels, as styled-components and Emotion read it
fn in_pixels(name: &str, items: &[(usize, Literal)]) -> Vec<(usize, Literal)> {
    let unitless = css::disassemble_property(name).into_iter().any(|property| {
        is_maintain_value_property(&property) || TIMED.contains(&property.as_ref())
    });
    items
        .iter()
        .map(|(level, literal)| match literal {
            Literal::Number(number) if !unitless => {
                (*level, Literal::Text(format!("{}px", number_code(*number))))
            }
            literal => (*level, literal.clone()),
        })
        .collect()
}
