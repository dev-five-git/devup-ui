use crate::as_visit::As;
use crate::composition::{KnownPart, KnownSide, KnownStyles, overlaps, set_prop_order};
use crate::css_prop::{
    ClassNamesParams, CssProp, THEME_READ, ThemeRoot, class_names_params, read_theme,
    render_function, template_parts, theme_rules_of,
};
use crate::css_utils::{
    TemplateStyles, css_to_style_template, keyframes_to_keyframes_style, optimize_css_block,
    template_css_text,
};
use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::extract_css::ExtractCss;
use crate::extract_style::extract_keyframes::ExtractKeyframes;
use crate::extract_style::style_property::StyleProperty;
use crate::extractor::KeyframesExtractResult;
use crate::extractor::extract_keyframes_from_expression::extract_keyframes_from_expression;
use crate::extractor::extract_style_from_stylex::{
    extract_stylex_declarations, extract_stylex_namespace_styles,
};
use crate::extractor::{
    ExtractResult, GlobalExtractResult,
    extract_global_style_from_expression::extract_global_style_from_expression,
    extract_style_from_expression::{
        LiteralHandling, extract_style_from_expression, flatten_spreads,
    },
    extract_style_from_jsx::extract_style_from_jsx,
    extract_style_from_styled::{
        FORWARD_REF, Naming, StyledDefinition, StyledExtraction, extended,
        extract_style_from_styled, forward_ref, read_forward, take_styled_modifiers,
        with_component,
    },
};
use crate::gen_class_name::{gen_class_names, merge_expression_for_class_name};
use crate::gen_style::gen_styles;
use crate::prop_modify_utils::{
    add_class_and_style, add_class_and_style_to_object, convert_class_name, modify_prop_object,
    modify_props, written_class_name, written_object_class_name,
};
use crate::scope::{Bindings, ClassNamesSymbols, NAMESPACE_STYLED};
use crate::stylex::{
    StylexDynamicInfo, StylexFunction, StylexNamespaceValue, create_theme_class,
    css_variable_block, css_variable_rules, define_vars_variable, variable_values,
};
use crate::util_type::UtilType;
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::disassemble_property;
use css::is_special_property::is_special_property;
use css::keyframes_to_keyframes_name;
use oxc_allocator::{Allocator, CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::ImportDeclarationSpecifier::{self, ImportSpecifier};
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, BinaryOperator, BindingPattern, CallExpression, ChainElement,
    ComputedMemberExpression, Expression, ExpressionStatement, FormalParameter,
    FormalParameterKind, FormalParameters, IdentifierName, ImportDeclaration, ImportOrExportKind,
    JSXAttributeItem, JSXAttributeValue, JSXChild, JSXElement, JSXElementName,
    JSXExpressionContainer, ObjectExpression, ObjectProperty, ObjectPropertyKind, Program,
    PropertyKey, PropertyKind, Statement, StaticMemberExpression, Str, StringLiteral,
    UnaryOperator, VariableDeclarator,
};
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut;
use oxc_ast_visit::walk_mut::{
    walk_call_expression, walk_expression, walk_expression_statement, walk_import_declaration,
    walk_jsx_attribute_value, walk_jsx_child, walk_jsx_element, walk_program,
    walk_variable_declarator, walk_variable_declarators,
};
use oxc_syntax::number::NumberBase;
use oxc_syntax::operator::LogicalOperator;
use oxc_syntax::symbol::SymbolId;

use crate::utils::{
    CLASS_NAMES_CALL, CLASS_NAMES_CHILD, CLASS_NAMES_CLASS_MAP, CLASS_NAMES_PART, CSS_PROP_VALUE,
    LOCAL_STYLES, ParsedStyleOrder, RUNTIME_VALUE, STYLE_OBJECT, StyleArguments, Suspends,
    build_time_error, call_with_values, css_prop_error, css_prop_override_error, element_error,
    expression_to_style_order, fixed_value, get_str_by_property_key,
    get_string_by_literal_expression, get_string_by_property_key, is_pure,
    jsx_expression_to_style_order, key_error, readable_argument, readable_code, reads_directly,
    reads_spreads_once, reads_unknown, runtime_classes, runtime_value, runtime_value_error,
    spread_error, stays_attribute, string_class, style_arguments, uncomposable_error,
    unplaced_error, unreadable_styles, unwrap_syntax_only, unwrap_syntax_only_mut,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, SPAN};
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::rc::Rc;

/// What text among composed parts holds
#[derive(Clone, Copy, PartialEq, Eq)]
enum Text {
    /// Classes, as `css()` takes them
    Classes,
    /// CSS text, as Emotion's `css` prop takes it
    Rules,
}

/// Emotion's `css` prop taken off an element: what it composes, the parts
/// written as `css_prop_value` writes them, and where it was written
struct CssValue<'a> {
    value: Expression<'a>,
    offset: u32,
}

/// Emotion's `css` prop taken off the props a `jsx()` call gives, with what
/// compiling it reads of the call
struct JsxCss<'a> {
    css: CssValue<'a>,
    /// The element the call builds, as messages name it
    element: String,
    /// The binding the element's type reads
    symbol: Option<oxc_syntax::symbol::SymbolId>,
    /// The `className` the props end up with
    class_name: Option<Expression<'a>>,
    /// Whether the props keep the type the call gives: no spread, `as` or
    /// `forwardedAs` can change it
    renders: bool,
}

/// `false ?? right` is `false`, which composes nothing
fn coalesce_keeps_left(logical: &oxc_ast::ast::LogicalExpression<'_>) -> bool {
    logical.operator == LogicalOperator::Coalesce
        && matches!(
            unwrap_syntax_only(&logical.left),
            Expression::BooleanLiteral(_)
        )
}

fn property_stays(property: &ObjectProperty<'_>) -> bool {
    property
        .key
        .static_name()
        .is_some_and(|name| stays_attribute(&name))
}

/// The value of a JSX attribute that stays an attribute of the element built
fn attribute_value<'b, 'a>(
    attribute: &'b oxc_ast::ast::JSXAttribute<'a>,
) -> Option<&'b Expression<'a>> {
    match (&attribute.name, &attribute.value) {
        (Identifier(name), Some(JSXAttributeValue::ExpressionContainer(container)))
            if stays_attribute(&name.name) =>
        {
            container.expression.as_expression()
        }
        _ => None,
    }
}

fn attribute_value_mut<'b, 'a>(
    attribute: &'b mut oxc_ast::ast::JSXAttribute<'a>,
) -> Option<&'b mut Expression<'a>> {
    match (&attribute.name, &mut attribute.value) {
        (Identifier(name), Some(JSXAttributeValue::ExpressionContainer(container)))
            if stays_attribute(&name.name) =>
        {
            container.expression.as_expression_mut()
        }
        _ => None,
    }
}

fn style_property_into_string(style_property: StyleProperty) -> String {
    match style_property {
        StyleProperty::ClassName(name) => name,
        StyleProperty::Variable { variable_name, .. } => {
            let mut value = String::with_capacity("var()".len() + variable_name.len());
            value.push_str("var(");
            value.push_str(&variable_name);
            value.push(')');
            value
        }
    }
}

pub struct DevupVisitor<'a> {
    pub ast: AstBuilder<'a>,
    filename: String,
    /// The bindings the file declares for the APIs the build compiles
    bindings: Bindings,
    package: String,
    /// Entry the rewritten imports of absorbed third-party APIs land on. Its specifiers
    /// register exactly like the main package's so those calls still compile away.
    compat_package: String,
    split_filename: Option<String>,
    pub css_files: Vec<String>,
    pub styles: FxHashSet<ExtractStyleValue>,
    /// Styles the file writes that cannot be extracted at build time, by the
    /// offset of the code each is about
    pub errors: Vec<(u32, String)>,
    /// Pending `StyleX` namespace map from the most recent `stylex.create()` call.
    /// Set in `visit_expression`, consumed in `visit_variable_declarator`.
    stylex_pending_create: Option<FxHashMap<String, StylexNamespaceValue>>,
    /// Maps bindings to their namespace→className mappings from `stylex.create()`.
    /// e.g., `styles` → { "base" → "a b", "active" → "c" }
    stylex_namespaces: FxHashMap<SymbolId, FxHashMap<String, StylexNamespaceValue>>,
    /// What the `<ClassNames>` child functions being compiled take
    class_names_scope: Vec<ClassNamesSymbols>,

    /// `defineVars` and `defineConsts` members of a binding as `key` ->
    /// `"var(--x)"`, so a `stylex.create()` value referencing one resolves to
    /// a static CSS value.
    stylex_var_refs: FxHashMap<SymbolId, FxHashMap<String, String>>,
    /// `defineVars` bindings as `vars` -> (`key` -> `--x`), the contract
    /// `createTheme` reassigns.
    stylex_var_names: FxHashMap<SymbolId, FxHashMap<String, String>>,
    /// `createTheme` bindings as `theme` -> `class`, so `stylex.props(theme)` resolves.
    stylex_theme_classes: FxHashMap<SymbolId, String>,
    /// `StyleX` variables and themes the program imports from other modules,
    /// by the name it binds them to
    imported_stylex: (
        FxHashMap<String, FxHashMap<String, String>>,
        FxHashMap<String, String>,
    ),
    /// Pending `defineVars` contract awaiting its variable declarator.
    stylex_pending_vars: Option<FxHashMap<String, String>>,
    /// Pending `createTheme` class awaiting its variable declarator.
    stylex_pending_theme_class: Option<String>,
    /// Pending `defineConsts` values awaiting their variable declarator.
    stylex_pending_consts: Option<FxHashMap<String, String>>,
    /// Pending keyframe animation name from most recent `stylex.keyframes()` call.
    stylex_pending_keyframe_name: Option<String>,
    /// Maps bindings to their keyframe animation names.
    /// e.g., `fadeIn` → "a-a"
    stylex_keyframe_names: FxHashMap<SymbolId, String>,
    /// What the element just visited becomes when it is not an element any
    /// more (a dynamic `as`, a spread evaluated once): set in
    /// `visit_jsx_element`, written where the element stands by the visit of
    /// the expression, child or attribute value holding it
    pending_replacement: Option<Expression<'a>>,
    spreads_read_once: usize,
    /// Elements whose type only the runtime gives, which each bind it to a
    /// name of their own
    runtime_types: usize,
    /// The semantic analysis of the program, when built before the visit
    scoping: Option<Rc<oxc_semantic::Scoping>>,
    /// The classes and keyframes names the file binds to a `const`
    style_values: crate::style_values::StyleValues,
    /// The styles of the last `css()` call giving a class, by where it starts,
    /// for the `const` it initializes
    css_styles: Option<(u32, Vec<ExtractStyleValue>)>,
    /// The styles behind `css()` classes the file imports, by binding
    imported_css: FxHashMap<String, Vec<ExtractStyleValue>>,
    imported_atoms: crate::vanilla_extract::producer_atoms::ProducerAtoms,
    pub(crate) style_operand_mode: crate::vanilla_extract::StyleOperandMode,
    /// The styled component just built, by where it starts, for the `const`
    /// it initializes
    pending_styled: Option<(u32, StyledDefinition<'a>)>,
    /// The styled bindings other styles select
    selected_components: FxHashSet<oxc_syntax::symbol::SymbolId>,
    /// The marker class of the selected styled component the declaration
    /// being visited defines, with where its definition starts
    pending_marker: Option<(u32, String)>,
    /// Whether a generated styled component forwards refs through React's
    /// `forwardRef`, which the program then imports
    forwards_refs: bool,
    /// The styled components the file binds to a `const`, which a component
    /// extending one renders in its place
    styled_definitions: FxHashMap<oxc_syntax::symbol::SymbolId, StyledDefinition<'a>>,
    unknown_bindings: crate::imported_constants::Unknown,
    /// Whether `css()` or `styled()` joined as a class, or an element took
    /// through a spread, a binding that may hold rules only running the module
    /// gives
    pub composes_unknown: bool,
    /// Parts `css()` or `styled()` join that read such a binding: the build
    /// cannot tell rules from a class in them, so each is a build error unless
    /// the build computes it
    pub unknown_parts: Vec<(u32, String)>,
    /// Objects and arrays code changes, which styles cannot take whole
    changed_bindings: crate::imported_constants::Changed,
    /// Which elements take Emotion's `css` prop
    css_prop: CssProp,
    /// Whether the visit compiled the `css` prop of an element
    pub compiled_css_prop: bool,
    /// Bindings declared to an object, an array, a function or text, which a
    /// `css` prop cannot compose as a class, besides the top-level constants
    /// the build reads in its place
    local_styles: FxHashSet<oxc_syntax::symbol::SymbolId>,
}

/// Whether `expression`, or a value it chooses, reads an object or array code
/// changes, through the identifiers `reads` accepts
fn reads_binding(
    expression: &Expression<'_>,
    changed: &crate::imported_constants::Changed,
    reads: &dyn Fn(&oxc_ast::ast::IdentifierReference<'_>) -> bool,
) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::ArrayExpression(array) => array.elements.iter().any(|element| {
            element
                .as_expression()
                .is_some_and(|element| reads_binding(element, changed, reads))
        }),
        Expression::LogicalExpression(logical) => {
            reads_binding(&logical.left, changed, reads)
                || reads_binding(&logical.right, changed, reads)
        }
        Expression::ConditionalExpression(conditional) => {
            reads_binding(&conditional.consequent, changed, reads)
                || reads_binding(&conditional.alternate, changed, reads)
        }
        expression => changed.read_by_in(expression, reads),
    }
}

impl<'a> DevupVisitor<'a> {
    pub fn changed_bindings(&mut self, changed: crate::imported_constants::Changed) {
        self.changed_bindings = changed;
    }

    /// Record the arguments of `api` that read a binding only running the
    /// module gives
    fn unknown_arguments(&mut self, api: &str, arguments: &[Argument<'a>]) {
        if self.unknown_bindings.is_empty() {
            return;
        }
        for argument in arguments {
            let expression = match argument {
                Argument::SpreadElement(spread) => &spread.argument,
                argument => argument.to_expression(),
            };
            if reads_unknown(expression, &self.unknown_bindings, &|identifier| {
                self.bindings.reads_module(identifier)
            }) {
                self.composes_unknown = true;
                self.unknown_parts.push((
                    argument.span().start,
                    build_time_error(api, &readable_argument(argument), STYLE_OBJECT),
                ));
            }
        }
    }

    /// Report the arguments of `api` that give styles code changes
    fn changed_arguments(&mut self, api: &str, arguments: &[Argument<'a>]) {
        if self.changed_bindings.is_empty() {
            return;
        }
        for argument in arguments {
            let expression = match argument {
                Argument::SpreadElement(spread) => &spread.argument,
                argument => argument.to_expression(),
            };
            if reads_binding(expression, &self.changed_bindings, &|identifier| {
                self.bindings.reads_module(identifier)
            }) {
                self.errors.push((
                    argument.span().start,
                    build_time_error(api, &readable_argument(argument), STYLE_OBJECT),
                ));
            }
        }
    }

    /// `css(...)` composing a class whose styles the build knows: the parts'
    /// styles merge, a later declaration replacing an earlier one. `None` when
    /// no part is such a class, or a part is one composing does not read.
    fn compose_known_styles(&mut self, call: &CallExpression<'a>) -> Option<Expression<'a>> {
        let arguments: Vec<&Expression<'a>> = call
            .arguments
            .iter()
            .map(|argument| match argument {
                Argument::SpreadElement(spread) => &spread.argument,
                argument => argument.to_expression(),
            })
            .collect();
        let ordered_rules = matches!(
            self.style_operand_mode,
            crate::vanilla_extract::StyleOperandMode::Ordered
        ) && arguments.len() > 1
            && arguments.iter().any(|argument| {
                matches!(
                    unwrap_syntax_only(argument),
                    Expression::ObjectExpression(_)
                )
            });
        if !(ordered_rules
            || arguments
                .iter()
                .any(|argument| self.reads_known_styles(argument)))
        {
            return None;
        }
        let mut parts = Vec::new();
        for argument in &arguments {
            self.known_parts(argument, &mut parts, Text::Classes)?;
        }
        self.unknown_arguments("css", &call.arguments);
        self.changed_arguments("css", &call.arguments);
        let (result, known) = self.composed_class(call.span.start, parts);
        if let Some(known) = known {
            self.css_styles = Some((call.span.start, known));
        }
        Some(result)
    }

    /// The classes `parts`, composed at `offset`, give, with the styles they
    /// always set when no class the build does not know joins them
    fn composed_class(
        &mut self,
        offset: u32,
        parts: Vec<KnownPart<'a>>,
    ) -> (Expression<'a>, Option<Vec<ExtractStyleValue>>) {
        let mut composition = crate::composition::Composition::default();
        let mut classes = Vec::new();
        for part in parts {
            match part {
                KnownPart::Styles(side) => {
                    let props = self.part_props(offset, side, None);
                    composition.apply(&self.ast, props);
                }
                KnownPart::Conditional {
                    test,
                    consequent,
                    alternate,
                } => {
                    let consequent = self.part_props(offset, consequent, None);
                    let alternate = self.part_props(offset, alternate, None);
                    composition.apply_conditional(&self.ast, &test, consequent, alternate);
                }
                KnownPart::Class(mut class) => {
                    self.style_values.read_in(&self.ast, &mut class);
                    classes.push(class);
                }
            }
        }
        let known = composition.unconditional().filter(|_| classes.is_empty());
        let mut props = composition.into_props();
        // Class names come out in reverse, so they read in composing order
        props.reverse();
        let class_name =
            gen_class_names(&self.ast, &mut props, None, self.split_filename.as_deref());
        self.styles
            .extend(props.into_iter().flat_map(ExtractStyleProp::into_extract));
        let result =
            merge_expression_for_class_name(&self.ast, classes.into_iter().chain(class_name))
                .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, &self.ast));
        (result, known)
    }

    /// Whether `expression`, or a part of it, is a class whose styles the
    /// build knows
    fn reads_known_styles(&self, expression: &Expression<'a>) -> bool {
        match unwrap_syntax_only(expression) {
            Expression::ArrayExpression(array) => array.elements.iter().any(|element| {
                element
                    .as_expression()
                    .is_some_and(|element| self.reads_known_styles(element))
            }),
            Expression::LogicalExpression(logical) => {
                self.reads_known_styles(&logical.right)
                    || (logical.operator != LogicalOperator::And
                        && matches!(unwrap_syntax_only(&logical.left), Expression::StringLiteral(literal) if self.style_values.has_literal_styles(literal.value.as_str())))
            }
            Expression::ConditionalExpression(conditional) => {
                self.reads_known_styles(&conditional.consequent)
                    || self.reads_known_styles(&conditional.alternate)
            }
            Expression::StringLiteral(literal) => {
                self.style_values.has_literal_styles(literal.value.as_str())
            }
            expression => self.style_values.styles(expression).is_some(),
        }
    }

    /// The parts `expression` composes, in order, reading text as `text`;
    /// `None` for a shape this path does not read, which the general one then
    /// reads
    fn known_parts(
        &self,
        expression: &Expression<'a>,
        parts: &mut Vec<KnownPart<'a>>,
        text: Text,
    ) -> Option<()> {
        let clone = |expression: &Expression<'a>| {
            expression.clone_in_with_semantic_ids(self.ast.allocator())
        };
        let (test, consequent, alternate) = match unwrap_syntax_only(expression) {
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    self.known_parts(element.as_expression()?, parts, text)?;
                }
                return Some(());
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                (&logical.left, &logical.right, None)
            }
            // `left || right` and `left ?? right`: `left` while it applies, `right`
            // otherwise
            Expression::LogicalExpression(logical) => {
                return match self.known_side(&logical.left, text)? {
                    KnownSide::Styles(side) => {
                        parts.push(KnownPart::Styles(side));
                        Some(())
                    }
                    KnownSide::Mixed(side, class) => {
                        parts.push(KnownPart::Styles(side));
                        parts.push(KnownPart::Class(class));
                        Some(())
                    }
                    KnownSide::Empty if coalesce_keeps_left(logical) => Some(()),
                    KnownSide::Empty => self.known_parts(&logical.right, parts, text),
                    KnownSide::Class(left) => {
                        let test = if logical.operator == LogicalOperator::Or {
                            clone(&left)
                        } else {
                            Expression::new_binary_expression(
                                SPAN,
                                clone(&left),
                                BinaryOperator::Inequality,
                                Expression::new_null_literal(SPAN, &self.ast),
                                &self.ast,
                            )
                        };
                        let right = self.known_side(&logical.right, text)?;
                        self.push_choice(
                            parts,
                            &test,
                            KnownSide::Class(string_class(&self.ast, &left)),
                            right,
                        );
                        Some(())
                    }
                };
            }
            Expression::ConditionalExpression(conditional) => (
                &conditional.test,
                &conditional.consequent,
                Some(&conditional.alternate),
            ),
            Expression::CallExpression(call) if self.class_names_text(&call.callee).is_some() => {
                let text = self.class_names_text(&call.callee)?;
                for argument in &call.arguments {
                    self.known_parts(argument.as_expression()?, parts, text)?;
                }
                return Some(());
            }
            expression => {
                match self.known_side(expression, text)? {
                    KnownSide::Styles(side) => parts.push(KnownPart::Styles(side)),
                    KnownSide::Mixed(side, class) => {
                        parts.push(KnownPart::Styles(side));
                        parts.push(KnownPart::Class(class));
                    }
                    KnownSide::Class(class) => parts.push(KnownPart::Class(class)),
                    KnownSide::Empty => {}
                }
                return Some(());
            }
        };
        let consequent = self.known_side(consequent, text)?;
        let alternate = alternate.map_or(Some(KnownSide::Empty), |alternate| {
            self.known_side(alternate, text)
        })?;
        self.push_choice(parts, test, consequent, alternate);
        Some(())
    }

    /// `test ? consequent : alternate` among the parts, its classes and its
    /// styles each choosing on their own
    fn push_choice(
        &self,
        parts: &mut Vec<KnownPart<'a>>,
        test: &Expression<'a>,
        consequent: KnownSide<'a>,
        alternate: KnownSide<'a>,
    ) {
        let clone = |expression: &Expression<'a>| {
            expression.clone_in_with_semantic_ids(self.ast.allocator())
        };
        let mut classes = [None, None];
        let mut styles = [None, None];
        for (index, side) in [consequent, alternate].into_iter().enumerate() {
            match side {
                KnownSide::Styles(side) => styles[index] = Some(side),
                KnownSide::Mixed(side, class) => {
                    styles[index] = Some(side);
                    classes[index] = Some(class);
                }
                KnownSide::Class(class) => classes[index] = Some(class),
                KnownSide::Empty => {}
            }
        }
        if classes.iter().any(Option::is_some) {
            let [consequent, alternate] = classes.map(|class| {
                class.unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, &self.ast))
            });
            parts.push(KnownPart::Class(Expression::new_conditional_expression(
                SPAN,
                clone(test),
                consequent,
                alternate,
                &self.ast,
            )));
        }
        if styles.iter().any(Option::is_some) {
            let [consequent, alternate] = styles.map(Option::unwrap_or_default);
            parts.push(KnownPart::Conditional {
                test: clone(test),
                consequent,
                alternate,
            });
        }
    }

    /// A side of a condition among composed parts. Its code keeps the bindings
    /// it reads, so the values the file binds them to are read in it.
    fn known_side(&self, expression: &Expression<'a>, text: Text) -> Option<KnownSide<'a>> {
        let expression = unwrap_syntax_only(expression);
        if let Expression::StringLiteral(literal) = expression
            && let Some((values, residual)) =
                self.style_values.literal_parts(literal.value.as_str())
        {
            let styles = vec![KnownStyles::Known(values)];
            return Some(if residual.is_empty() {
                KnownSide::Styles(styles)
            } else {
                KnownSide::Mixed(
                    styles,
                    Expression::new_string_literal(
                        SPAN,
                        Str::from_in(residual.as_str(), self.ast.allocator()),
                        None,
                        &self.ast,
                    ),
                )
            });
        }
        if let Some(styles) = self.style_values.styles(expression) {
            return Some(KnownSide::Styles(vec![KnownStyles::Known(styles.to_vec())]));
        }
        let code = || expression.clone_in_with_semantic_ids(self.ast.allocator());
        match expression {
            Expression::StringLiteral(literal)
                if text == Text::Rules && literal.value.trim().is_empty() =>
            {
                Some(KnownSide::Empty)
            }
            Expression::ObjectExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
                if text == Text::Rules || matches!(expression, Expression::ObjectExpression(_)) =>
            {
                Some(KnownSide::Styles(vec![KnownStyles::Rules(code())]))
            }
            Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => Some(KnownSide::Empty),
            Expression::Identifier(identifier) if identifier.name == "undefined" => {
                Some(KnownSide::Empty)
            }
            Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_) => Some(KnownSide::Class(code())),
            Expression::CallExpression(call) if self.class_names_text(&call.callee).is_some() => {
                self.folded_side(expression, text)
            }
            Expression::ArrayExpression(_) if !self.class_names_scope.is_empty() => {
                self.folded_side(expression, text)
            }
            _ => None,
        }
    }

    /// The parts `expression` composes as one side of a condition: only
    /// styles, or only classes, which join
    fn folded_side(&self, expression: &Expression<'a>, text: Text) -> Option<KnownSide<'a>> {
        let mut parts = Vec::new();
        self.known_parts(expression, &mut parts, text)?;
        let mut styles = Vec::new();
        let mut classes = Vec::new();
        for part in parts {
            match part {
                KnownPart::Styles(side) => styles.extend(side),
                KnownPart::Class(class) => classes.push(class),
                KnownPart::Conditional { .. } => return None,
            }
        }
        match (styles.is_empty(), classes.is_empty()) {
            (true, true) => Some(KnownSide::Empty),
            (false, true) => Some(KnownSide::Styles(styles)),
            (true, false) => {
                merge_expression_for_class_name(&self.ast, classes).map(KnownSide::Class)
            }
            (false, false) => None,
        }
    }

    /// The styles of a part, each at the order the part gives it; `element`
    /// names the element taking them as its `css` prop, which sets a value
    /// only the runtime gives as a CSS variable
    fn part_props(
        &mut self,
        offset: u32,
        styles: Vec<KnownStyles<'a>>,
        element: Option<&str>,
    ) -> Vec<ExtractStyleProp<'a>> {
        let mut props = Vec::new();
        for styles in styles {
            match styles {
                KnownStyles::Known(values) => {
                    props.extend(values.into_iter().map(ExtractStyleProp::Static));
                }
                KnownStyles::Rules(mut rules) => {
                    self.style_values.read_in(&self.ast, &mut rules);
                    let ExtractResult {
                        mut styles,
                        style_order,
                        ..
                    } = extract_style_from_expression(
                        &self.ast,
                        None,
                        &mut rules,
                        0,
                        &None,
                        LiteralHandling::ExpandResponsiveThemeToken,
                    );
                    if let Some(element) = element {
                        let mut unreadable = Vec::new();
                        unreadable_styles(&styles, true, &mut unreadable);
                        for (at, code) in unreadable {
                            self.errors
                                .push((at, css_prop_error(element, &code, STYLE_OBJECT)));
                        }
                    } else if let Some(value) = runtime_value(&styles) {
                        self.errors
                            .push((offset, runtime_value_error("css", &value)));
                    }
                    if let Some(order) = style_order {
                        for prop in &mut styles {
                            set_prop_order(prop, order);
                        }
                    }
                    props.extend(styles);
                }
            }
        }
        props
    }
    pub fn new(
        allocator: &'a Allocator,
        filename: &str,
        package: &str,
        css_files: Vec<String>,
        split_filename: Option<String>,
    ) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            filename: filename.to_string(),
            bindings: Bindings::default(),
            package: package.to_string(),
            compat_package: format!("{package}/compat"),
            css_files,
            styles: FxHashSet::default(),
            errors: Vec::new(),
            split_filename,
            class_names_scope: Vec::new(),
            stylex_var_refs: FxHashMap::default(),
            stylex_var_names: FxHashMap::default(),
            stylex_theme_classes: FxHashMap::default(),
            imported_stylex: (FxHashMap::default(), FxHashMap::default()),
            stylex_pending_vars: None,
            stylex_pending_theme_class: None,
            stylex_pending_consts: None,
            stylex_pending_create: None,
            stylex_namespaces: FxHashMap::default(),
            stylex_pending_keyframe_name: None,
            stylex_keyframe_names: FxHashMap::default(),
            pending_replacement: None,
            spreads_read_once: 0,
            runtime_types: 0,
            scoping: None,
            style_values: crate::style_values::StyleValues::default(),
            css_styles: None,
            pending_styled: None,
            selected_components: FxHashSet::default(),
            pending_marker: None,
            forwards_refs: false,
            styled_definitions: FxHashMap::default(),
            imported_css: FxHashMap::default(),
            imported_atoms: Default::default(),
            style_operand_mode: Default::default(),
            unknown_bindings: crate::imported_constants::Unknown::default(),
            composes_unknown: false,
            unknown_parts: Vec::new(),
            changed_bindings: crate::imported_constants::Changed::default(),
            css_prop: CssProp::Off,
            compiled_css_prop: false,
            local_styles: FxHashSet::default(),
        }
    }

    pub const fn takes_css_prop(&mut self, css_prop: CssProp) {
        self.css_prop = css_prop;
    }

    pub fn unknown_bindings(&mut self, unknown: &crate::imported_constants::Unknown) {
        self.unknown_bindings.clone_from(unknown);
    }

    /// The style function `callee` names: `css`, `Devup.keyframes`, ...
    fn util_type(&self, callee: &Expression<'a>) -> Option<Rc<UtilType>> {
        self.bindings.util(callee)
    }

    /// `node` with a `styled` read as a member of the package imported whole
    /// (`Devup.styled.div`) written as a binding no source can name, which
    /// the extraction of `styled` reads like the named import
    fn plain_styled(&self, node: &mut Expression<'a>) {
        if let Expression::StaticMemberExpression(member) = unwrap_syntax_only(node)
            && member.property.name == "styled"
            && let Expression::Identifier(object) = &member.object
        {
            if self.bindings.is_namespace(object) {
                *node = Expression::new_identifier(SPAN, NAMESPACE_STYLED, &self.ast);
            }
            return;
        }
        match unwrap_syntax_only_mut(node) {
            Expression::StaticMemberExpression(member) => self.plain_styled(&mut member.object),
            Expression::CallExpression(call) => self.plain_styled(&mut call.callee),
            _ => {}
        }
    }

    /// Report where `program` still reads what the build compiled away: its
    /// import is gone, so that code would throw at runtime
    fn report_compiled_reads(&mut self, program: &Program<'a>) {
        let reads = self.bindings.compiled_reads(program);
        self.report_reads(reads);
    }

    /// Report the reads `reads` tells of what the build compiled away
    fn report_reads(&mut self, reads: Vec<(u32, String)>) {
        for (offset, name) in reads {
            if !self.errors.iter().any(|(at, _)| *at == offset) {
                self.errors.push((
                    offset,
                    format!(
                        "`{name}` is read at runtime, where it does not exist: the build compiles it only where it is called or rendered"
                    ),
                ));
            }
        }
    }

    /// Report what `it`, a call or template the build compiles, reads of
    /// what the build compiled away: its styles are written from the code
    /// read, which the reads would leave as the names of nothing
    fn report_lowered_reads(&mut self, it: &Expression<'a>) {
        let callee = match it {
            Expression::CallExpression(call) => &call.callee,
            Expression::TaggedTemplateExpression(tag) => &tag.tag,
            _ => return,
        };
        if self.bindings.compiles(callee) {
            let reads = self.bindings.lowered_reads(it);
            self.report_reads(reads);
        }
    }

    /// Reuse the semantic analysis of the program as parsed, which constants
    /// inlined since then leave valid
    pub fn reuse_scoping(&mut self, scoping: Option<Rc<oxc_semantic::Scoping>>) {
        self.scoping = scoping;
    }

    /// The bindings of `program`: the analysis given, or one built when the
    /// program may bind what the build compiles
    fn scoping_of(&mut self, program: &Program<'a>) -> Option<Rc<oxc_semantic::Scoping>> {
        self.scoping.take().or_else(|| {
            let imports_api = program.body.iter().any(|statement| {
                matches!(statement, Statement::ImportDeclaration(import)
                    if import.source.value == self.package
                        || import.source.value == self.compat_package.as_str()
                        || import.source.value == "@stylexjs/stylex")
            });
            (imports_api
                || self.css_prop != CssProp::Off
                || crate::scope::requires(program, &self.package))
            .then(|| {
                Rc::new(
                    oxc_semantic::SemanticBuilder::new()
                        .build(program)
                        .semantic
                        .into_scoping(),
                )
            })
        })
    }

    /// Whether `program` imports a style function whose result it may bind:
    /// `css()` gives a class, `keyframes()` a name
    fn binds_style_results(&self, program: &Program<'a>) -> bool {
        program.body.iter().any(|statement| {
            matches!(statement, Statement::ImportDeclaration(import)
            if (import.source.value == self.package
                || import.source.value == self.compat_package.as_str())
                && import.specifiers.iter().flatten().any(|specifier| match specifier {
                    ImportSpecifier(specifier) => {
                        matches!(
                            specifier.imported.name().as_str(),
                            "css" | "keyframes" | "styled"
                        )
                    }
                    _ => true,
                }))
        })
    }

    /// Put the spreads of `props` that may change when read again, and the
    /// props written before them, in names read in their place, returning the
    /// names and what they read: the element reads a spread's `className` and
    /// `style` beside the spread
    fn read_spreads_once(&mut self, props: &mut Expression<'a>) -> Vec<(String, Expression<'a>)> {
        let mut read_once = Vec::new();
        let Expression::ObjectExpression(object) = props else {
            return read_once;
        };
        let suspends = |value: &Expression<'a>| {
            let mut suspends = Suspends::default();
            oxc_ast_visit::Visit::visit_expression(&mut suspends, value);
            suspends.found
        };
        let moves = |property: &ObjectPropertyKind<'a>| match property {
            ObjectPropertyKind::SpreadProperty(spread) => !is_pure(&spread.argument),
            ObjectPropertyKind::ObjectProperty(property) => {
                property_stays(property) && suspends(&property.value)
            }
        };
        let stuck = object.properties.iter().any(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(property)
                if (property.computed || !property_stays(property)) && suspends(&property.value))
        });
        let Some(last) = object.properties.iter().rposition(moves) else {
            return read_once;
        };
        if stuck
            || !object.properties.iter().any(|property| {
                matches!(property, ObjectPropertyKind::SpreadProperty(spread)
                    if !is_pure(&spread.argument))
            })
        {
            return read_once;
        }
        for property in object.properties.iter_mut().take(last + 1) {
            let value = match property {
                ObjectPropertyKind::SpreadProperty(spread) => &mut spread.argument,
                ObjectPropertyKind::ObjectProperty(property)
                    if !property.computed && property_stays(property) =>
                {
                    &mut property.value
                }
                ObjectPropertyKind::ObjectProperty(_) => continue,
            };
            if !is_pure(value) {
                read_once.push(self.read_once(value));
            }
        }
        read_once
    }

    fn read_once(&mut self, value: &mut Expression<'a>) -> (String, Expression<'a>) {
        let name = format!("__devupSpread{}", self.spreads_read_once);
        self.spreads_read_once += 1;
        let read = Expression::new_identifier(
            SPAN,
            Str::from_in(name.as_str(), self.ast.allocator()),
            &self.ast,
        );
        (name, std::mem::replace(value, read))
    }

    /// Whether the element `name` takes Emotion's `css` prop
    fn takes_css(&self, name: &JSXElementName<'a>) -> bool {
        self.css_prop
            .takes(name, |_| self.bindings.element(name).is_some())
    }

    /// Take the `css` prop off `element` when it takes one
    fn take_css_prop(&mut self, element: &mut JSXElement<'a>) -> Option<CssValue<'a>> {
        if !self.takes_css(&element.opening_element.name) {
            return None;
        }
        let mut written = None;
        element.opening_element.attributes.retain_mut(|attribute| {
            let JSXAttributeItem::Attribute(attribute) = attribute else {
                return true;
            };
            if !matches!(&attribute.name, Identifier(name) if name.name == "css") {
                return true;
            }
            written = Some((attribute.span.start, attribute.value.take()));
            false
        });
        let (offset, value) = written?;
        let value = match value {
            None => Expression::new_boolean_literal(SPAN, true, &self.ast),
            Some(JSXAttributeValue::StringLiteral(literal)) => Expression::StringLiteral(literal),
            Some(JSXAttributeValue::ExpressionContainer(mut container)) => {
                container.expression.as_expression_mut().map_or_else(
                    || Expression::new_identifier(SPAN, "undefined", &self.ast),
                    |value| value.take_in(&self.ast),
                )
            }
            Some(JSXAttributeValue::Element(element)) => Expression::JSXElement(element),
            Some(JSXAttributeValue::Fragment(fragment)) => Expression::JSXFragment(fragment),
        };
        Some(self.css_value(&element.opening_element.name.to_string(), value, offset))
    }

    /// The `css` prop `value` of `element`, written at `offset`, with what it
    /// composes written as parts, then visited
    fn css_value(&mut self, element: &str, mut value: Expression<'a>, offset: u32) -> CssValue<'a> {
        self.bindings.remember(&value);
        let reads = self.bindings.lowered_reads(&value);
        self.report_reads(reads);
        self.css_prop_value(element, &mut value);
        self.visit_expression(&mut value);
        self.compiled_css_prop = true;
        CssValue { value, offset }
    }

    /// `value`, a `css` prop of `element`, with what it composes written as
    /// parts: a `css()` call as the array of what it composes, CSS text a mixin
    /// stands in as the parts around the mixin, and a function of the theme as
    /// the rules it gives
    fn css_prop_value(&mut self, element: &str, value: &mut Expression<'a>) {
        let parts = match unwrap_syntax_only_mut(value) {
            Expression::ArrayExpression(array) => {
                for part in &mut array.elements {
                    if let Some(part) = part.as_expression_mut() {
                        self.css_prop_value(element, part);
                    }
                }
                return;
            }
            Expression::ConditionalExpression(conditional) => {
                self.css_prop_value(element, &mut conditional.consequent);
                self.css_prop_value(element, &mut conditional.alternate);
                return;
            }
            Expression::LogicalExpression(logical) => {
                if logical.operator != LogicalOperator::And {
                    self.css_prop_value(element, &mut logical.left);
                }
                self.css_prop_value(element, &mut logical.right);
                return;
            }
            Expression::CallExpression(call)
                if self
                    .util_type(&call.callee)
                    .is_some_and(|util| matches!(util.as_ref(), UtilType::Css)) =>
            {
                let elements = call.arguments.drain(..).map(|argument| match argument {
                    Argument::SpreadElement(spread) => {
                        ArrayExpressionElement::SpreadElement(spread)
                    }
                    argument => argument.into_expression().into(),
                });
                Expression::new_array_expression(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(elements, &self.ast),
                    &self.ast,
                )
            }
            Expression::TaggedTemplateExpression(tag)
                if self
                    .util_type(&tag.tag)
                    .is_some_and(|util| matches!(util.as_ref(), UtilType::Css)) =>
            {
                match template_parts(&self.ast, &tag.quasi, true) {
                    Ok(parts) => Expression::new_array_expression(
                        SPAN,
                        oxc_allocator::Vec::from_iter_in(
                            parts.into_iter().map(Into::into),
                            &self.ast,
                        ),
                        &self.ast,
                    ),
                    Err(unplaced) => self.css_prop_failure(element, unplaced),
                }
            }
            Expression::TemplateLiteral(template) => {
                match template_parts(&self.ast, template, false) {
                    Ok(_) => return,
                    Err(unplaced) => self.css_prop_failure(element, unplaced),
                }
            }
            function @ (Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)) => {
                match theme_rules_of(&self.ast, function, &|identifier| {
                    self.bindings.symbol(identifier)
                }) {
                    Ok(rules) => rules,
                    Err(unread) => self.css_prop_failure(element, unread),
                }
            }
            _ => return,
        };
        *value = parts;
        self.css_prop_value(element, value);
    }

    /// Report `code` of the `css` prop of `element`, which cannot be compiled
    /// for `requirement`, leaving nothing in its place
    fn css_prop_failure(
        &mut self,
        element: &str,
        (code, requirement): (Expression<'a>, &str),
    ) -> Expression<'a> {
        self.errors.push((
            code.span().start,
            css_prop_error(element, &readable_code(&code), requirement),
        ));
        Expression::new_identifier(SPAN, "undefined", &self.ast)
    }

    /// The first part a `css` prop `value` composes for which `test` holds
    fn css_part<'b>(
        &self,
        value: &'b Expression<'a>,
        test: &impl Fn(&Self, &Expression<'a>) -> bool,
    ) -> Option<&'b Expression<'a>> {
        match unwrap_syntax_only(value) {
            Expression::ArrayExpression(array) => array
                .elements
                .iter()
                .filter_map(ArrayExpressionElement::as_expression)
                .find_map(|part| self.css_part(part, test)),
            Expression::ConditionalExpression(conditional) => self
                .css_part(&conditional.consequent, test)
                .or_else(|| self.css_part(&conditional.alternate, test)),
            Expression::LogicalExpression(logical) => (logical.operator != LogicalOperator::And)
                .then(|| self.css_part(&logical.left, test))
                .flatten()
                .or_else(|| self.css_part(&logical.right, test)),
            part => test(self, part).then_some(part),
        }
    }

    /// Whether `part` reads a binding `local_styles` holds
    fn reads_local_styles(&self, part: &Expression<'a>) -> bool {
        let mut root = part;
        while let Expression::StaticMemberExpression(member) = root {
            root = &member.object;
        }
        if let Expression::ComputedMemberExpression(member) = root {
            root = &member.object;
        }
        self.style_values
            .symbol(root)
            .is_some_and(|symbol| self.local_styles.contains(&symbol))
    }

    /// The styles `earlier` and the `css` prop of `element` compose, the
    /// prop's replacing what `earlier` sets, then what the classes
    /// `class_name` always holds set replacing both
    fn compose_css_prop(
        &mut self,
        element: &str,
        css: CssValue<'a>,
        earlier: Vec<ExtractStyleProp<'a>>,
        class_name: Option<&Expression<'a>>,
    ) -> Vec<ExtractStyleProp<'a>> {
        let CssValue { value, offset } = css;
        let mut parts = Vec::new();
        let unusable = if let Some(part) = self.css_part(&value, &Self::reads_local_styles) {
            Some((part, LOCAL_STYLES))
        } else if self.known_parts(&value, &mut parts, Text::Rules).is_none() {
            Some((
                self.css_part(&value, &|visitor, part| {
                    visitor.known_side(part, Text::Rules).is_none()
                })
                .unwrap_or(&value),
                CSS_PROP_VALUE,
            ))
        } else {
            None
        };
        if let Some((part, requirement)) = unusable {
            let at = if part.span().is_unspanned() {
                offset
            } else {
                part.span().start
            };
            self.errors.push((
                at,
                css_prop_error(element, &readable_code(part), requirement),
            ));
            return earlier;
        }
        let unknown = self.css_part(&value, &|visitor, part| {
            visitor.unknown_bindings.read_by_in(part, &|identifier| {
                visitor.bindings.reads_module(identifier)
            })
        });
        if let Some(part) = unknown {
            self.composes_unknown = true;
            self.unknown_parts.push((
                part.span().start,
                css_prop_error(element, &readable_code(part), STYLE_OBJECT),
            ));
        }
        let changed = self.css_part(&value, &|visitor, part| {
            visitor.changed_bindings.read_by_in(part, &|identifier| {
                visitor.bindings.reads_module(identifier)
            })
        });
        if let Some(part) = changed {
            self.errors.push((
                part.span().start,
                css_prop_error(element, &readable_code(part), STYLE_OBJECT),
            ));
        }

        let mut composition = crate::composition::Composition::default();
        composition.apply(&self.ast, earlier);
        let mut classes = Vec::new();
        for part in parts {
            match part {
                KnownPart::Styles(side) => {
                    let props = self.part_props(offset, side, Some(element));
                    composition.apply(&self.ast, props);
                }
                KnownPart::Conditional {
                    test,
                    consequent,
                    alternate,
                } => {
                    let consequent = self.part_props(offset, consequent, Some(element));
                    let alternate = self.part_props(offset, alternate, Some(element));
                    composition.apply_conditional(&self.ast, &test, consequent, alternate);
                }
                KnownPart::Class(mut class) => {
                    self.style_values.read_in(&self.ast, &mut class);
                    classes.push(ExtractStyleProp::Expression {
                        expression: class,
                        styles: vec![],
                    });
                }
            }
        }
        if let Some(class_name) = class_name {
            composition.cover(&self.class_styles(class_name));
        }
        let mut props = composition.into_props();
        props.extend(classes);
        props
    }

    /// The styles of the classes the file knows that `class_name` always holds
    fn class_styles(&self, class_name: &Expression<'a>) -> Vec<ExtractStyleValue> {
        match unwrap_syntax_only(class_name) {
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                let mut styles = self.class_styles(&binary.left);
                styles.extend(self.class_styles(&binary.right));
                styles
            }
            Expression::TemplateLiteral(template) => template
                .expressions
                .iter()
                .flat_map(|class| self.class_styles(class))
                .collect(),
            class => self
                .style_values
                .styles(class)
                .map(<[ExtractStyleValue]>::to_vec)
                .unwrap_or_default(),
        }
    }

    /// The classes and CSS variables `props` compile to, which an element
    /// outside Devup UI takes as its `className` and `style`
    fn css_class_and_style(
        &mut self,
        mut props: Vec<ExtractStyleProp<'a>>,
    ) -> (Option<Expression<'a>>, Option<Expression<'a>>) {
        // Class names come out in reverse, so they read in composing order
        props.reverse();
        let filename = self.split_filename.as_deref();
        let class_name = gen_class_names(&self.ast, &mut props, None, filename);
        let style = gen_styles(&self.ast, &props, filename);
        self.styles
            .extend(props.into_iter().flat_map(ExtractStyleProp::into_extract));
        (class_name, style)
    }

    /// The styles the `css` prop of `element` gives it. When the element is a
    /// styled component the file binds to `symbol`, it renders the tag it
    /// renders in its place if `renders`, which comes with them, its styles
    /// composed under the prop's; `None` when the build cannot order them.
    fn css_prop_styles(
        &mut self,
        element: &str,
        css: CssValue<'a>,
        symbol: Option<oxc_syntax::symbol::SymbolId>,
        renders: bool,
        class_name: Option<&Expression<'a>>,
    ) -> Option<(Option<String>, Vec<ExtractStyleProp<'a>>)> {
        let allocator = self.ast.allocator();
        let clone = |styles: &[ExtractStyleProp<'a>]| -> Vec<ExtractStyleProp<'a>> {
            styles
                .iter()
                .map(|style| style.clone_in(allocator))
                .collect()
        };
        let definition = symbol.and_then(|symbol| self.styled_definitions.get(&symbol));
        let inline = definition
            .filter(|_| renders)
            .and_then(StyledDefinition::inline)
            .map(|(tag, styles)| (tag.to_string(), clone(styles)));
        let marker = definition.and_then(StyledDefinition::marker).map(|marker| {
            ExtractStyleProp::Expression {
                expression: Expression::new_string_literal(
                    SPAN,
                    Str::from_in(marker, allocator),
                    None,
                    &self.ast,
                ),
                styles: vec![],
            }
        });
        let own = definition.map(|definition| clone(definition.styles()));
        if let Some((tag, styles)) = inline {
            let mut props = self.compose_css_prop(element, css, styles, class_name);
            props.extend(marker);
            return Some((Some(tag), props));
        }
        let offset = css.offset;
        let props = self.compose_css_prop(element, css, vec![], class_name);
        if own.is_some_and(|own| overlaps(&own, &props)) {
            self.errors.push((offset, css_prop_override_error(element)));
            return None;
        }
        Some((None, props))
    }

    /// Compile the `css` prop of `element`, an element outside Devup UI
    fn lower_css_prop(&mut self, element: &mut JSXElement<'a>, css: CssValue<'a>) {
        let name = element.opening_element.name.to_string();
        let attributes = &element.opening_element.attributes;
        let class_name = written_class_name(&self.ast, attributes);
        let renders = attributes.iter().all(|attribute| match attribute {
            JSXAttributeItem::Attribute(attribute) => !matches!(&attribute.name,
                Identifier(name) if name.name == "as" || name.name == "forwardedAs"),
            JSXAttributeItem::SpreadAttribute(_) => false,
        });
        let symbol = match &element.opening_element.name {
            JSXElementName::IdentifierReference(reference) => {
                self.style_values.reference_symbol(reference)
            }
            _ => None,
        };
        let Some((tag, props)) =
            self.css_prop_styles(&name, css, symbol, renders, class_name.as_ref())
        else {
            return;
        };
        if let Some(tag) = tag {
            crate::as_visit::rename(
                &self.ast,
                element,
                JSXElementName::new_identifier(
                    SPAN,
                    Str::from_in(tag.as_str(), self.ast.allocator()),
                    &self.ast,
                ),
            );
        }
        let (class_name, style) = self.css_class_and_style(props);
        add_class_and_style(
            &self.ast,
            &mut element.opening_element.attributes,
            class_name,
            style,
        );
    }

    /// Take the `css` prop off the props a `jsx()` call gives when the element
    /// it builds takes one; `devup` tells a Devup UI component
    fn take_jsx_css_prop(
        &mut self,
        call: &mut CallExpression<'a>,
        devup: bool,
    ) -> Option<JsxCss<'a>> {
        let element = call.arguments.first()?.as_expression()?;
        let takes = match self.css_prop {
            CssProp::Off => false,
            css_prop => devup || css_prop.takes_type(element, |_| false),
        };
        if !takes {
            return None;
        }
        let name = match unwrap_syntax_only(element) {
            Expression::StringLiteral(literal) => literal.value.to_string(),
            element => readable_code(element),
        };
        let symbol = self.style_values.symbol(unwrap_syntax_only(element));
        let Some(Argument::ObjectExpression(props)) = call.arguments.get_mut(1) else {
            return None;
        };
        let mut written = None;
        let ast = &self.ast;
        props.properties.retain_mut(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return true;
            };
            if property.computed || property.key.static_name().is_none_or(|key| key != "css") {
                return true;
            }
            written = Some((property.span.start, property.value.take_in(ast)));
            false
        });
        let (offset, value) = written?;
        let class_name = written_object_class_name(&self.ast, &props.properties);
        let renders = props.properties.iter().all(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => {
                !property.computed
                    && property
                        .key
                        .static_name()
                        .is_none_or(|key| key != "as" && key != "forwardedAs")
            }
            ObjectPropertyKind::SpreadProperty(_) => false,
        });
        let css = self.css_value(&name, value, offset);
        Some(JsxCss {
            css,
            element: name,
            symbol,
            class_name,
            renders,
        })
    }

    /// Compile the `css` prop a `jsx()` call building an element outside
    /// Devup UI gives
    fn lower_jsx_css_prop(&mut self, call: &mut CallExpression<'a>, jsx: JsxCss<'a>) {
        let JsxCss {
            css,
            element,
            symbol,
            class_name,
            renders,
        } = jsx;
        let Some((tag, props)) =
            self.css_prop_styles(&element, css, symbol, renders, class_name.as_ref())
        else {
            return;
        };
        if let Some(tag) = tag {
            call.arguments[0] = Argument::from(Expression::new_string_literal(
                SPAN,
                Str::from_in(tag.as_str(), self.ast.allocator()),
                None,
                &self.ast,
            ));
        }
        let (class_name, style) = self.css_class_and_style(props);
        if let Some(Argument::ObjectExpression(props)) = call.arguments.get_mut(1) {
            add_class_and_style_to_object(&self.ast, &mut props.properties, class_name, style);
        }
    }

    /// What Emotion's `<ClassNames>` element renders, its classes compiled;
    /// `None` for any other element
    fn class_names(&mut self, element: &mut JSXElement<'a>) -> Option<Expression<'a>> {
        let JSXElementName::IdentifierReference(name) = &element.opening_element.name else {
            return None;
        };
        if !self.bindings.is_class_names(name) {
            return None;
        }
        Some(
            self.compile_class_names(element)
                .unwrap_or_else(|(at, code)| {
                    self.errors
                        .push((at, element_error("ClassNames", &code, CLASS_NAMES_CHILD)));
                    Expression::new_null_literal(SPAN, &self.ast)
                }),
        )
    }

    /// `<ClassNames>{({ css, cx, theme }) => rendered}</ClassNames>` as
    /// `rendered`: its `css` and `cx` calls compile as `css()`, and the theme
    /// they read becomes the CSS variables the `ThemeProvider` sets. `Err`
    /// holds where and what the build cannot compile.
    fn compile_class_names(
        &mut self,
        element: &mut JSXElement<'a>,
    ) -> Result<Expression<'a>, (u32, String)> {
        if let Some(attribute) = element.opening_element.attributes.first() {
            return Err((attribute.span().start, "an attribute".to_string()));
        }
        let span = element.span;
        let mut children = element
            .children
            .iter_mut()
            .filter(|child| !matches!(child, JSXChild::Text(text) if text.value.trim().is_empty()));
        let (Some(JSXChild::ExpressionContainer(container)), None) =
            (children.next(), children.next())
        else {
            return Err((span.start, "children".to_string()));
        };
        let Some(function) = container.expression.as_expression_mut() else {
            return Err((container.span.start, "{}".to_string()));
        };
        let function_at = function.span().start;
        let function_code = readable_code(function);
        let Some((names, symbols, body)) = render_function(function).and_then(|(params, body)| {
            Some((
                class_names_params(params)?,
                ClassNamesSymbols::of(params),
                body,
            ))
        }) else {
            return Err((function_at, function_code));
        };
        let mut rendered = body.take_in(&self.ast);
        let mut calls = ClassNamesCalls {
            ast: &self.ast,
            bindings: &self.bindings,
            names,
            symbols,
            unread: None,
        };
        calls.visit_expression(&mut rendered);
        if let Some((unread, requirement)) = calls.unread {
            self.errors.push((
                unread.span().start,
                element_error("ClassNames", &readable_code(&unread), requirement),
            ));
        }
        self.class_names_scope.push(symbols);
        self.visit_expression(&mut rendered);
        self.class_names_scope.pop();
        Ok(rendered)
    }

    /// What a call of the `css` or `cx` a `<ClassNames>` child function
    /// takes composes: `css` reads text as CSS, `cx` as classes
    fn class_names_text(&self, callee: &Expression<'a>) -> Option<Text> {
        let Expression::Identifier(callee) = callee else {
            return None;
        };
        let symbol = self.bindings.symbol(callee);
        self.class_names_scope.iter().rev().find_map(|names| {
            if ClassNamesSymbols::is(names.css, symbol) {
                Some(Text::Rules)
            } else if ClassNamesSymbols::is(names.cx, symbol) {
                Some(Text::Classes)
            } else {
                None
            }
        })
    }

    /// Replace `it`, when a call of the `css` or `cx` a `<ClassNames>` child
    /// function takes, with the classes it composes
    fn compile_class_names_call(&mut self, it: &mut Expression<'a>) -> bool {
        let Expression::CallExpression(call) = it else {
            return false;
        };
        if self.class_names_text(&call.callee).is_none() {
            return false;
        }
        let offset = call.span.start;
        self.unknown_arguments("css", &call.arguments);
        self.changed_arguments("css", &call.arguments);
        let mut parts = Vec::new();
        *it = if self.known_parts(it, &mut parts, Text::Classes).is_some() {
            self.composed_class(offset, parts).0
        } else {
            self.errors.push((
                offset,
                element_error("ClassNames", &readable_code(it), CLASS_NAMES_PART),
            ));
            Expression::new_string_literal(SPAN, "", None, &self.ast)
        };
        true
    }
}

/// The calls of the `css` and `cx` a `<ClassNames>` child function takes,
/// readied to compose: the theme their rules read becomes the CSS variables
/// the `ThemeProvider` sets, an object `cx` takes the conditions of its
/// classes, and CSS text `css` tags the parts around its mixins. The first
/// code the build cannot ready so is kept.
struct ClassNamesCalls<'r, 'a> {
    ast: &'r AstBuilder<'a>,
    bindings: &'r Bindings,
    names: ClassNamesParams<'a>,
    symbols: ClassNamesSymbols,
    unread: Option<(Expression<'a>, &'static str)>,
}

impl<'a> ClassNamesCalls<'_, 'a> {
    fn calls(&self, callee: &Expression<'a>, slot: Option<SymbolId>) -> bool {
        matches!(callee, Expression::Identifier(callee)
            if ClassNamesSymbols::is(slot, self.bindings.symbol(callee)))
    }

    fn read_theme(&mut self, expression: &mut Expression<'a>, value: bool) {
        let Some(name) = self.names.theme else {
            return;
        };
        let theme = ThemeRoot {
            name,
            symbol: self.symbols.theme,
            key: None,
        };
        let bindings = self.bindings;
        if let Err(unread) = read_theme(self.ast, expression, theme, value, &|identifier| {
            bindings.symbol(identifier)
        }) {
            self.unread.get_or_insert((unread, THEME_READ));
        }
    }

    /// `part` of what `cx` composes with each object in it written as the
    /// conditions of the classes it names
    fn class_maps(&mut self, part: &mut Expression<'a>) {
        let classes = match unwrap_syntax_only_mut(part) {
            Expression::ArrayExpression(array) => {
                for part in &mut array.elements {
                    if let Some(part) = part.as_expression_mut() {
                        self.class_maps(part);
                    }
                }
                return;
            }
            Expression::ConditionalExpression(conditional) => {
                self.class_maps(&mut conditional.consequent);
                self.class_maps(&mut conditional.alternate);
                return;
            }
            Expression::LogicalExpression(logical) => {
                if logical.operator != LogicalOperator::And {
                    self.class_maps(&mut logical.left);
                }
                self.class_maps(&mut logical.right);
                return;
            }
            Expression::ObjectExpression(object) => self.class_map(object),
            _ => return,
        };
        *part = classes;
    }

    /// `{ name: condition }` as `[condition && 'name']`
    fn class_map(&mut self, object: &mut ObjectExpression<'a>) -> Expression<'a> {
        let mut classes = oxc_allocator::Vec::new_in(self.ast);
        for property in object.properties.drain(..) {
            let mut property = match property {
                ObjectPropertyKind::ObjectProperty(property)
                    if property.kind == PropertyKind::Init && !property.method =>
                {
                    property
                }
                ObjectPropertyKind::ObjectProperty(mut property) => {
                    let value = property.value.take_in(self.ast);
                    self.unread.get_or_insert((value, CLASS_NAMES_CLASS_MAP));
                    continue;
                }
                ObjectPropertyKind::SpreadProperty(mut spread) => {
                    let argument = spread.argument.take_in(self.ast);
                    self.unread.get_or_insert((argument, CLASS_NAMES_CLASS_MAP));
                    continue;
                }
            };
            let class = if property.computed
                && let Some(key) = property.key.as_expression_mut()
            {
                key.take_in(self.ast)
            } else {
                let name = property.key.static_name().unwrap_or_default();
                Expression::new_string_literal(
                    SPAN,
                    Str::from_in(name.as_ref(), self.ast.allocator()),
                    None,
                    self.ast,
                )
            };
            let condition = property.value.take_in(self.ast);
            classes.push(
                Expression::new_logical_expression(
                    SPAN,
                    condition,
                    LogicalOperator::And,
                    class,
                    self.ast,
                )
                .into(),
            );
        }
        Expression::new_array_expression(SPAN, classes, self.ast)
    }
}

impl<'a> VisitMut<'a> for ClassNamesCalls<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let Expression::TaggedTemplateExpression(tagged) = it else {
            walk_mut::walk_expression(self, it);
            return;
        };
        if !self.calls(&tagged.tag, self.symbols.css) {
            walk_mut::walk_expression(self, it);
            return;
        }
        for expression in &mut tagged.quasi.expressions {
            self.read_theme(expression, true);
            self.visit_expression(expression);
        }
        *it = match template_parts(self.ast, &tagged.quasi, true) {
            Ok(parts) => Expression::new_call_expression(
                SPAN,
                tagged.tag.take_in(self.ast),
                None::<oxc_allocator::Box<'_, oxc_ast::ast::TSTypeParameterInstantiation<'_>>>,
                oxc_allocator::Vec::from_iter_in(parts.into_iter().map(Argument::from), self.ast),
                false,
                self.ast,
            ),
            Err(unplaced) => {
                self.unread.get_or_insert(unplaced);
                Expression::new_identifier(SPAN, "undefined", self.ast)
            }
        };
    }

    fn visit_call_expression(&mut self, call: &mut CallExpression<'a>) {
        let cx = self.calls(&call.callee, self.symbols.cx);
        if !cx && !self.calls(&call.callee, self.symbols.css) {
            walk_call_expression(self, call);
            return;
        }
        for argument in &mut call.arguments {
            let part = match argument {
                Argument::SpreadElement(spread) => &mut spread.argument,
                argument => argument.to_expression_mut(),
            };
            if cx {
                self.class_maps(part);
            }
            self.read_theme(part, false);
            self.visit_expression(part);
        }
    }

    /// A read of the theme or of `css` and `cx` the calls do not take
    fn visit_identifier_reference(&mut self, it: &mut oxc_ast::ast::IdentifierReference<'a>) {
        let symbol = self.bindings.symbol(it);
        let requirement = if ClassNamesSymbols::is(self.symbols.theme, symbol) {
            THEME_READ
        } else if ClassNamesSymbols::is(self.symbols.css, symbol)
            || ClassNamesSymbols::is(self.symbols.cx, symbol)
        {
            CLASS_NAMES_CALL
        } else {
            return;
        };
        self.unread.get_or_insert_with(|| {
            (
                Expression::new_identifier(it.span, it.name, self.ast),
                requirement,
            )
        });
    }
}

impl<'a> DevupVisitor<'a> {
    /// Resolve a `stylex.props(...)` / `stylex.attrs(...)` callee to the class attribute
    /// it produces. `props()` targets React (`className`), `attrs()` targets raw HTML
    /// (`class`); everything else about the two calls is identical.
    fn stylex_class_attribute(&self, callee: &Expression) -> Option<&'static str> {
        match self.bindings.stylex_function(callee) {
            Some(StylexFunction::Props) => Some("className"),
            Some(StylexFunction::Attrs) => Some("class"),
            _ => None,
        }
    }

    /// The styles behind the `css()` classes the program imports from other
    /// modules, by the name it binds them to
    pub fn import_css(&mut self, styles: FxHashMap<String, Vec<ExtractStyleValue>>) {
        self.imported_css = styles;
    }

    pub(crate) fn import_producer_atoms(
        &mut self,
        atoms: crate::vanilla_extract::producer_atoms::ProducerAtoms,
    ) {
        self.imported_atoms = atoms;
    }

    /// `StyleX` variables and themes the program imports from other modules,
    /// by the name it binds them to
    pub fn import_stylex(
        &mut self,
        vars: FxHashMap<String, FxHashMap<String, String>>,
        themes: FxHashMap<String, String>,
    ) {
        self.imported_stylex = (vars, themes);
    }

    /// Read the `StyleX` variables and themes the program imports as the
    /// bindings that hold them
    fn bind_imported_stylex(&mut self) {
        let (vars, themes) = std::mem::take(&mut self.imported_stylex);
        for (name, contract) in vars {
            if let Some(symbol) = self.bindings.imported(&name) {
                self.stylex_var_refs.entry(symbol).or_default().extend(
                    contract
                        .iter()
                        .map(|(key, variable)| (key.clone(), format!("var({variable})"))),
                );
                self.stylex_var_names.insert(symbol, contract.clone());
            }
        }
        for (name, class) in themes {
            if let Some(symbol) = self.bindings.imported(&name) {
                self.stylex_theme_classes.insert(symbol, class);
            }
        }
    }

    /// The keyframe names and `defineVars` members, as `vars.key`, that the
    /// bindings `visible` to a `stylex.create()` call stand for
    fn stylex_names(
        &self,
        visible: &FxHashMap<String, SymbolId>,
    ) -> (FxHashMap<String, String>, FxHashMap<String, String>) {
        let keyframes = visible
            .iter()
            .filter_map(|(name, symbol)| {
                Some((
                    name.clone(),
                    self.stylex_keyframe_names.get(symbol)?.clone(),
                ))
            })
            .collect();
        let vars = visible
            .iter()
            .filter_map(|(name, symbol)| Some((name, self.stylex_var_refs.get(symbol)?)))
            .flat_map(|(name, members)| {
                members
                    .iter()
                    .map(move |(key, value)| (format!("{name}.{key}"), value.clone()))
            })
            .collect();
        (keyframes, vars)
    }

    /// The namespaces of the `stylex.create()` binding `object` reads
    fn stylex_namespace(
        &self,
        object: &Expression<'a>,
    ) -> Option<&FxHashMap<String, StylexNamespaceValue>> {
        let Expression::Identifier(object) = object else {
            return None;
        };
        self.stylex_namespaces.get(&self.bindings.symbol(object)?)
    }

    /// The key of an entry of a `StyleX` object, or `None` after reporting a
    /// spread or a key known only at runtime
    fn stylex_key<'p>(
        &mut self,
        api: &str,
        prop: &'p ObjectPropertyKind<'a>,
    ) -> Option<(String, &'p ObjectProperty<'a>)> {
        let prop = match prop {
            ObjectPropertyKind::ObjectProperty(prop) => prop,
            ObjectPropertyKind::SpreadProperty(spread) => {
                self.errors.push(spread_error(api, spread));
                return None;
            }
        };
        let Some(key) = get_string_by_property_key(&prop.key) else {
            self.errors.push(key_error(api, &prop.key));
            return None;
        };
        Some((key, prop))
    }

    fn string_property(&self, key: &str, value: &str) -> ObjectPropertyKind<'a> {
        ObjectPropertyKind::new_object_property(
            SPAN,
            PropertyKind::Init,
            PropertyKey::StringLiteral(StringLiteral::boxed(
                SPAN,
                Str::from_in(key, self.ast.allocator()),
                None,
                &self.ast,
            )),
            Expression::new_string_literal(
                SPAN,
                Str::from_in(value, self.ast.allocator()),
                None,
                &self.ast,
            ),
            false,
            false,
            false,
            &self.ast,
        )
    }

    fn is_stylex_call(&self, callee: &Expression, function: &StylexFunction) -> bool {
        self.bindings.stylex_function(callee).as_ref() == Some(function)
    }

    /// Resolve `stylex.props()` arguments to className expressions and style properties.
    /// Returns (`class_exprs`, `style_props`) where `style_props` are CSS variable assignments
    /// from dynamic namespace calls like `styles.bar(h)`.
    fn resolve_stylex_props_args(
        &self,
        arguments: &oxc_allocator::Vec<'a, Argument<'a>>,
    ) -> (Vec<Expression<'a>>, Vec<ObjectPropertyKind<'a>>) {
        let mut class_exprs: Vec<Expression<'a>> = vec![];
        let mut style_props: Vec<ObjectPropertyKind<'a>> = vec![];

        for arg in arguments {
            // `...spread` carries no statically resolvable namespace reference,
            // so its values join at runtime
            let Some(expr) = arg.as_expression() else {
                if let Argument::SpreadElement(spread) = arg {
                    class_exprs.push(runtime_classes(&self.ast, &spread.argument));
                }
                continue;
            };
            // Check for dynamic namespace call first: styles.bar(h)
            if let Expression::CallExpression(call) = expr
                && let Some((class_expr, props)) = self.resolve_stylex_dynamic_call(call)
            {
                class_exprs.push(class_expr);
                style_props.extend(props);
                continue;
            }
            if let Some(class_expr) = self.resolve_stylex_arg(expr) {
                class_exprs.push(class_expr);
            }
        }

        (class_exprs, style_props)
    }

    /// `((v) => typeof v === "number" ? v + unit : v)(value)`: how `StyleX` gives a
    /// dynamic number its unit without evaluating `value` twice.
    fn with_number_unit(&self, value: Expression<'a>, unit: &'static str) -> Expression<'a> {
        if unit.is_empty() {
            return value;
        }
        let param = || Expression::new_identifier(SPAN, "v", &self.ast);
        let body = Expression::new_conditional_expression(
            SPAN,
            Expression::new_binary_expression(
                SPAN,
                Expression::new_unary_expression(SPAN, UnaryOperator::Typeof, param(), &self.ast),
                BinaryOperator::StrictEquality,
                Expression::new_string_literal(SPAN, "number", None, &self.ast),
                &self.ast,
            ),
            Expression::new_binary_expression(
                SPAN,
                param(),
                BinaryOperator::Addition,
                Expression::new_string_literal(SPAN, unit, None, &self.ast),
                &self.ast,
            ),
            param(),
            &self.ast,
        );
        let mut params = oxc_allocator::Vec::new_in(&self.ast);
        params.push(FormalParameter::new(
            SPAN,
            oxc_allocator::Vec::new_in(&self.ast),
            BindingPattern::new_binding_identifier(SPAN, "v", &self.ast),
            None,
            None,
            false,
            None,
            false,
            false,
            &self.ast,
        ));
        let arrow = Expression::new_arrow_function_expression(
            SPAN,
            false,
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
            FormalParameters::boxed(
                SPAN,
                FormalParameterKind::ArrowFormalParameters,
                params,
                None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
                &self.ast,
            ),
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
            body.into(),
            &self.ast,
        );
        let mut arguments = oxc_allocator::Vec::new_in(&self.ast);
        arguments.push(Argument::from(value));
        Expression::new_call_expression(
            SPAN,
            Expression::new_parenthesized_expression(SPAN, arrow, &self.ast),
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
            arguments,
            false,
            &self.ast,
        )
    }
    /// Resolve a dynamic namespace call like `styles.bar(h)` to (className, `style_props`).
    fn resolve_stylex_dynamic_call(
        &self,
        call: &CallExpression<'a>,
    ) -> Option<(Expression<'a>, Vec<ObjectPropertyKind<'a>>)> {
        if let Expression::StaticMemberExpression(member) = &call.callee
            && let Some(ns_map) = self.stylex_namespace(&member.object)
            && let Some(StylexNamespaceValue::Dynamic(info)) =
                ns_map.get(member.property.name.as_str())
        {
            let class_expr = Expression::new_string_literal(
                SPAN,
                Str::from_in(&info.class_name, self.ast.allocator()),
                None,
                &self.ast,
            );

            let mut props = Vec::with_capacity(info.css_vars.len());
            for (param_idx, var_name, unit) in &info.css_vars {
                if let Some(arg) = call.arguments.get(*param_idx)
                    && let Some(arg_expr) = arg.as_expression()
                {
                    let arg_expr = self.with_number_unit(
                        arg_expr.clone_in_with_semantic_ids(self.ast.allocator()),
                        unit,
                    );
                    props.push(ObjectPropertyKind::new_object_property(
                        SPAN,
                        PropertyKind::Init,
                        PropertyKey::StringLiteral(StringLiteral::boxed(
                            SPAN,
                            Str::from_in(var_name, self.ast.allocator()),
                            None,
                            &self.ast,
                        )),
                        arg_expr,
                        false,
                        false,
                        false,
                        &self.ast,
                    ));
                }
            }

            Some((class_expr, props))
        } else {
            None
        }
    }

    /// What a global-CSS call collapses to once its rules are extracted: `createGlobalStyle`
    /// callers render the result (`<GlobalStyle />`), so it must stay a component, while
    /// `globalCss` is a bare statement and leaves nothing behind.
    fn global_css_result(&self, is_component: bool) -> Expression<'a> {
        if is_component {
            Expression::new_arrow_function_expression(
                SPAN,
                false,
                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
                FormalParameters::boxed(
                    SPAN,
                    FormalParameterKind::ArrowFormalParameters,
                    oxc_allocator::Vec::new_in(&self.ast),
                    None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
                    &self.ast,
                ),
                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
                Expression::new_null_literal(SPAN, &self.ast).into(),
                &self.ast,
            )
        } else {
            Expression::new_unary_expression(
                SPAN,
                UnaryOperator::Void,
                Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, &self.ast),
                &self.ast,
            )
        }
    }

    /// Build a className string literal for a resolved static `StyleX` namespace.
    /// An empty namespace (`stylex.create({ empty: {} })`) contributes nothing.
    fn stylex_class_literal(&self, class_name: &str) -> Option<Expression<'a>> {
        (!class_name.is_empty()).then(|| {
            Expression::new_string_literal(
                SPAN,
                Str::from_in(class_name, self.ast.allocator()),
                None,
                &self.ast,
            )
        })
    }

    /// Resolve a dotted namespace access (`styles.base`) to a className expression.
    fn resolve_stylex_static_member(
        &self,
        member: &StaticMemberExpression<'a>,
    ) -> Option<Expression<'a>> {
        if let Some(ns_map) = self.stylex_namespace(&member.object)
            && let Some(StylexNamespaceValue::Static(cn)) =
                ns_map.get(member.property.name.as_str())
        {
            return self.stylex_class_literal(cn);
        }
        None
    }

    /// Resolve a computed namespace access (`styles[key]`) to a className expression.
    ///
    /// A literal key (`styles['base']`) folds to the same string literal as
    /// `styles.base`. A non-literal key (`colorStyles[color]`) cannot be resolved at
    /// build time, so it defers to a runtime lookup on the declaration `stylex.create()`
    /// was rewritten into — `colorStyles[color] || ""` — instead of dropping the
    /// argument and leaving the generated atoms unreferenced.
    fn resolve_stylex_computed_member(
        &self,
        member: &ComputedMemberExpression<'a>,
    ) -> Option<Expression<'a>> {
        let ns_map = self.stylex_namespace(&member.object)?;

        if let Some(key) = get_string_by_literal_expression(&member.expression) {
            return match ns_map.get(key.as_ref()) {
                Some(StylexNamespaceValue::Static(cn)) => self.stylex_class_literal(cn),
                _ => None,
            };
        }

        // A dynamic namespace only yields its CSS variables when called
        // (`styles[key](x)`), so a variable holding nothing else is unresolvable here.
        if !ns_map
            .values()
            .any(|value| matches!(value, StylexNamespaceValue::Static(cn) if !cn.is_empty()))
        {
            return None;
        }

        // `|| ""` guards keys with no matching namespace, which would otherwise
        // interpolate `undefined` into the className.
        Some(convert_class_name(
            &self.ast,
            &Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
                SPAN,
                member
                    .object
                    .clone_in_with_semantic_ids(self.ast.allocator()),
                member
                    .expression
                    .clone_in_with_semantic_ids(self.ast.allocator()),
                member.optional,
                &self.ast,
            )),
        ))
    }

    /// Resolve a single `stylex.props()` argument to a className expression.
    fn resolve_stylex_arg(&self, expr: &Expression<'a>) -> Option<Expression<'a>> {
        let unwrapped = unwrap_syntax_only(expr);
        let compiled = match unwrapped {
            // styles.base → StaticMemberExpression
            Expression::StaticMemberExpression(member) => self.resolve_stylex_static_member(member),
            // colorStyles[color] / styles['base'] → ComputedMemberExpression
            Expression::ComputedMemberExpression(member) => {
                self.resolve_stylex_computed_member(member)
            }
            // darkTheme → Identifier bound to a stylex.createTheme() class
            Expression::Identifier(ident) if ident.name != "undefined" => self
                .bindings
                .symbol(ident)
                .and_then(|symbol| self.stylex_theme_classes.get(&symbol))
                .and_then(|class_name| self.stylex_class_literal(class_name)),
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::StaticMemberExpression(member) => {
                    self.resolve_stylex_static_member(member)
                }
                ChainElement::ComputedMemberExpression(member) => {
                    self.resolve_stylex_computed_member(member)
                }
                _ => None,
            },
            _ => return self.resolve_stylex_composed_arg(expr),
        };
        // This file's namespaces are fully known: what did not resolve adds no
        // class. Styles compiled elsewhere (a prop, another module's `create()`)
        // hold class names, which join at runtime
        if compiled.is_some() || self.reads_local_namespace(unwrapped) {
            return compiled;
        }
        Some(runtime_classes(&self.ast, expr))
    }

    fn reads_local_namespace(&self, expr: &Expression<'a>) -> bool {
        let object = match expr {
            Expression::StaticMemberExpression(member) => &member.object,
            Expression::ComputedMemberExpression(member) => &member.object,
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::StaticMemberExpression(member) => &member.object,
                ChainElement::ComputedMemberExpression(member) => &member.object,
                _ => return false,
            },
            _ => expr,
        };
        self.stylex_namespace(object).is_some()
    }

    fn resolve_stylex_composed_arg(&self, expr: &Expression<'a>) -> Option<Expression<'a>> {
        match unwrap_syntax_only(expr) {
            // stylex.props([a, b]) → StyleXArray, nestable to any depth
            Expression::ArrayExpression(array) => merge_expression_for_class_name(
                &self.ast,
                array
                    .elements
                    .iter()
                    .filter_map(|element| element.as_expression())
                    .filter_map(|element| self.resolve_stylex_arg(element)),
            ),
            // isActive && styles.active → LogicalExpression(And)
            Expression::LogicalExpression(logical)
                if logical.operator == oxc_ast::ast::LogicalOperator::And =>
            {
                // The right side should be the namespace reference
                if let Some(class_expr) = self.resolve_stylex_arg(&logical.right) {
                    // Build: condition ? " className" : ""
                    let condition = logical
                        .left
                        .clone_in_with_semantic_ids(self.ast.allocator());
                    Some(Expression::new_conditional_expression(
                        SPAN,
                        condition,
                        class_expr,
                        Expression::new_string_literal(SPAN, "", None, &self.ast),
                        &self.ast,
                    ))
                } else {
                    None
                }
            }
            // cond ? styles.a : styles.b → ConditionalExpression
            Expression::ConditionalExpression(cond) => {
                let consequent = self.resolve_stylex_arg(&cond.consequent);
                let alternate = self.resolve_stylex_arg(&cond.alternate);
                match (consequent, alternate) {
                    (Some(cons), Some(alt)) => {
                        let test = cond.test.clone_in_with_semantic_ids(self.ast.allocator());
                        Some(Expression::new_conditional_expression(
                            SPAN, test, cons, alt, &self.ast,
                        ))
                    }
                    (Some(cons), None) => {
                        let test = cond.test.clone_in_with_semantic_ids(self.ast.allocator());
                        Some(Expression::new_conditional_expression(
                            SPAN,
                            test,
                            cons,
                            Expression::new_string_literal(SPAN, "", None, &self.ast),
                            &self.ast,
                        ))
                    }
                    (None, Some(alt)) => {
                        let test = cond.test.clone_in_with_semantic_ids(self.ast.allocator());
                        Some(Expression::new_conditional_expression(
                            SPAN,
                            Expression::new_unary_expression(
                                SPAN,
                                oxc_ast::ast::UnaryOperator::LogicalNot,
                                test,
                                &self.ast,
                            ),
                            alt,
                            Expression::new_string_literal(SPAN, "", None, &self.ast),
                            &self.ast,
                        ))
                    }
                    (None, None) => None,
                }
            }
            // false, null, undefined, 0, "" → falsy, skip
            Expression::BooleanLiteral(b) if !b.value => None,
            Expression::NullLiteral(_) => None,
            Expression::Identifier(ident) if ident.name == "undefined" => None,
            Expression::NumericLiteral(n) if n.value == 0.0 => None,
            Expression::StringLiteral(s) if s.value.is_empty() => None,
            _ => Some(runtime_classes(&self.ast, expr)),
        }
    }
}

impl<'a> VisitMut<'a> for DevupVisitor<'a> {
    fn visit_expression_statement(&mut self, it: &mut ExpressionStatement<'a>) {
        walk_expression_statement(self, it);
        // `globalCss()` gives nothing, which a statement need not write out
        if matches!(&it.expression, Expression::UnaryExpression(unary)
            if unary.span == SPAN && unary.operator == UnaryOperator::Void)
        {
            it.expression = Expression::new_identifier(SPAN, "", &self.ast);
        }
    }

    fn visit_variable_declarators(
        &mut self,
        it: &mut oxc_allocator::Vec<'a, VariableDeclarator<'a>>,
    ) {
        for declarator in it.iter() {
            self.bindings.require(declarator, &self.package);
            self.bindings.alias(declarator);
        }
        walk_variable_declarators(self, it);
    }

    fn visit_program(&mut self, it: &mut Program<'a>) {
        if let Some(scoping) = self.scoping_of(it) {
            self.bindings.scope(Rc::clone(&scoping));
            self.bind_imported_stylex();
            self.style_values = crate::style_values::StyleValues::new(scoping);
            self.style_values
                .import_producer_atoms(std::mem::take(&mut self.imported_atoms));
            self.style_values
                .import(std::mem::take(&mut self.imported_css));
            if self.binds_style_results(it) || self.css_prop != CssProp::Off {
                self.selected_components = crate::style_values::selected(it, &self.style_values);
            }
        }
        walk_program(self, it);
        if self.bindings.is_compiled() {
            self.bindings.remove_aliases(it);
            self.report_compiled_reads(it);
        }
        if self.forwards_refs {
            let source = self.ast.allocator().alloc_str(&format!(
                "import {{ forwardRef as {FORWARD_REF} }} from 'react';"
            ));
            let program =
                oxc_parser::Parser::new(self.ast.allocator(), source, oxc_span::SourceType::mjs())
                    .parse()
                    .program;
            for statement in program.body.into_iter().rev() {
                it.body.insert(0, statement);
            }
        }
        if !self.styles.is_empty() {
            for css_file in self.css_files.iter().rev() {
                it.body.insert(
                    0,
                    Statement::ImportDeclaration(ImportDeclaration::boxed(
                        SPAN,
                        None,
                        StringLiteral::new(
                            SPAN,
                            Str::from_in(css_file, self.ast.allocator()),
                            None,
                            &self.ast,
                        ),
                        None,
                        None,
                        ImportOrExportKind::Value,
                        &self.ast,
                    )),
                );
            }
        }

        for i in (0..it.body.len()).rev() {
            if let Statement::ImportDeclaration(decl) = &it.body[i]
                && (decl.source.value == self.package
                    || decl.source.value == self.compat_package.as_str())
                && decl.specifiers.iter().all(|s| s.is_empty())
            {
                it.body.remove(i);
            }
        }
    }
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if !self.class_names_scope.is_empty() && self.compile_class_names_call(it) {
            return;
        }
        if self.bindings.may_style() {
            match it {
                Expression::CallExpression(call) => self.plain_styled(&mut call.callee),
                Expression::TaggedTemplateExpression(tag) => self.plain_styled(&mut tag.tag),
                _ => {}
            }
        }
        // Emotion's `styled(tag, options)` is called for a component, so its second
        // argument holds options rather than the rules of Devup UI's two-argument form
        let factory = match it {
            Expression::CallExpression(call) => Some(&mut call.callee),
            Expression::TaggedTemplateExpression(tag) => Some(&mut tag.tag),
            _ => None,
        };
        let bindings = &self.bindings;
        let (mut attrs, mut configs) = factory.map_or_else(
            || (Vec::new(), Vec::new()),
            |factory| {
                take_styled_modifiers(&self.ast, factory, |expression| {
                    bindings.is_styled(expression)
                })
            },
        );
        let factory = match it {
            Expression::CallExpression(call) => Some(&mut call.callee),
            Expression::TaggedTemplateExpression(tag) => Some(&mut tag.tag),
            _ => None,
        };
        if let Some(factory) = factory
            && let Expression::CallExpression(call) = unwrap_syntax_only_mut(factory)
            && call.arguments.len() == 2
            && call.arguments[1].as_expression().is_some_and(|options| {
                matches!(unwrap_syntax_only(options), Expression::ObjectExpression(_))
            })
            && matches!(&call.callee, callee @ Expression::Identifier(_) if self.bindings.is_styled(callee))
            && let Some(options) = call.arguments.pop()
        {
            configs.insert(0, options.into_expression());
        }
        let (forward, forward_error) = read_forward(&configs.iter().collect::<Vec<_>>());
        if let Some(error) = forward_error {
            self.errors.push(error);
        }
        for attr in &mut attrs {
            let reads = self.bindings.lowered_reads(attr);
            self.report_reads(reads);
            self.bindings.remember(attr);
            self.visit_expression(attr);
        }
        walk_expression(self, it);
        self.report_lowered_reads(it);

        // Handle styled function calls
        if self.bindings.may_style() {
            let (tag_or_call, argument_count) = match it {
                Expression::TaggedTemplateExpression(tag) => (Some(&tag.tag), 0),
                Expression::CallExpression(call) => (Some(&call.callee), call.arguments.len()),
                _ => (None, 0),
            };
            let bindings = &self.bindings;

            let is_styled = tag_or_call
                .map(unwrap_syntax_only)
                .is_some_and(|tag_or_call| match tag_or_call {
                    Expression::StaticMemberExpression(member) => {
                        bindings.is_styled(&member.object)
                    }
                    Expression::CallExpression(call) => bindings.is_styled(&call.callee),
                    // styled("div", { ... }) puts the tag in the arguments, so the callee is
                    // the bare identifier. One argument is the curried creator `styled("div")`,
                    // which only becomes a component once its result is called.
                    Expression::Identifier(_) => {
                        bindings.is_styled(tag_or_call) && argument_count == 2
                    }
                    _ => false,
                });

            if is_styled {
                self.style_values.read_in(&self.ast, it);
                if let Expression::CallExpression(call) = &*it {
                    self.unknown_arguments("styled", &call.arguments);
                    self.changed_arguments("styled", &call.arguments);
                }
                let inherited = extended(it)
                    .and_then(|base| self.style_values.symbol(base))
                    .and_then(|symbol| self.styled_definitions.get(&symbol))
                    .filter(|definition| definition.extendable());
                let start = it.span().start;
                let marker = self
                    .pending_marker
                    .take_if(|(at, _)| *at == start)
                    .map(|(_, marker)| marker);
                let StyledExtraction {
                    result,
                    expression,
                    errors,
                    definition,
                } = extract_style_from_styled(
                    &self.ast,
                    it,
                    Naming {
                        split_filename: self.split_filename.as_deref(),
                        marker: marker.as_deref(),
                    },
                    &|expression| self.bindings.kind(expression),
                    &attrs,
                    inherited,
                    forward,
                );
                self.errors.extend(errors);
                self.styles.extend(
                    result
                        .styles
                        .into_iter()
                        .flat_map(ExtractStyleProp::into_extract),
                );
                *it = if definition.is_some() {
                    self.forwards_refs = true;
                    forward_ref(&self.ast, expression)
                } else {
                    expression
                };
                self.pending_styled = definition.map(|definition| (start, definition));
            }
        }

        // `Component.withComponent(target)` on a styled component the file
        // defines renders its styles as `target`
        if let Expression::CallExpression(call) = it
            && let [
                Argument::StringLiteral(_)
                | Argument::Identifier(_)
                | Argument::StaticMemberExpression(_),
            ] = call.arguments.as_slice()
            && let Expression::StaticMemberExpression(member) = &call.callee
            && member.property.name == "withComponent"
            && let Some(definition) = self
                .style_values
                .symbol(&member.object)
                .and_then(|symbol| self.styled_definitions.get(&symbol))
            && let Some((component, definition)) = with_component(
                &self.ast,
                definition,
                call.arguments[0].to_expression(),
                self.split_filename.as_deref(),
            )
        {
            let start = call.span.start;
            self.forwards_refs = true;
            *it = forward_ref(&self.ast, component);
            self.pending_styled = Some((start, definition));
        }

        // Handle StyleX: stylex.create({...}) calls
        if let Expression::CallExpression(call) = it
            && self.is_stylex_call(&call.callee, &StylexFunction::Create)
            && let [Argument::ObjectExpression(arg)] = call.arguments.as_slice()
        {
            let visible = self.bindings.visible(arg);
            let (keyframe_names, var_refs) = self.stylex_names(&visible);
            let namespaces = extract_stylex_namespace_styles(
                arg,
                &keyframe_names,
                &var_refs,
                &mut self.errors,
                &|callee| self.bindings.stylex_function(callee),
            );

            let mut namespace_map: FxHashMap<String, StylexNamespaceValue> = FxHashMap::default();
            let mut properties = oxc_allocator::Vec::new_in(&self.ast);
            for (ns_name, mut styles, css_vars, include_refs) in namespaces {
                let class_name =
                    gen_class_names(&self.ast, &mut styles, None, self.split_filename.as_deref());
                self.styles
                    .extend(styles.into_iter().flat_map(ExtractStyleProp::into_extract));

                // Extract className string for props() resolution
                let mut class_name_str = class_name.as_ref().map_or(String::new(), |expr| {
                    if let Expression::StringLiteral(s) = expr {
                        s.value.to_string()
                    } else {
                        String::new()
                    }
                });

                // Resolve include() references — prepend included classNames
                for inc_ref in &include_refs {
                    let Some(ns_value) = visible
                        .get(&inc_ref.var_name)
                        .and_then(|symbol| self.stylex_namespaces.get(symbol))
                        .and_then(|ns| ns.get(&inc_ref.member_name))
                    else {
                        self.errors.push((
                            inc_ref.offset,
                            build_time_error(
                                "stylex.include",
                                &format!("{}.{}", inc_ref.var_name, inc_ref.member_name),
                                "it takes a namespace `stylex.create()` defines earlier in this file",
                            ),
                        ));
                        continue;
                    };
                    let included_class = match ns_value {
                        StylexNamespaceValue::Static(s) => s.clone(),
                        StylexNamespaceValue::Dynamic(info) => info.class_name.clone(),
                    };
                    if !included_class.is_empty() {
                        if class_name_str.is_empty() {
                            class_name_str = included_class;
                        } else {
                            class_name_str = format!("{included_class} {class_name_str}");
                        }
                    }
                }

                let ns_value = if let Some(vars) = css_vars {
                    StylexNamespaceValue::Dynamic(StylexDynamicInfo {
                        class_name: class_name_str.clone(),
                        css_vars: vars,
                    })
                } else {
                    StylexNamespaceValue::Static(class_name_str.clone())
                };
                namespace_map.insert(ns_name.clone(), ns_value);

                // If include refs changed the className, use the combined string
                let value = if !include_refs.is_empty() && !class_name_str.is_empty() {
                    Expression::new_string_literal(
                        SPAN,
                        Str::from_in(&class_name_str, self.ast.allocator()),
                        None,
                        &self.ast,
                    )
                } else {
                    class_name.unwrap_or_else(|| {
                        Expression::new_string_literal(SPAN, "", None, &self.ast)
                    })
                };

                properties.push(ObjectPropertyKind::new_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StringLiteral(StringLiteral::boxed(
                        SPAN,
                        Str::from_in(&ns_name, self.ast.allocator()),
                        None,
                        &self.ast,
                    )),
                    value,
                    false,
                    false,
                    false,
                    &self.ast,
                ));
            }

            self.stylex_pending_create = Some(namespace_map);
            *it = Expression::new_object_expression(SPAN, properties, &self.ast);
        }

        // Handle StyleX: stylex.defineConsts({...}) calls — build-time constants that
        // produce no CSS, so the call collapses to the literal object it described.
        if let Expression::CallExpression(call) = it
            && self.is_stylex_call(&call.callee, &StylexFunction::DefineConsts)
            && let [arg] = call.arguments.as_slice()
            && let Some(Expression::ObjectExpression(obj)) = arg.as_expression()
        {
            let mut contract = FxHashMap::default();
            let mut properties = oxc_allocator::Vec::new_in(&self.ast);
            for prop in &obj.properties {
                let Some((key, prop)) = self.stylex_key("stylex.defineConsts", prop) else {
                    continue;
                };
                let Some(value) = get_string_by_literal_expression(&prop.value) else {
                    self.errors.push((
                        prop.value.span().start,
                        runtime_value_error("stylex.defineConsts", &readable_code(&prop.value)),
                    ));
                    continue;
                };
                properties.push(self.string_property(&key, &value));
                contract.insert(key, value.into_owned());
            }
            self.stylex_pending_consts = Some(contract);
            *it = Expression::new_object_expression(SPAN, properties, &self.ast);
        }

        // Handle StyleX: stylex.defineVars({...}) / stylex.createThemeContract({...})
        // calls. A contract has no values to publish, so only `defineVars` emits the
        // `:root` block; both hand back the same `var()` references.
        if let Expression::CallExpression(call) = it
            && let Some(publishes_values) = [
                (StylexFunction::DefineVars, true),
                (StylexFunction::CreateThemeContract, false),
            ]
            .into_iter()
            .find_map(|(function, publishes)| {
                self.is_stylex_call(&call.callee, &function)
                    .then_some(publishes)
            })
            && let [arg] = call.arguments.as_slice()
            && let Some(Expression::ObjectExpression(obj)) = arg.as_expression()
        {
            let mut contract = FxHashMap::default();
            let mut variables = vec![];
            let mut properties = oxc_allocator::Vec::new_in(&self.ast);
            for prop in &obj.properties {
                let api = if publishes_values {
                    "stylex.defineVars"
                } else {
                    "stylex.createThemeContract"
                };
                let Some((key, prop)) = self.stylex_key(api, prop) else {
                    continue;
                };
                let values = if publishes_values {
                    let Some(values) = variable_values(&prop.value, &|callee| {
                        self.bindings.stylex_function(callee)
                    }) else {
                        self.errors.push((
                            prop.value.span().start,
                            runtime_value_error("stylex.defineVars", &readable_code(&prop.value)),
                        ));
                        continue;
                    };
                    values
                } else {
                    vec![]
                };
                let variable =
                    define_vars_variable(&self.filename, &key, self.split_filename.as_deref());
                variables.push((variable.clone(), values));
                properties.push(self.string_property(&key, &format!("var({variable})")));
                contract.insert(key, variable);
            }
            let css = css_variable_rules(":root", &variables);
            if !css.is_empty() {
                self.styles.insert(ExtractStyleValue::Css(ExtractCss {
                    css,
                    file: self.filename.clone(),
                }));
            }
            self.stylex_pending_vars = Some(contract);
            *it = Expression::new_object_expression(SPAN, properties, &self.ast);
        }

        // Handle StyleX: stylex.createTheme(contract, {...}) calls
        if let Expression::CallExpression(call) = it
            && self.is_stylex_call(&call.callee, &StylexFunction::CreateTheme)
            && let [contract_arg, values_arg] = call.arguments.as_slice()
            && let Some(Expression::Identifier(contract_ident)) = contract_arg.as_expression()
            && let Some(contract) = self
                .bindings
                .symbol(contract_ident)
                .and_then(|symbol| self.stylex_var_names.get(&symbol))
            && let Some(Expression::ObjectExpression(obj)) = values_arg.as_expression()
        {
            let mut variables = vec![];
            for prop in &obj.properties {
                let prop = match prop {
                    ObjectPropertyKind::ObjectProperty(prop) => prop,
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        self.errors.push(spread_error("stylex.createTheme", spread));
                        continue;
                    }
                };
                let Some(key) = get_string_by_property_key(&prop.key) else {
                    self.errors.push(key_error("stylex.createTheme", &prop.key));
                    continue;
                };
                let Some(variable) = contract.get(&key) else {
                    self.errors.push((
                        prop.key.span().start,
                        build_time_error(
                            "stylex.createTheme",
                            &key,
                            &format!("`{}` has no such variable", contract_ident.name),
                        ),
                    ));
                    continue;
                };
                match variable_values(&prop.value, &|callee| self.bindings.stylex_function(callee))
                {
                    Some(values) => variables.push((variable.clone(), values)),
                    None => self.errors.push((
                        prop.value.span().start,
                        runtime_value_error("stylex.createTheme", &readable_code(&prop.value)),
                    )),
                }
            }
            let class_name = create_theme_class(
                &self.filename,
                &contract_ident.name,
                self.split_filename.as_deref(),
            );
            let css = css_variable_rules(&format!(".{class_name}"), &variables);
            if !css.is_empty() {
                self.styles.insert(ExtractStyleValue::Css(ExtractCss {
                    css,
                    file: self.filename.clone(),
                }));
            }
            self.stylex_pending_theme_class = Some(class_name.clone());
            *it = Expression::new_string_literal(
                SPAN,
                Str::from_in(&class_name, self.ast.allocator()),
                None,
                &self.ast,
            );
        }

        // Handle StyleX: stylex.positionTry({...}) / stylex.viewTransitionClass({...}).
        // Both name a block of rules and hand the name back: `@position-try` takes a
        // dashed-ident, a view-transition class takes a plain class name.
        if let Expression::CallExpression(call) = it
            && let Some(is_position_try) = [
                (StylexFunction::PositionTry, true),
                (StylexFunction::ViewTransitionClass, false),
            ]
            .into_iter()
            .find_map(|(function, position_try)| {
                self.is_stylex_call(&call.callee, &function)
                    .then_some(position_try)
            })
            && let [Argument::ObjectExpression(arg)] = call.arguments.as_slice()
        {
            let generated = keyframes_to_keyframes_name(
                &format!("sxp-{}-{}", self.filename, u8::from(is_position_try)),
                self.split_filename.as_deref(),
            );
            let name = if is_position_try {
                format!("--{generated}")
            } else {
                generated
            };
            let api = if is_position_try {
                "stylex.positionTry"
            } else {
                "stylex.viewTransitionClass"
            };
            let declarations = extract_stylex_declarations(api, arg, &mut self.errors, &|callee| {
                self.bindings.stylex_function(callee)
            });
            if !declarations.is_empty() {
                let css = if is_position_try {
                    css_variable_block(&format!("@position-try {name}"), &declarations)
                } else {
                    css_variable_block(&format!(".{name}"), &declarations)
                };
                self.styles.insert(ExtractStyleValue::Css(ExtractCss {
                    css,
                    file: self.filename.clone(),
                }));
            }
            *it = Expression::new_string_literal(
                SPAN,
                Str::from_in(&name, self.ast.allocator()),
                None,
                &self.ast,
            );
        }

        // Handle StyleX: stylex.keyframes({...}) calls
        if let Expression::CallExpression(call) = it
            && self.is_stylex_call(&call.callee, &StylexFunction::Keyframes)
            && let [arg] = call.arguments.as_mut_slice()
            && let Some(arg @ Expression::ObjectExpression(_)) = arg.as_expression_mut()
        {
            let KeyframesExtractResult {
                keyframes,
                runtime_value,
            } = extract_keyframes_from_expression(&self.ast, arg);
            if let Some(value) = runtime_value {
                self.errors.push((
                    call.span.start,
                    runtime_value_error("stylex.keyframes", &value),
                ));
            }
            let name =
                style_property_into_string(keyframes.extract(self.split_filename.as_deref()));
            self.styles.insert(ExtractStyleValue::Keyframes(keyframes));
            self.stylex_pending_keyframe_name = Some(name.clone());
            *it = Expression::new_string_literal(
                SPAN,
                Str::from_in(&name, self.ast.allocator()),
                None,
                &self.ast,
            );
        }

        // Handle StyleX: stylex.props(...) / stylex.attrs(...) calls
        if let Expression::CallExpression(call) = it
            && let Some(class_attribute) = self.stylex_class_attribute(&call.callee)
        {
            let (class_exprs, style_props) = self.resolve_stylex_props_args(&call.arguments);

            // Build className expression using existing merge utility
            let class_name_expr = merge_expression_for_class_name(&self.ast, class_exprs)
                .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, &self.ast));

            // Build replacement: { className: <expr>, style?: { ... } }
            let mut props = oxc_allocator::Vec::new_in(&self.ast);
            props.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StaticIdentifier(IdentifierName::boxed(
                    SPAN,
                    class_attribute,
                    &self.ast,
                )),
                class_name_expr,
                false,
                false,
                false,
                &self.ast,
            ));

            // Add style property for dynamic CSS variables
            if !style_props.is_empty() {
                let style_obj = Expression::new_object_expression(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(style_props, &self.ast),
                    &self.ast,
                );
                props.push(ObjectPropertyKind::new_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StaticIdentifier(IdentifierName::boxed(SPAN, "style", &self.ast)),
                    style_obj,
                    false,
                    false,
                    false,
                    &self.ast,
                ));
            }

            *it = Expression::new_object_expression(SPAN, props, &self.ast);
        }

        // Reached only when none of the blocks above replaced the call, so a surviving
        // compile-time call is one whose arguments could not be read statically.
        if let Expression::CallExpression(call) = it
            && let Some(function) = self.bindings.stylex_function(&call.callee)
            && let Some(requirement) = function.requirement()
        {
            let arguments: Vec<String> = call.arguments.iter().map(readable_argument).collect();
            self.errors.push((
                call.span.start,
                build_time_error(
                    &format!("stylex.{}", function.export_name()),
                    &arguments.join(", "),
                    requirement,
                ),
            ));
        }

        if let Expression::CallExpression(call) = it
            && self
                .util_type(&call.callee)
                .is_some_and(|util| matches!(util.as_ref(), UtilType::Css))
            && let Some(composed) = self.compose_known_styles(call)
        {
            *it = composed;
        } else if let Expression::CallExpression(call) = it {
            if let Some(util_type) = self.util_type(&call.callee) {
                for argument in &mut call.arguments {
                    let expression = match argument {
                        Argument::SpreadElement(spread) => &mut spread.argument,
                        argument => argument.to_expression_mut(),
                    };
                    self.style_values.read_in(&self.ast, expression);
                }
                let offset = call.span.start;
                let is_css = matches!(util_type.as_ref(), UtilType::Css);
                if is_css {
                    self.unknown_arguments("css", &call.arguments);
                    self.changed_arguments("css", &call.arguments);
                }
                let composed_classes = if is_css
                    && let Some(StyleArguments { classes, rules }) =
                        style_arguments(&self.ast, &call.arguments)
                {
                    call.arguments =
                        oxc_allocator::Vec::from_array_in([Argument::from(rules)], &self.ast);
                    classes
                } else {
                    if is_css && !reads_directly(&call.arguments) {
                        self.errors
                            .push((offset, uncomposable_error(&call.arguments)));
                        call.arguments.clear();
                    }
                    vec![]
                };
                if call.arguments.len() == 1 {
                    let r = util_type.as_ref();
                    *it = if matches!(r, UtilType::Css) {
                        let ExtractResult {
                            mut styles,
                            style_order,
                            ..
                        } = extract_style_from_expression(
                            &self.ast,
                            None,
                            if let Argument::SpreadElement(spread) = &mut call.arguments[0] {
                                &mut spread.argument
                            } else {
                                call.arguments[0].to_expression_mut()
                            },
                            0,
                            &None,
                            LiteralHandling::ExpandResponsiveThemeToken,
                        );
                        if let Some(value) = runtime_value(&styles) {
                            self.errors
                                .push((offset, runtime_value_error("css", &value)));
                        }

                        if styles.is_empty() {
                            Expression::new_string_literal(SPAN, "", None, &self.ast)
                        } else {
                            let known = composed_classes.is_empty().then(|| {
                                let mut known = crate::composition::Composition::default();
                                let mut props: Vec<ExtractStyleProp<'a>> = styles
                                    .iter()
                                    .map(|prop| prop.clone_in(self.ast.allocator()))
                                    .collect();
                                if let Some(order) = style_order {
                                    for prop in &mut props {
                                        set_prop_order(prop, order);
                                    }
                                }
                                known.apply(&self.ast, props);
                                known.unconditional()
                            });
                            // css can not reachable
                            let class_name = gen_class_names(
                                &self.ast,
                                &mut styles,
                                style_order,
                                self.split_filename.as_deref(),
                            );

                            // already set style order
                            self.styles.extend(
                                styles.into_iter().flat_map(ExtractStyleProp::into_extract),
                            );
                            if let Some(Some(known)) = known
                                && matches!(class_name, Some(Expression::StringLiteral(_)))
                            {
                                self.css_styles = Some((offset, known));
                            }
                            if let Some(cls) = class_name {
                                cls
                            } else {
                                Expression::new_string_literal(SPAN, "", None, &self.ast)
                            }
                        }
                    } else if matches!(r, UtilType::Keyframes) {
                        let KeyframesExtractResult {
                            keyframes,
                            runtime_value,
                        } = extract_keyframes_from_expression(
                            &self.ast,
                            if let Argument::SpreadElement(spread) = &mut call.arguments[0] {
                                &mut spread.argument
                            } else {
                                call.arguments[0].to_expression_mut()
                            },
                        );
                        if let Some(value) = runtime_value {
                            self.errors
                                .push((offset, runtime_value_error("keyframes", &value)));
                        }

                        let name = style_property_into_string(
                            keyframes.extract(self.split_filename.as_deref()),
                        );
                        self.styles.insert(ExtractStyleValue::Keyframes(keyframes));
                        Expression::new_string_literal(
                            SPAN,
                            Str::from_in(&name, self.ast.allocator()),
                            None,
                            &self.ast,
                        )
                    } else {
                        // global
                        let GlobalExtractResult {
                            styles,
                            style_order,
                        } = extract_global_style_from_expression(
                            &self.ast,
                            if let Argument::SpreadElement(spread) = &mut call.arguments[0] {
                                &mut spread.argument
                            } else {
                                call.arguments[0].to_expression_mut()
                            },
                            &self.filename,
                        );
                        if let Some(value) = fixed_value(&styles) {
                            self.errors
                                .push((offset, runtime_value_error("globalCss", &value)));
                        }
                        // already set style order
                        let style_order = style_order.unwrap_or(0);
                        self.styles.extend(
                            styles
                                .into_iter()
                                .flat_map(ExtractStyleProp::into_extract)
                                .map(|mut style| {
                                    style.set_style_order(style_order);
                                    style
                                }),
                        );
                        self.global_css_result(r.is_component())
                    }
                } else if call.arguments.len() == 2
                    && util_type.is_global()
                    && let Some(selector) = call.arguments[0]
                        .as_expression()
                        .and_then(get_string_by_literal_expression)
                    && let Some(rules) = call.arguments[1].as_expression()
                {
                    // vanilla-extract spells global rules `globalStyle(selector, rules)`;
                    // fold the selector back into the object `globalCss` expects.
                    let mut folded = Expression::new_object_expression(
                        SPAN,
                        oxc_allocator::Vec::from_array_in(
                            [ObjectPropertyKind::new_object_property(
                                SPAN,
                                PropertyKind::Init,
                                PropertyKey::StringLiteral(StringLiteral::boxed(
                                    SPAN,
                                    Str::from_in(selector.as_ref(), self.ast.allocator()),
                                    None,
                                    &self.ast,
                                )),
                                rules.clone_in_with_semantic_ids(self.ast.allocator()),
                                false,
                                false,
                                false,
                                &self.ast,
                            )],
                            &self.ast,
                        ),
                        &self.ast,
                    );
                    let GlobalExtractResult {
                        styles,
                        style_order,
                    } = extract_global_style_from_expression(
                        &self.ast,
                        &mut folded,
                        &self.filename,
                    );
                    if let Some(value) = fixed_value(&styles) {
                        self.errors
                            .push((offset, runtime_value_error("globalCss", &value)));
                    }
                    let style_order = style_order.unwrap_or(0);
                    self.styles.extend(
                        styles
                            .into_iter()
                            .flat_map(ExtractStyleProp::into_extract)
                            .map(|mut style| {
                                style.set_style_order(style_order);
                                style
                            }),
                    );
                    *it = self.global_css_result(util_type.is_component());
                } else {
                    *it = match util_type.as_ref() {
                        UtilType::Css | UtilType::Keyframes => {
                            Expression::new_string_literal(SPAN, "", None, &self.ast)
                        }
                        global => self.global_css_result(global.is_component()),
                    };
                }
                if !composed_classes.is_empty() {
                    let own = std::mem::replace(
                        it,
                        Expression::new_string_literal(SPAN, "", None, &self.ast),
                    );
                    *it = merge_expression_for_class_name(
                        &self.ast,
                        composed_classes.into_iter().chain([own]),
                    )
                    .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, &self.ast));
                }
            }
        } else if let Expression::TaggedTemplateExpression(tag) = it
            && let Some(css_type) = self.util_type(&tag.tag)
        {
            self.style_values.read_in_text(&self.ast, &mut tag.quasi);
            let r = css_type.as_ref();
            let api = match r {
                UtilType::Css => "css",
                UtilType::Keyframes => "keyframes",
                UtilType::GlobalCss | UtilType::GlobalCssComponent => "globalCss",
            };
            let mut build_css_str = || {
                template_css_text(&tag.quasi, api).unwrap_or_else(|error| {
                    self.errors.push(error);
                    String::new()
                })
            };
            *it = if matches!(r, UtilType::Css) {
                let TemplateStyles {
                    styles,
                    statements,
                    unplaced,
                } = css_to_style_template(&tag.quasi, 0, &None);
                let mut style_props = styles
                    .into_iter()
                    .map(|ex| ExtractStyleProp::Static(ex.into()))
                    .collect::<Vec<_>>();
                if let Some(value) = runtime_value(&style_props) {
                    self.errors
                        .push((tag.span.start, runtime_value_error(api, &value)));
                }
                for index in unplaced {
                    let expression = &tag.quasi.expressions[index];
                    self.errors
                        .push((expression.span().start, unplaced_error(expression)));
                }
                let class_name = gen_class_names(
                    &self.ast,
                    &mut style_props,
                    None,
                    self.split_filename.as_deref(),
                );

                for ex in style_props {
                    // every entry was built as `Static` above
                    if let ExtractStyleProp::Static(v) = ex {
                        self.styles.insert(v);
                    }
                }
                let mixins = statements.iter().map(|index| {
                    tag.quasi.expressions[*index].clone_in_with_semantic_ids(self.ast.allocator())
                });
                merge_expression_for_class_name(&self.ast, mixins.chain(class_name))
                    .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, &self.ast))
                // already set style order
            } else if matches!(r, UtilType::Keyframes) {
                let keyframes = ExtractKeyframes {
                    keyframes: keyframes_to_keyframes_style(&build_css_str()),
                };
                let name =
                    style_property_into_string(keyframes.extract(self.split_filename.as_deref()));

                self.styles.insert(ExtractStyleValue::Keyframes(keyframes));
                Expression::new_string_literal(
                    SPAN,
                    Str::from_in(&name, self.ast.allocator()),
                    None,
                    &self.ast,
                )
            } else {
                let optimized_css = optimize_css_block(&build_css_str());
                if !optimized_css.is_empty() {
                    let css = ExtractStyleValue::Css(ExtractCss {
                        css: optimized_css,
                        file: self.filename.clone(),
                    });
                    self.styles.insert(css);
                }
                self.global_css_result(r.is_component())
            }
        }

        if let Expression::JSXElement(_) = it
            && let Some(replacement) = self.pending_replacement.take()
        {
            *it = replacement;
        }
    }

    fn visit_jsx_child(&mut self, it: &mut JSXChild<'a>) {
        walk_jsx_child(self, it);
        if let JSXChild::Element(_) = it
            && let Some(replacement) = self.pending_replacement.take()
        {
            *it = JSXChild::ExpressionContainer(JSXExpressionContainer::boxed(
                SPAN,
                replacement.into(),
                &self.ast,
            ));
        }
    }

    fn visit_jsx_attribute_value(&mut self, it: &mut JSXAttributeValue<'a>) {
        walk_jsx_attribute_value(self, it);
        if let JSXAttributeValue::Element(_) = it
            && let Some(replacement) = self.pending_replacement.take()
        {
            *it = JSXAttributeValue::new_expression_container(SPAN, replacement.into(), &self.ast);
        }
    }
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        if let Some(j) = self.bindings.jsx_function(&it.callee)
            && matches!(j.as_str(), "jsx" | "jsxs" | "jsxDEV")
            && let Some(expr) = it.arguments.first().and_then(|arg| arg.as_expression())
        {
            let element_kind = self.bindings.kind(expr);
            if let Some(props) = it.arguments.get(1).and_then(Argument::as_expression) {
                self.bindings.remember(props);
            }
            if element_kind.is_some()
                && let Some(props) = it.arguments.get(1).and_then(Argument::as_expression)
            {
                let reads = self.bindings.lowered_reads(props);
                self.report_reads(reads);
            }
            let css = self.take_jsx_css_prop(it, element_kind.is_some());
            if element_kind.is_none()
                && let Some(css) = css
            {
                self.lower_jsx_css_prop(it, css);
            } else if let Some(kind) = element_kind
                && it
                    .arguments
                    .get(1)
                    .is_some_and(|arg| arg.as_expression().is_some())
            {
                // Pre-scan: detect conditional styleOrder before extract_style_from_expression
                // consumes the property (which only handles static values)
                let parsed_style_order =
                    if let Expression::ObjectExpression(obj) = it.arguments[1].to_expression() {
                        obj.properties.iter().find_map(|prop| {
                            if let ObjectPropertyKind::ObjectProperty(p) = prop
                                && let Some(name) = get_str_by_property_key(&p.key)
                                && name == "styleOrder"
                            {
                                Some(expression_to_style_order(&p.value, self.ast.allocator()))
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                    .unwrap_or(ParsedStyleOrder::None);

                let mut tag = Expression::new_string_literal(
                    SPAN,
                    Str::from_in(kind.to_tag(), self.ast.allocator()),
                    None,
                    &self.ast,
                );
                let mut props_styles = vec![];
                let ExtractResult {
                    styles,
                    tag: _tag,
                    style_order,
                    style_vars,
                    props,
                } = extract_style_from_expression(
                    &self.ast,
                    None,
                    it.arguments[1].to_expression_mut(),
                    0,
                    &None,
                    LiteralHandling::ExpandResponsiveThemeToken,
                );
                props_styles.extend(styles);

                if let Some(t) = _tag {
                    tag = t;
                }

                props_styles.extend(
                    kind.extract()
                        .into_iter()
                        .rev()
                        .map(ExtractStyleProp::Static),
                );
                if let Some(JsxCss {
                    css,
                    element,
                    class_name,
                    ..
                }) = css
                {
                    props_styles.reverse();
                    props_styles =
                        self.compose_css_prop(&element, css, props_styles, class_name.as_ref());
                    props_styles.reverse();
                }

                // Use pre-scanned ParsedStyleOrder, falling back to extract_style_from_expression's
                // static result for backward compat.
                // Note: pre-scan and extract_style_from_expression both use get_number_by_literal_expression
                // on the same value, so style_order is always None when parsed_style_order is None.
                let parsed_style_order = match parsed_style_order {
                    ParsedStyleOrder::None => {
                        style_order.map_or(ParsedStyleOrder::None, ParsedStyleOrder::Static)
                    }
                    other => other,
                };

                let read_once = self.read_spreads_once(it.arguments[1].to_expression_mut());

                if let ParsedStyleOrder::Conditional {
                    condition,
                    consequent,
                    alternate,
                } = &parsed_style_order
                {
                    // Clone styles for alternate branch before consequent processing mutates them
                    let mut alt_props_styles: Vec<ExtractStyleProp<'a>> = props_styles
                        .iter()
                        .map(|s| s.clone_in(self.ast.allocator()))
                        .collect();

                    if let Expression::ObjectExpression(obj) = it.arguments[1].to_expression_mut() {
                        let tailwind_styles = modify_prop_object(
                            &self.ast,
                            &mut obj.properties,
                            &mut props_styles,
                            *consequent,
                            style_vars,
                            props,
                            self.split_filename.as_deref(),
                            Some((
                                condition.clone_in(self.ast.allocator()),
                                &mut alt_props_styles,
                                *alternate,
                            )),
                        );
                        self.styles.extend(tailwind_styles);
                    }

                    // Collect styles from both branches for CSS output
                    props_styles.into_iter().rev().for_each(|style| {
                        self.styles
                            .extend(style.into_extract().into_iter().map(|mut s| {
                                if let Some(order) = consequent {
                                    s.set_style_order(*order);
                                }
                                s
                            }));
                    });
                    alt_props_styles.into_iter().rev().for_each(|style| {
                        self.styles
                            .extend(style.into_extract().into_iter().map(|mut s| {
                                if let Some(order) = alternate {
                                    s.set_style_order(*order);
                                }
                                s
                            }));
                    });
                } else {
                    let style_order = parsed_style_order.as_static();
                    props_styles.iter().rev().for_each(|style| {
                        self.styles.extend(style.extract().into_iter().map(|mut s| {
                            style_order.into_iter().for_each(|order| {
                                s.set_style_order(order);
                            });
                            s
                        }));
                    });

                    if let Expression::ObjectExpression(obj) = it.arguments[1].to_expression_mut() {
                        let tailwind_styles = modify_prop_object(
                            &self.ast,
                            &mut obj.properties,
                            &mut props_styles,
                            style_order,
                            style_vars,
                            props,
                            self.split_filename.as_deref(),
                            None,
                        );
                        self.styles.extend(tailwind_styles);
                    }
                }

                it.arguments[0] = Argument::from(tag);
                if !read_once.is_empty() {
                    let call = Expression::CallExpression(oxc_allocator::Box::new_in(
                        it.clone_in_with_semantic_ids(self.ast.allocator()),
                        &self.ast,
                    ));
                    if let Expression::CallExpression(reading_once) =
                        call_with_values(&self.ast, read_once, call)
                    {
                        *it = reading_once.unbox();
                    }
                }
            }
        }
        walk_call_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        let style_result = match &it.init {
            Some(Expression::CallExpression(call)) => self.util_type(&call.callee),
            Some(Expression::TaggedTemplateExpression(tag)) => self.util_type(&tag.tag),
            _ => None,
        }
        .filter(|util| matches!(util.as_ref(), UtilType::Css | UtilType::Keyframes))
        .and_then(|util| Some((util, self.style_values.constant(&it.id)?)));
        let start = it.init.as_ref().map(|init| init.span().start);
        let styled_binding = self.style_values.constant(&it.id);
        if self.css_prop != CssProp::Off
            && matches!(
                it.init.as_ref().map(unwrap_syntax_only),
                Some(
                    Expression::ObjectExpression(_)
                        | Expression::ArrayExpression(_)
                        | Expression::ArrowFunctionExpression(_)
                        | Expression::FunctionExpression(_)
                        | Expression::StringLiteral(_)
                        | Expression::TemplateLiteral(_)
                )
            )
            && let Some(symbol) = it
                .id
                .get_binding_identifier()
                .and_then(|id| id.symbol_id.get())
                .filter(|symbol| {
                    self.style_values.constant(&it.id).is_none()
                        || self.style_values.is_local(*symbol)
                })
        {
            self.local_styles.insert(symbol);
        }

        let marked = styled_binding
            .filter(|symbol| self.selected_components.contains(symbol))
            .zip(start)
            .zip(it.id.get_binding_identifier().map(|id| id.name.to_string()));
        if let Some(((symbol, start), name)) = &marked {
            let (symbol, start) = (*symbol, *start);
            let marker = css::component_marker(name, &self.filename);
            self.style_values.insert(
                symbol,
                crate::style_values::StyleValue::Component(format!(".{marker}")),
            );
            self.pending_marker = Some((start, marker));
        }

        walk_variable_declarator(self, it);

        // A binding selected but not to a styled component stays unread
        if let Some(((symbol, _), _)) = marked
            && self.pending_marker.take().is_some()
        {
            self.style_values.remove(symbol);
        }

        if let Some((at, definition)) = self.pending_styled.take()
            && Some(at) == start
            && let Some(symbol) = styled_binding
        {
            self.styled_definitions.insert(symbol, definition);
        }

        if let Some((util, symbol)) = style_result
            && let Some(Expression::StringLiteral(value)) = &it.init
        {
            let value = value.value.to_string();
            let styles = self
                .css_styles
                .take()
                .and_then(|(at, styles)| (Some(at) == start).then_some(styles));
            self.style_values.insert(
                symbol,
                if matches!(util.as_ref(), UtilType::Css) {
                    crate::style_values::StyleValue::Class(value, styles)
                } else {
                    crate::style_values::StyleValue::Keyframes(value)
                },
            );
        }

        // Phase 4c: Check for destructuring of stylex.create()
        if self.stylex_pending_create.is_some() && it.id.get_binding_identifier().is_none() {
            self.errors.push((
                it.span.start,
                "`stylex.create()` cannot be destructured at build time: assign it to one variable, as `const styles = stylex.create({ ... })`".to_string(),
            ));
            self.stylex_pending_create.take();
        }

        let bound = it
            .id
            .get_binding_identifier()
            .and_then(|ident| ident.symbol_id.get());

        // After walking, capture stylex.create() variable binding
        if let Some(pending) = self.stylex_pending_create.take()
            && let Some(symbol) = bound
        {
            self.stylex_namespaces.insert(symbol, pending);
        }

        // Capture stylex.keyframes() variable binding
        if let Some(name) = self.stylex_pending_keyframe_name.take()
            && let Some(symbol) = bound
        {
            self.stylex_keyframe_names.insert(symbol, name);
        }

        // Capture stylex.defineVars() variable binding
        if let Some(contract) = self.stylex_pending_vars.take()
            && let Some(symbol) = bound
        {
            self.stylex_var_refs.entry(symbol).or_default().extend(
                contract
                    .iter()
                    .map(|(key, variable)| (key.clone(), format!("var({variable})"))),
            );
            self.stylex_var_names.insert(symbol, contract);
        }

        // Capture stylex.createTheme() variable binding
        if let Some(class_name) = self.stylex_pending_theme_class.take()
            && let Some(symbol) = bound
        {
            self.stylex_theme_classes.insert(symbol, class_name);
        }

        // Capture stylex.defineConsts() variable binding
        if let Some(constants) = self.stylex_pending_consts.take()
            && let Some(symbol) = bound
        {
            self.stylex_var_refs
                .entry(symbol)
                .or_default()
                .extend(constants);
        }
    }
    fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
        if it.source.value != self.package
            && matches!(
                it.source.value.as_str(),
                "react/jsx-runtime" | "react/jsx-dev-runtime"
            )
            && let Some(specifiers) = &it.specifiers
        {
            for specifier in specifiers {
                if let ImportSpecifier(import) = specifier {
                    self.bindings
                        .jsx_import(&import.local, import.imported.to_string());
                }
            }
        } else if (it.source.value == self.package
            || it.source.value == self.compat_package.as_str())
            && let Some(specifiers) = &mut it.specifiers
        {
            let compat = it.source.value == self.compat_package.as_str();
            for i in (0..specifiers.len()).rev() {
                match &specifiers[i] {
                    ImportSpecifier(import) => {
                        let imported_str = import.imported.to_string();
                        let local = import.local.symbol_id.get();
                        // Emotion's `jsx`, which builds elements as React does once
                        // their `css` props are compiled
                        if compat && imported_str == "jsx" {
                            self.bindings.jsx_import(&import.local, imported_str);
                            continue;
                        }
                        if compat && imported_str == "ClassNames" {
                            self.bindings.class_names_component(local);
                            self.bindings.compile(local);
                            specifiers.remove(i);
                            continue;
                        }
                        if !self.bindings.export(local, &imported_str) {
                            // `Global` stays, rendering nothing, so its binding does too
                            if imported_str == "Global" {
                                self.bindings.global_component(local);
                            }
                            continue;
                        }
                        self.bindings.compile(local);
                        specifiers.remove(i);
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                        self.bindings.namespace(specifier.local.symbol_id.get());
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                        self.bindings.namespace(specifier.local.symbol_id.get());
                    }
                }
            }
        } else if it.source.value == "@stylexjs/stylex" {
            if let Some(specifiers) = &it.specifiers {
                for specifier in specifiers {
                    match specifier {
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(default_spec) => {
                            self.bindings
                                .stylex_namespace(default_spec.local.symbol_id.get());
                        }
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns_spec) => {
                            self.bindings
                                .stylex_namespace(ns_spec.local.symbol_id.get());
                        }
                        ImportSpecifier(named_spec) => {
                            let imported = named_spec.imported.to_string();
                            if let Some(func) = StylexFunction::from_export_name(&imported) {
                                self.bindings
                                    .stylex_import(named_spec.local.symbol_id.get(), func);
                            }
                        }
                    }
                }
            }
        } else {
            walk_import_declaration(self, it);
        }
    }
    #[allow(clippy::set_contains_or_insert)]
    fn visit_jsx_element(&mut self, elem: &mut JSXElement<'a>) {
        if let Some(rendered) = self.class_names(elem) {
            self.pending_replacement = Some(rendered);
            return;
        }
        let css = self.take_css_prop(elem);
        walk_jsx_element(self, elem);

        // A styled component the file defines drops the props it neither reads
        // nor passes on, as styled-components and Emotion do at runtime
        if let JSXElementName::IdentifierReference(name) = &elem.opening_element.name
            && let Some(definition) = self
                .style_values
                .reference_symbol(name)
                .and_then(|symbol| self.styled_definitions.get(&symbol))
        {
            elem.opening_element
                .attributes
                .retain(|attribute| match attribute {
                    Attribute(attribute) => match &attribute.name {
                        Identifier(name) => definition.takes(&name.name),
                        oxc_ast::ast::JSXAttributeName::NamespacedName(_) => true,
                    },
                    JSXAttributeItem::SpreadAttribute(_) => true,
                });
        }

        // `<Global styles={...} />` is Emotion's spelling of a global stylesheet.
        // Lift the rules out and strip every attribute, leaving a component that
        // renders nothing — the same shape `createGlobalStyle` collapses to.
        if self
            .bindings
            .is_global_component(&elem.opening_element.name)
        {
            let name = elem.opening_element.name.to_string();
            for i in (0..elem.opening_element.attributes.len()).rev() {
                let Attribute(attr) = &mut elem.opening_element.attributes[i] else {
                    continue;
                };
                if let Identifier(attr_name) = &attr.name
                    && attr_name.name == "styles"
                    && let Some(JSXAttributeValue::ExpressionContainer(container)) = &mut attr.value
                    && let Some(expression) = container.expression.as_expression_mut()
                {
                    let offset = expression.span().start;
                    self.style_values.read_in(&self.ast, expression);
                    let GlobalExtractResult {
                        styles,
                        style_order,
                    } = extract_global_style_from_expression(&self.ast, expression, &self.filename);
                    if let Some(value) = fixed_value(&styles) {
                        self.errors
                            .push((offset, element_error(&name, &value, RUNTIME_VALUE)));
                    }
                    let style_order = style_order.unwrap_or(0);
                    self.styles.extend(
                        styles
                            .into_iter()
                            .flat_map(ExtractStyleProp::into_extract)
                            .map(|mut style| {
                                style.set_style_order(style_order);
                                style
                            }),
                    );
                }
                elem.opening_element.attributes.remove(i);
            }
            return;
        }

        // after run to convert css literal
        let kind = self.bindings.element(&elem.opening_element.name);
        if let Some(kind) = kind {
            let reads = self
                .bindings
                .lowered_attribute_reads(&elem.opening_element.attributes);
            self.report_reads(reads);
            let element_name = elem.opening_element.name.to_string();
            // A spread whose value may change when read again is read once, as
            // its `className` and `style` are read beside it
            let reads_once = reads_spreads_once(elem);
            let attrs = &mut elem.opening_element.attributes;
            let default_tag = kind.to_tag();
            let mut tag_name = Expression::new_string_literal(
                SPAN,
                Str::from_in(default_tag, self.ast.allocator()),
                None,
                &self.ast,
            );
            let mut props_styles = vec![];

            // extract ExtractStyleProp and remain style and class name, just extract
            let mut duplicate_set = FxHashSet::default();
            let mut parsed_style_order = ParsedStyleOrder::None;
            let mut style_vars = None;
            let mut props = None;
            for i in (0..attrs.len()).rev() {
                let mut attr = attrs.remove(i);
                if let Attribute(attr) = &mut attr
                    && let Identifier(name) = &attr.name
                    && !is_special_property(&name.name)
                {
                    let property_name = name.name.as_str();
                    for disassembled in disassemble_property(property_name) {
                        // Probe with `contains`, run the body borrowing `&disassembled`
                        // (it has no early exits), then MOVE the value into the set at
                        // the end — instead of `insert(disassembled.clone())`, which
                        // heap-copied the `Cow::Owned` String for every non-phf-mapped
                        // camelCase prop. The lint's suggested single `insert` would
                        // force that clone back, since the body still needs the name.
                        // The `allow` sits on `visit_jsx_element` rather than here so
                        // coverage does not report the attribute line itself as unrun.
                        if !duplicate_set.contains(&disassembled) {
                            if property_name == "styleOrder" {
                                if let Some(value) = attr.value.as_ref() {
                                    parsed_style_order =
                                        jsx_expression_to_style_order(value, self.ast.allocator());
                                }
                            } else if property_name == "props" {
                                if let Some(value) = attr.value.as_ref()
                                    && let JSXAttributeValue::ExpressionContainer(expr) = value
                                    && let Some(expression) = expr.expression.as_expression()
                                {
                                    props = Some(
                                        expression.clone_in_with_semantic_ids(self.ast.allocator()),
                                    );
                                }
                            } else if property_name == "styleVars" {
                                if let Some(value) = attr.value.as_ref()
                                    && let JSXAttributeValue::ExpressionContainer(expr) = value
                                    && let Some(expression) = expr.expression.as_expression()
                                {
                                    style_vars = Some(
                                        expression.clone_in_with_semantic_ids(self.ast.allocator()),
                                    );
                                }
                            } else if let Some(at) = &mut attr.value {
                                if let JSXAttributeValue::ExpressionContainer(container) = at
                                    && let Some(expression) =
                                        container.expression.as_expression_mut()
                                {
                                    self.style_values.read_in(&self.ast, expression);
                                }
                                let ExtractResult { styles, tag, .. } =
                                    extract_style_from_jsx(&self.ast, &disassembled, at);
                                props_styles.extend(styles.into_iter().rev());
                                tag_name = tag.unwrap_or(tag_name);
                            }
                            duplicate_set.insert(disassembled);
                        }
                    }
                } else if let JSXAttributeItem::SpreadAttribute(spread) = &mut attr {
                    self.style_values.read_in(&self.ast, &mut spread.argument);
                    // A later attribute wins over what the spread gives, and the
                    // spread over earlier ones, as props are assigned in order
                    if let Expression::ObjectExpression(object) =
                        unwrap_syntax_only_mut(&mut spread.argument)
                    {
                        flatten_spreads(&self.ast, object);
                        let mut given = Vec::new();
                        object.properties.retain(|property| {
                            let ObjectPropertyKind::ObjectProperty(property) = property else {
                                return true;
                            };
                            let Some(key) = get_str_by_property_key(&property.key) else {
                                return true;
                            };
                            if is_special_property(&key) {
                                return true;
                            }
                            let names: Vec<_> =
                                disassemble_property(&key).map(Cow::into_owned).collect();
                            let overridden = names
                                .iter()
                                .all(|name| duplicate_set.contains(name.as_str()));
                            given.extend(names);
                            !overridden
                        });
                        duplicate_set.extend(given.into_iter().map(Cow::Owned));
                    }
                    // Extract styles from spread attributes (e.g., {...{"@media": {...}}})
                    let ExtractResult { styles, .. } = extract_style_from_expression(
                        &self.ast,
                        None,
                        &mut spread.argument,
                        0,
                        &None,
                        LiteralHandling::ExpandResponsiveThemeToken,
                    );
                    // A spread the build cannot read passes its props at runtime,
                    // and an object literal keeps the props that are not styles
                    let runtime = styles
                        .iter()
                        .all(|style| matches!(style, ExtractStyleProp::Unreadable { .. }));
                    self.composes_unknown |= runtime
                        && (matches!(
                            unwrap_syntax_only(&spread.argument),
                            Expression::CallExpression(_)
                        ) || reads_unknown(
                            &spread.argument,
                            &self.unknown_bindings,
                            &|identifier| self.bindings.reads_module(identifier),
                        ));
                    if runtime
                        && reads_binding(&spread.argument, &self.changed_bindings, &|identifier| {
                            self.bindings.reads_module(identifier)
                        })
                    {
                        self.errors.push((
                            spread.span.start,
                            element_error(
                                &kind.to_string(),
                                &readable_code(&spread.argument),
                                STYLE_OBJECT,
                            ),
                        ));
                    }
                    if runtime
                        || matches!(unwrap_syntax_only(&spread.argument),
                            Expression::ObjectExpression(object) if !object.properties.is_empty())
                    {
                        attrs.insert(i, attr);
                    }
                    if !runtime {
                        props_styles.extend(styles.into_iter().rev());
                    }
                } else {
                    attrs.insert(i, attr);
                }
            }

            kind.extract()
                .into_iter()
                .rev()
                .for_each(|ex| props_styles.push(ExtractStyleProp::Static(ex)));

            let mut unreadable = Vec::new();
            unreadable_styles(&props_styles, false, &mut unreadable);

            // The `css` prop applies over the component's own styles, which
            // read in reverse
            if let Some(css) = css {
                let class_name = written_class_name(&self.ast, attrs);
                props_styles.reverse();
                props_styles =
                    self.compose_css_prop(&element_name, css, props_styles, class_name.as_ref());
                props_styles.reverse();
            }

            let mut read_once = Vec::new();
            if reads_once {
                // What comes before the last value moved out moves out too, so
                // everything is evaluated in the order it is written
                let last_attribute = attrs.iter().rposition(|attribute| match attribute {
                    JSXAttributeItem::SpreadAttribute(spread) => !is_pure(&spread.argument),
                    JSXAttributeItem::Attribute(attribute) => attribute_value(attribute)
                        .is_some_and(|value| {
                            let mut suspends = Suspends::default();
                            oxc_ast_visit::Visit::visit_expression(&mut suspends, value);
                            suspends.found
                        }),
                });
                let last_child = elem.children.iter().rposition(|child| {
                    let mut suspends = Suspends::default();
                    oxc_ast_visit::Visit::visit_jsx_child(&mut suspends, child);
                    suspends.found
                });
                let attributes_moved = if last_child.is_some() {
                    attrs.len()
                } else {
                    last_attribute.map_or(0, |last| last + 1)
                };
                for attribute in attrs.iter_mut().take(attributes_moved) {
                    let value = match attribute {
                        JSXAttributeItem::SpreadAttribute(spread) => Some(&mut spread.argument),
                        JSXAttributeItem::Attribute(attribute) => attribute_value_mut(attribute),
                    };
                    if let Some(value) = value.filter(|value| !is_pure(value)) {
                        read_once.push(self.read_once(value));
                    }
                }
                for child in elem
                    .children
                    .iter_mut()
                    .take(last_child.map_or(0, |last| last + 1))
                {
                    match child {
                        JSXChild::ExpressionContainer(container) => {
                            if let Some(value) = container.expression.as_expression_mut()
                                && !is_pure(value)
                            {
                                read_once.push(self.read_once(value));
                            }
                        }
                        JSXChild::Spread(spread) => {
                            if !is_pure(&spread.expression) {
                                read_once.push(self.read_once(&mut spread.expression));
                            }
                        }
                        JSXChild::Element(element) => {
                            let mut value = Expression::JSXElement(
                                element.clone_in_with_semantic_ids(self.ast.allocator()),
                            );
                            read_once.push(self.read_once(&mut value));
                            *child = JSXChild::ExpressionContainer(JSXExpressionContainer::boxed(
                                SPAN,
                                value.into(),
                                &self.ast,
                            ));
                        }
                        JSXChild::Fragment(fragment) => {
                            let mut value = Expression::JSXFragment(
                                fragment.clone_in_with_semantic_ids(self.ast.allocator()),
                            );
                            read_once.push(self.read_once(&mut value));
                            *child = JSXChild::ExpressionContainer(JSXExpressionContainer::boxed(
                                SPAN,
                                value.into(),
                                &self.ast,
                            ));
                        }
                        JSXChild::Text(_) => {}
                    }
                }
            }

            if let ParsedStyleOrder::Conditional {
                condition,
                consequent,
                alternate,
            } = &parsed_style_order
            {
                // Clone styles for alternate branch before consequent processing mutates them
                let mut alt_props_styles: Vec<ExtractStyleProp<'a>> = props_styles
                    .iter()
                    .map(|s| s.clone_in(self.ast.allocator()))
                    .collect();

                // Process consequent branch
                let tailwind_styles_con = modify_props(
                    &self.ast,
                    attrs,
                    &mut props_styles,
                    *consequent,
                    style_vars,
                    props,
                    self.split_filename.as_deref(),
                    Some((
                        condition.clone_in(self.ast.allocator()),
                        &mut alt_props_styles,
                        *alternate,
                    )),
                );
                self.styles.extend(tailwind_styles_con);

                // Collect styles from both branches for CSS output
                props_styles.into_iter().rev().for_each(|style| {
                    self.styles
                        .extend(style.into_extract().into_iter().map(|mut s| {
                            if let Some(order) = consequent {
                                s.set_style_order(*order);
                            }
                            s
                        }));
                });
                alt_props_styles.into_iter().rev().for_each(|style| {
                    self.styles
                        .extend(style.into_extract().into_iter().map(|mut s| {
                            if let Some(order) = alternate {
                                s.set_style_order(*order);
                            }
                            s
                        }));
                });
            } else {
                let style_order = parsed_style_order.as_static();
                let tailwind_styles = modify_props(
                    &self.ast,
                    attrs,
                    &mut props_styles,
                    style_order,
                    style_vars,
                    props,
                    self.split_filename.as_deref(),
                    None,
                );
                self.styles.extend(tailwind_styles);

                props_styles
                    .into_iter()
                    .rev()
                    .for_each(|style| self.styles.extend(style.into_extract()));
            }

            for (offset, code) in unreadable {
                self.errors.push((
                    offset,
                    element_error(&elem.opening_element.name.to_string(), &code, STYLE_OBJECT),
                ));
            }

            // A type only the runtime gives is read once, before what the
            // element reads once
            let mut values = Vec::with_capacity(read_once.len() + 1);
            match crate::as_visit::resolve(&self.ast, elem, tag_name, default_tag) {
                As::Name(name) => crate::as_visit::rename(&self.ast, elem, name),
                As::Choice(choice) => self.pending_replacement = Some(choice),
                As::Runtime(value) => {
                    let name = format!("DevupAs{}", self.runtime_types);
                    self.runtime_types += 1;
                    crate::as_visit::rename(
                        &self.ast,
                        elem,
                        JSXElementName::new_identifier(
                            SPAN,
                            Str::from_in(name.as_str(), self.ast.allocator()),
                            &self.ast,
                        ),
                    );
                    values.push((name, value));
                }
            }
            values.extend(read_once);

            if !values.is_empty() {
                let element = self.pending_replacement.take().unwrap_or_else(|| {
                    Expression::JSXElement(oxc_allocator::Box::new_in(
                        elem.clone_in_with_semantic_ids(self.ast.allocator()),
                        &self.ast,
                    ))
                });
                self.pending_replacement = Some(call_with_values(&self.ast, values, element));
            }
        } else if let Some(css) = css {
            self.lower_css_prop(elem, css);
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use css::{class_map::reset_class_map, file_map::reset_file_map};
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use serial_test::serial;

    #[test]
    fn test_style_property_variable_into_string() {
        let value = style_property_into_string(StyleProperty::Variable {
            class_name: "dynamic".to_string(),
            variable_name: "--color".to_string(),
            identifier: "color".to_string(),
        });

        assert_eq!(value, "var(--color)");
    }

    #[test]
    #[serial]
    fn test_stylex_named_and_unrelated_imports() {
        reset_class_map();
        reset_file_map();
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("test.ts").unwrap();
        let mut program = Parser::new(&allocator, "import { create, props as sxProps, keyframes, unknown } from '@stylexjs/stylex'; import value from 'other'; const styles = create({ base: { color: 'red' } }); export const a = sxProps(styles.base); export const b = unknown(styles.base); export const animation = keyframes({ from: { opacity: 0 } });", source_type).parse().program;
        let mut visitor =
            DevupVisitor::new(&allocator, "test.ts", "@devup-ui/react", Vec::new(), None);

        visitor.visit_program(&mut program);

        let code = oxc_codegen::Codegen::new().build(&program).code;
        assert!(
            code.contains("const styles = { \"base\": \"a\" };"),
            "{code}"
        );
        assert!(
            code.contains("export const a = { className: styles.base };")
                || code.contains("export const a = { className: \"a\" };"),
            "{code}"
        );
        assert!(
            code.contains("export const b = unknown(styles.base);"),
            "{code}"
        );
        assert!(code.contains("export const animation = \""), "{code}");
        assert!(code.contains("import value from \"other\";"), "{code}");
    }

    #[test]
    #[serial]
    fn test_jsx_conditional_style_order() {
        reset_class_map();
        reset_file_map();
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("test.tsx").unwrap();
        let mut program = Parser::new(&allocator, "import { Box } from '@devup-ui/react'; const view = <Box styleOrder={active ? 1 : 2} p={4} padding={8} />;", source_type).parse().program;
        let mut visitor =
            DevupVisitor::new(&allocator, "test.tsx", "@devup-ui/react", Vec::new(), None);

        visitor.visit_program(&mut program);

        assert!(!visitor.styles.is_empty());
    }
}
