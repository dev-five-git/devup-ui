use crate::as_visit::As;
use crate::component::ExportVariableKind;
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
    extract_style_from_styled::{extract_style_from_styled, take_styled_modifiers},
};
use crate::gen_class_name::{gen_class_names, merge_expression_for_class_name};
use crate::prop_modify_utils::{convert_class_name, modify_prop_object, modify_props};
use crate::stylex::{
    StylexDynamicInfo, StylexFunction, StylexNamespaceValue, create_theme_class,
    css_variable_block, css_variable_rules, define_vars_variable, variable_values,
};
use crate::util_type::UtilType;
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::disassemble_property;
use css::is_special_property::is_special_property;
use css::keyframes_to_keyframes_name;
use oxc_allocator::{Allocator, CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::ImportDeclarationSpecifier::{self, ImportSpecifier};
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Argument, BinaryOperator, BindingPattern, CallExpression, ChainElement,
    ComputedMemberExpression, Expression, ExpressionStatement, FormalParameter,
    FormalParameterKind, FormalParameters, IdentifierName, ImportDeclaration, ImportOrExportKind,
    JSXAttributeItem, JSXAttributeValue, JSXChild, JSXElement, JSXElementName,
    JSXExpressionContainer, ObjectProperty, ObjectPropertyKind, Program, PropertyKey, PropertyKind,
    Statement, StaticMemberExpression, Str, StringLiteral, UnaryOperator, VariableDeclarator,
};
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::{
    walk_call_expression, walk_expression, walk_expression_statement, walk_import_declaration,
    walk_jsx_attribute_value, walk_jsx_child, walk_jsx_element, walk_program,
    walk_variable_declarator, walk_variable_declarators,
};
use oxc_syntax::number::NumberBase;
use strum::IntoEnumIterator;

use crate::utils::{
    ParsedStyleOrder, RUNTIME_VALUE, STYLE_OBJECT, StyleArguments, Suspends, build_time_error,
    call_with_values, element_error, expression_to_style_order, fixed_value,
    get_str_by_property_key, get_string_by_literal_expression, get_string_by_property_key, is_pure,
    jsx_expression_to_style_order, key_error, readable_argument, readable_code, reads_directly,
    reads_spreads_once, reads_unknown, runtime_classes, runtime_value, runtime_value_error,
    spread_error, stays_attribute, style_arguments, uncomposable_error, unplaced_error,
    unreadable_styles, unwrap_syntax_only, unwrap_syntax_only_mut,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, SPAN};
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::rc::Rc;

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
    imports: FxHashMap<String, ExportVariableKind>,
    import_object: Option<String>,
    jsx_imports: FxHashMap<String, String>,
    util_imports: FxHashMap<String, Rc<UtilType>>,
    jsx_object: Option<String>,
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
    styled_imports: FxHashSet<String>,
    /// Tracked `StyleX` default/namespace import names (e.g., `stylex` from `import stylex from '...'`)
    stylex_imports: FxHashSet<String>,
    /// Tracked `StyleX` named imports (e.g., `create` from `import { create } from '...'`)
    stylex_named_imports: FxHashMap<String, StylexFunction>,
    /// Pending `StyleX` namespace map from the most recent `stylex.create()` call.
    /// Set in `visit_expression`, consumed in `visit_variable_declarator`.
    stylex_pending_create: Option<FxHashMap<String, StylexNamespaceValue>>,
    /// Maps variable names to their namespace→className mappings from `stylex.create()`.
    /// e.g., "styles" → { "base" → "a b", "active" → "c" }
    stylex_namespaces: FxHashMap<String, FxHashMap<String, StylexNamespaceValue>>,
    /// Local names bound to the `Global` component, whose `styles` prop declares
    /// global CSS instead of rendering markup.
    global_style_components: FxHashSet<String>,

    /// `defineVars` members flattened to `"vars.key"` -> `"var(--x)"`, so a
    /// `stylex.create()` value referencing one resolves to a static CSS value.
    stylex_var_refs: FxHashMap<String, String>,
    /// `defineVars` bindings as `vars` -> (`key` -> `--x`), the contract
    /// `createTheme` reassigns.
    stylex_var_names: FxHashMap<String, FxHashMap<String, String>>,
    /// `createTheme` bindings as `theme` -> `class`, so `stylex.props(theme)` resolves.
    stylex_theme_classes: FxHashMap<String, String>,
    /// Pending `defineVars` contract awaiting its variable declarator.
    stylex_pending_vars: Option<FxHashMap<String, String>>,
    /// Pending `createTheme` class awaiting its variable declarator.
    stylex_pending_theme_class: Option<String>,
    /// Pending `defineConsts` values awaiting their variable declarator.
    stylex_pending_consts: Option<FxHashMap<String, String>>,
    /// Pending keyframe animation name from most recent `stylex.keyframes()` call.
    stylex_pending_keyframe_name: Option<String>,
    /// Maps variable names to their keyframe animation names.
    /// e.g., "fadeIn" → "a-a"
    stylex_keyframe_names: FxHashMap<String, String>,
    /// What the element just visited becomes when it is not an element any
    /// more (a dynamic `as`, a spread evaluated once): set in
    /// `visit_jsx_element`, written where the element stands by the visit of
    /// the expression, child or attribute value holding it
    pending_replacement: Option<Expression<'a>>,
    spreads_read_once: usize,
    /// Elements whose type only the runtime gives, which each bind it to a
    /// name of their own
    runtime_types: usize,
    /// The classes and keyframes names the file binds to a `const`
    style_values: crate::style_values::StyleValues,
    /// What the build compiles away: the imports of the package it removes,
    /// and the bindings aliasing them, which only its calls and elements read
    compiled_names: FxHashSet<String>,
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
}

/// Whether `declarator` only aliases what the build compiles away
/// (`const newCss = css`)
fn is_alias(declarator: &VariableDeclarator<'_>, compiled: &FxHashSet<String>) -> bool {
    matches!(&declarator.init, Some(Expression::Identifier(init)) if compiled.contains(init.name.as_str()))
        && declarator
            .id
            .get_binding_identifier()
            .is_some_and(|id| compiled.contains(id.name.as_str()))
}

/// The reads of `names` a program keeps outside the types it erases; with
/// `scoping`, only those no binding of the program declares
struct CompiledReads<'s> {
    names: &'s FxHashSet<String>,
    scoping: Option<&'s oxc_semantic::Scoping>,
    found: Vec<(u32, String)>,
}

impl<'a> oxc_ast_visit::Visit<'a> for CompiledReads<'_> {
    fn visit_identifier_reference(&mut self, it: &oxc_ast::ast::IdentifierReference<'a>) {
        if self.names.contains(it.name.as_str())
            && self.scoping.is_none_or(|scoping| {
                it.reference_id
                    .get()
                    .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none())
            })
        {
            self.found.push((it.span.start, it.name.to_string()));
        }
    }

    fn visit_ts_type(&mut self, _: &oxc_ast::ast::TSType<'a>) {}
}

/// Whether `expression`, or a value it chooses, reads an object or array code
/// changes
fn reads_binding(
    expression: &Expression<'_>,
    changed: &crate::imported_constants::Changed,
) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::ArrayExpression(array) => array.elements.iter().any(|element| {
            element
                .as_expression()
                .is_some_and(|element| reads_binding(element, changed))
        }),
        Expression::LogicalExpression(logical) => {
            reads_binding(&logical.left, changed) || reads_binding(&logical.right, changed)
        }
        Expression::ConditionalExpression(conditional) => {
            reads_binding(&conditional.consequent, changed)
                || reads_binding(&conditional.alternate, changed)
        }
        expression => changed.read_by(expression),
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
            if reads_unknown(expression, &self.unknown_bindings) {
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
            if reads_binding(expression, &self.changed_bindings) {
                self.errors.push((
                    argument.span().start,
                    build_time_error(api, &readable_argument(argument), STYLE_OBJECT),
                ));
            }
        }
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
            imports: FxHashMap::default(),
            jsx_imports: FxHashMap::default(),
            package: package.to_string(),
            compat_package: format!("{package}/compat"),
            css_files,
            styles: FxHashSet::default(),
            errors: Vec::new(),
            import_object: None,
            jsx_object: None,
            util_imports: FxHashMap::default(),
            split_filename,
            styled_imports: FxHashSet::default(),
            stylex_imports: FxHashSet::default(),
            stylex_named_imports: FxHashMap::default(),
            global_style_components: FxHashSet::default(),
            stylex_var_refs: FxHashMap::default(),
            stylex_var_names: FxHashMap::default(),
            stylex_theme_classes: FxHashMap::default(),
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
            style_values: crate::style_values::StyleValues::default(),
            compiled_names: FxHashSet::default(),
            unknown_bindings: crate::imported_constants::Unknown::default(),
            composes_unknown: false,
            unknown_parts: Vec::new(),
            changed_bindings: crate::imported_constants::Changed::default(),
        }
    }

    pub fn unknown_bindings(&mut self, unknown: &crate::imported_constants::Unknown) {
        self.unknown_bindings.clone_from(unknown);
    }

    /// The style function `callee` names: `css`, `Devup.keyframes`, ...
    fn util_type(&self, callee: &Expression<'a>) -> Option<Rc<UtilType>> {
        match callee {
            Expression::Identifier(ident) => self.util_imports.get(ident.name.as_str()),
            Expression::StaticMemberExpression(member) if !self.util_imports.is_empty() => {
                let Expression::Identifier(ident) = &member.object else {
                    return None;
                };
                let obj = ident.name.as_str();
                let prop = member.property.name.as_str();
                let mut key = String::with_capacity(obj.len() + 1 + prop.len());
                key.push_str(obj);
                key.push('.');
                key.push_str(prop);
                self.util_imports.get(key.as_str())
            }
            _ => None,
        }
        .cloned()
    }

    /// Register what the package imported whole as `local` gives: its
    /// components, style functions, `styled` and `Global` as members
    fn import_whole(&mut self, local: &str) {
        for kind in ExportVariableKind::iter() {
            self.imports.insert(format!("{local}.{kind}"), kind);
        }
        for (name, kind) in [
            ("css", UtilType::Css),
            ("globalCss", UtilType::GlobalCss),
            ("keyframes", UtilType::Keyframes),
            ("createGlobalStyle", UtilType::GlobalCssComponent),
        ] {
            self.util_imports
                .insert(format!("{local}.{name}"), Rc::new(kind));
        }
        self.styled_imports.insert(format!("{local}.styled"));
        self.global_style_components
            .insert(format!("{local}.Global"));
    }

    /// `node` with a `styled` read as a member of the package imported whole
    /// (`Devup.styled.div`) written as a binding of the same text, which the
    /// extraction of `styled` reads like the named import
    fn plain_styled(&self, node: &mut Expression<'a>) {
        if let Expression::StaticMemberExpression(member) = unwrap_syntax_only(node)
            && member.property.name == "styled"
            && let Expression::Identifier(object) = &member.object
        {
            let name = format!("{}.styled", object.name);
            if self.styled_imports.contains(&name) {
                *node = Expression::new_identifier(
                    SPAN,
                    Str::from_in(name.as_str(), self.ast.allocator()),
                    &self.ast,
                );
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
        let mut reads = CompiledReads {
            names: &self.compiled_names,
            scoping: None,
            found: Vec::new(),
        };
        oxc_ast_visit::Visit::visit_program(&mut reads, program);
        if reads.found.is_empty() {
            return;
        }
        // A binding the code declares may share the name
        let scoping = oxc_semantic::SemanticBuilder::new()
            .build(program)
            .semantic
            .into_scoping();
        let mut reads = CompiledReads {
            names: &self.compiled_names,
            scoping: Some(&scoping),
            found: Vec::new(),
        };
        oxc_ast_visit::Visit::visit_program(&mut reads, program);
        for (offset, name) in reads.found {
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

    /// Whether `program` imports a style function whose result it may bind:
    /// `css()` gives a class, `keyframes()` a name
    fn binds_style_results(&self, program: &Program<'a>) -> bool {
        program.body.iter().any(|statement| {
            matches!(statement, Statement::ImportDeclaration(import)
            if (import.source.value == self.package
                || import.source.value == self.compat_package.as_str())
                && import.specifiers.iter().flatten().any(|specifier| match specifier {
                    ImportSpecifier(specifier) => {
                        matches!(specifier.imported.name().as_str(), "css" | "keyframes")
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
}

impl<'a> DevupVisitor<'a> {
    /// Resolve a `stylex.props(...)` / `stylex.attrs(...)` callee to the class attribute
    /// it produces. `props()` targets React (`className`), `attrs()` targets raw HTML
    /// (`class`); everything else about the two calls is identical.
    fn stylex_class_attribute(&self, callee: &Expression) -> Option<&'static str> {
        // Check namespace/default call: stylex.props(...)
        if let Expression::StaticMemberExpression(member) = callee
            && let Expression::Identifier(ident) = &member.object
            && self.stylex_imports.contains(ident.name.as_str())
        {
            return match member.property.name.as_str() {
                "props" => Some("className"),
                "attrs" => Some("class"),
                _ => None,
            };
        }
        // Check named import call: props(...)
        if let Expression::Identifier(ident) = callee {
            return match self.stylex_named_imports.get(ident.name.as_str()) {
                Some(StylexFunction::Props) => Some("className"),
                Some(StylexFunction::Attrs) => Some("class"),
                _ => None,
            };
        }
        None
    }

    /// `StyleX` variables and themes the program imports from other modules,
    /// by the name it binds them to
    pub fn import_stylex(
        &mut self,
        vars: FxHashMap<String, FxHashMap<String, String>>,
        themes: FxHashMap<String, String>,
    ) {
        for (name, contract) in vars {
            for (key, variable) in &contract {
                self.stylex_var_refs
                    .insert(format!("{name}.{key}"), format!("var({variable})"));
            }
            self.stylex_var_names.insert(name, contract);
        }
        self.stylex_theme_classes.extend(themes);
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

    /// The `StyleX` API a callee names, through either the namespace form
    /// (`stylex.defineVars`) or a named import.
    fn stylex_function(&self, callee: &Expression) -> Option<StylexFunction> {
        match callee {
            Expression::StaticMemberExpression(member)
                if matches!(&member.object, Expression::Identifier(ident)
                    if self.stylex_imports.contains(ident.name.as_str())) =>
            {
                StylexFunction::from_export_name(member.property.name.as_str())
            }
            Expression::Identifier(ident) => {
                self.stylex_named_imports.get(ident.name.as_str()).cloned()
            }
            _ => None,
        }
    }

    fn is_stylex_call(&self, callee: &Expression, function: &StylexFunction) -> bool {
        self.stylex_function(callee).as_ref() == Some(function)
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
            && let Expression::Identifier(obj) = &member.object
            && let Some(ns_map) = self.stylex_namespaces.get(obj.name.as_str())
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
                    let arg_expr =
                        self.with_number_unit(arg_expr.clone_in(self.ast.allocator()), unit);
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
        if let Expression::Identifier(obj) = &member.object
            && let Some(ns_map) = self.stylex_namespaces.get(obj.name.as_str())
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
        let Expression::Identifier(obj) = &member.object else {
            return None;
        };
        let ns_map = self.stylex_namespaces.get(obj.name.as_str())?;

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
                member.object.clone_in(self.ast.allocator()),
                member.expression.clone_in(self.ast.allocator()),
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
                .stylex_theme_classes
                .get(ident.name.as_str())
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
        matches!(object, Expression::Identifier(ident)
            if self.stylex_namespaces.contains_key(ident.name.as_str()))
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
                    let condition = logical.left.clone_in(self.ast.allocator());
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
                        let test = cond.test.clone_in(self.ast.allocator());
                        Some(Expression::new_conditional_expression(
                            SPAN, test, cons, alt, &self.ast,
                        ))
                    }
                    (Some(cons), None) => {
                        let test = cond.test.clone_in(self.ast.allocator());
                        Some(Expression::new_conditional_expression(
                            SPAN,
                            test,
                            cons,
                            Expression::new_string_literal(SPAN, "", None, &self.ast),
                            &self.ast,
                        ))
                    }
                    (None, Some(alt)) => {
                        let test = cond.test.clone_in(self.ast.allocator());
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
        for v in it.iter() {
            if let VariableDeclarator {
                id,
                init: Some(Expression::Identifier(ident)),
                ..
            } = v
                && let Some(name) = id.get_binding_identifier().map(|id| id.name.to_string())
            {
                let original = ident.name.as_str();
                if let Some(util) = self.util_imports.get(original).cloned() {
                    self.util_imports.insert(name.clone(), util);
                } else if self.styled_imports.contains(original) {
                    self.styled_imports.insert(name.clone());
                } else if let Some(kind) = self.imports.get(original).cloned() {
                    self.imports.insert(name.clone(), kind);
                } else {
                    continue;
                }
                self.compiled_names.insert(name);
            }
        }
        walk_variable_declarators(self, it);
    }

    fn visit_program(&mut self, it: &mut Program<'a>) {
        if self.binds_style_results(it) {
            self.style_values = crate::style_values::StyleValues::new(
                oxc_semantic::SemanticBuilder::new()
                    .build(it)
                    .semantic
                    .into_scoping(),
            );
        }
        walk_program(self, it);
        if !self.compiled_names.is_empty() {
            // Aliases only the calls and elements the build compiled read; at
            // the top level nothing can shadow what they alias
            let compiled = &self.compiled_names;
            it.body.retain_mut(|statement| {
                let Statement::VariableDeclaration(declaration) = statement else {
                    return true;
                };
                declaration
                    .declarations
                    .retain(|declarator| !is_alias(declarator, compiled));
                !declaration.declarations.is_empty()
            });
            self.report_compiled_reads(it);
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
        if !self.styled_imports.is_empty() {
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
        let styled_imports = &self.styled_imports;
        let attrs = factory.map_or_else(Vec::new, |factory| {
            take_styled_modifiers(&self.ast, factory, |name| styled_imports.contains(name))
        });
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
            && matches!(&call.callee, Expression::Identifier(ident) if self.styled_imports.contains(ident.name.as_str()))
        {
            call.arguments.truncate(1);
        }
        walk_expression(self, it);

        // Handle styled function calls
        if !self.styled_imports.is_empty() {
            let (tag_or_call, argument_count) = match it {
                Expression::TaggedTemplateExpression(tag) => (Some(&tag.tag), 0),
                Expression::CallExpression(call) => (Some(&call.callee), call.arguments.len()),
                _ => (None, 0),
            };
            let is_styled_name = |name: &str| self.styled_imports.contains(name);

            let is_styled = if let Some(tag_or_call) = tag_or_call.map(unwrap_syntax_only) {
                if let Expression::StaticMemberExpression(member) = tag_or_call {
                    if let Expression::Identifier(ident) = &member.object {
                        is_styled_name(ident.name.as_str())
                    } else {
                        false
                    }
                } else if let Expression::CallExpression(call) = tag_or_call {
                    if let Expression::Identifier(ident) = &call.callee {
                        is_styled_name(ident.name.as_str())
                    } else {
                        false
                    }
                } else if let Expression::Identifier(ident) = tag_or_call {
                    // styled("div", { ... }) puts the tag in the arguments, so the callee is
                    // the bare identifier. One argument is the curried creator `styled("div")`,
                    // which only becomes a component once its result is called.
                    is_styled_name(ident.name.as_str()) && argument_count == 2
                } else {
                    false
                }
            } else {
                false
            };

            if is_styled {
                self.style_values.read_in(&self.ast, it);
                if let Expression::CallExpression(call) = &*it {
                    self.unknown_arguments("styled", &call.arguments);
                    self.changed_arguments("styled", &call.arguments);
                }
                let (result, new_expr, errors) = extract_style_from_styled(
                    &self.ast,
                    it,
                    self.split_filename.as_deref(),
                    &self.imports,
                    &attrs,
                );
                self.errors.extend(errors);
                self.styles.extend(
                    result
                        .styles
                        .into_iter()
                        .flat_map(ExtractStyleProp::into_extract),
                );
                *it = new_expr;
            }
        }

        // Handle StyleX: stylex.create({...}) calls
        if let Expression::CallExpression(call) = it
            && self.is_stylex_call(&call.callee, &StylexFunction::Create)
            && let [Argument::ObjectExpression(arg)] = call.arguments.as_slice()
        {
            let namespaces = extract_stylex_namespace_styles(
                arg,
                &self.stylex_keyframe_names,
                &self.stylex_var_refs,
                &mut self.errors,
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
                    let Some(ns_value) = self
                        .stylex_namespaces
                        .get(&inc_ref.var_name)
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
                    let Some(values) = variable_values(&prop.value) else {
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
            && let Some(contract) = self.stylex_var_names.get(contract_ident.name.as_str())
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
                match variable_values(&prop.value) {
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
            let declarations = extract_stylex_declarations(api, arg, &mut self.errors);
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
            && let Some(function) = self.stylex_function(&call.callee)
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

        if let Expression::CallExpression(call) = it {
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
                                rules.clone_in(self.ast.allocator()),
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
                let mixins = statements
                    .iter()
                    .map(|index| tag.quasi.expressions[*index].clone_in(self.ast.allocator()));
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
        let jsx = if let Expression::Identifier(ident) = &it.callee {
            self.jsx_imports.get(ident.name.as_str()).cloned()
        } else if let Some(name) = &self.jsx_object
            && let Expression::StaticMemberExpression(member) = &it.callee
            && let Expression::Identifier(ident) = &member.object
            && name == ident.name.as_str()
        {
            Some(member.property.name.to_string())
        } else {
            None
        };
        if let Some(j) = jsx
            && (j == "jsx" || j == "jsxs")
            && let Some(expr) = it.arguments.first().and_then(|arg| arg.as_expression())
        {
            let element_kind = if let Expression::Identifier(ident) = expr {
                self.imports.get(ident.name.as_str()).cloned()
            } else if let Expression::StaticMemberExpression(member) = expr
                && let Expression::Identifier(ident) = &member.object
                && self.import_object.as_deref() == Some(ident.name.as_str())
            {
                member
                    .property
                    .name
                    .as_str()
                    .parse::<ExportVariableKind>()
                    .ok()
            } else {
                None
            };
            if let Some(kind) = element_kind
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
                        it.clone_in(self.ast.allocator()),
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
        if let Some(Expression::CallExpression(call)) = &it.init
            && call.arguments.len() == 1
            && let (Expression::Identifier(ident), Argument::StringLiteral(arg)) =
                (&call.callee, &call.arguments[0])
            && ident.name == "require"
        {
            if arg.value == "react/jsx-runtime" {
                if let BindingPattern::BindingIdentifier(ident) = &it.id {
                    self.jsx_object = Some(ident.name.to_string());
                } else if let BindingPattern::ObjectPattern(object) = &it.id {
                    for prop in &object.properties {
                        if let Some(name) = get_string_by_property_key(&prop.key)
                            && let Some(k) = prop
                                .value
                                .get_binding_identifier()
                                .map(|id| id.name.to_string())
                        {
                            self.jsx_imports.insert(k, name);
                        }
                    }
                }
            } else if arg.value == self.package {
                if let BindingPattern::BindingIdentifier(ident) = &it.id {
                    self.import_object = Some(ident.name.to_string());
                } else if let BindingPattern::ObjectPattern(object) = &it.id {
                    for prop in &object.properties {
                        if let Some(name) = get_string_by_property_key(&prop.key)
                            && let Ok(kind) = ExportVariableKind::try_from(
                                prop.value
                                    .get_binding_identifier()
                                    .map(|id| id.name.to_string())
                                    .unwrap_or_default(),
                            )
                        {
                            self.imports.insert(name, kind);
                        }
                    }
                }
            }
        }

        let style_result = match &it.init {
            Some(Expression::CallExpression(call)) => self.util_type(&call.callee),
            Some(Expression::TaggedTemplateExpression(tag)) => self.util_type(&tag.tag),
            _ => None,
        }
        .filter(|util| matches!(util.as_ref(), UtilType::Css | UtilType::Keyframes))
        .and_then(|util| Some((util, self.style_values.constant(&it.id)?)));

        walk_variable_declarator(self, it);

        if let Some((util, symbol)) = style_result
            && let Some(Expression::StringLiteral(value)) = &it.init
        {
            let value = value.value.to_string();
            self.style_values.insert(
                symbol,
                if matches!(util.as_ref(), UtilType::Css) {
                    crate::style_values::StyleValue::Class(value)
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

        // After walking, capture stylex.create() variable binding
        if let Some(pending) = self.stylex_pending_create.take()
            && let Some(ident) = it.id.get_binding_identifier()
        {
            self.stylex_namespaces
                .insert(ident.name.to_string(), pending);
        }

        // Capture stylex.keyframes() variable binding
        if let Some(name) = self.stylex_pending_keyframe_name.take()
            && let Some(ident) = it.id.get_binding_identifier()
        {
            self.stylex_keyframe_names
                .insert(ident.name.to_string(), name);
        }

        // Capture stylex.defineVars() variable binding
        if let Some(contract) = self.stylex_pending_vars.take()
            && let Some(ident) = it.id.get_binding_identifier()
        {
            for (key, variable) in &contract {
                self.stylex_var_refs
                    .insert(format!("{}.{key}", ident.name), format!("var({variable})"));
            }
            self.stylex_var_names
                .insert(ident.name.to_string(), contract);
        }

        // Capture stylex.createTheme() variable binding
        if let Some(class_name) = self.stylex_pending_theme_class.take()
            && let Some(ident) = it.id.get_binding_identifier()
        {
            self.stylex_theme_classes
                .insert(ident.name.to_string(), class_name);
        }

        // Capture stylex.defineConsts() variable binding
        if let Some(constants) = self.stylex_pending_consts.take()
            && let Some(ident) = it.id.get_binding_identifier()
        {
            for (key, value) in constants {
                self.stylex_var_refs
                    .insert(format!("{}.{key}", ident.name), value);
            }
        }
    }
    fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
        if it.source.value != self.package
            && it.source.value == "react/jsx-runtime"
            && let Some(specifiers) = &it.specifiers
        {
            for specifier in specifiers {
                if let ImportSpecifier(import) = specifier {
                    self.jsx_imports
                        .insert(import.local.to_string(), import.imported.to_string());
                }
            }
        } else if (it.source.value == self.package
            || it.source.value == self.compat_package.as_str())
            && let Some(specifiers) = &mut it.specifiers
        {
            for i in (0..specifiers.len()).rev() {
                match &specifiers[i] {
                    ImportSpecifier(import) => {
                        let imported_str = import.imported.to_string();
                        let local = import.local.to_string();
                        if let Ok(kind) = imported_str.parse::<ExportVariableKind>() {
                            self.imports.insert(local.clone(), kind);
                        } else if let Some(kind) = UtilType::from_str_opt(&imported_str) {
                            self.util_imports.insert(local.clone(), Rc::new(kind));
                        } else if imported_str == "styled" {
                            self.styled_imports.insert(local.clone());
                        } else {
                            // `Global` stays, rendering nothing, so its binding does too
                            if imported_str == "Global" {
                                self.global_style_components.insert(local);
                            }
                            continue;
                        }
                        self.compiled_names.insert(local);
                        specifiers.remove(i);
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                        self.import_whole(&specifier.local.name);
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                        self.import_whole(&specifier.local.name);
                    }
                }
            }
        } else if it.source.value == "@stylexjs/stylex" {
            if let Some(specifiers) = &it.specifiers {
                for specifier in specifiers {
                    match specifier {
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(default_spec) => {
                            self.stylex_imports
                                .insert(default_spec.local.name.to_string());
                        }
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns_spec) => {
                            self.stylex_imports.insert(ns_spec.local.name.to_string());
                        }
                        ImportSpecifier(named_spec) => {
                            let imported = named_spec.imported.to_string();
                            let local = named_spec.local.name.to_string();
                            if let Some(func) = StylexFunction::from_export_name(&imported) {
                                self.stylex_named_imports.insert(local, func);
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
        walk_jsx_element(self, elem);

        // `<Global styles={...} />` is Emotion's spelling of a global stylesheet.
        // Lift the rules out and strip every attribute, leaving a component that
        // renders nothing — the same shape `createGlobalStyle` collapses to.
        if let Some(name) = match &elem.opening_element.name {
            JSXElementName::Identifier(id) => Some(Cow::Borrowed(id.name.as_str())),
            JSXElementName::IdentifierReference(id) => Some(Cow::Borrowed(id.name.as_str())),
            JSXElementName::MemberExpression(member)
                if !self.global_style_components.is_empty() =>
            {
                Some(Cow::Owned(member.to_string()))
            }
            _ => None,
        } && self.global_style_components.contains(name.as_ref())
        {
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
        let kind = match &elem.opening_element.name {
            // Fast path: probe with `&str` directly, no allocation
            JSXElementName::Identifier(id) => self.imports.get(id.name.as_str()),
            JSXElementName::IdentifierReference(id) => self.imports.get(id.name.as_str()),
            name => self.imports.get(name.to_string().as_str()),
        };
        if let Some(kind) = kind {
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
                                    props = Some(expression.clone_in(self.ast.allocator()));
                                }
                            } else if property_name == "styleVars" {
                                if let Some(value) = attr.value.as_ref()
                                    && let JSXAttributeValue::ExpressionContainer(expr) = value
                                    && let Some(expression) = expr.expression.as_expression()
                                {
                                    style_vars = Some(expression.clone_in(self.ast.allocator()));
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
                        ) || reads_unknown(&spread.argument, &self.unknown_bindings));
                    if runtime && reads_binding(&spread.argument, &self.changed_bindings) {
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
                            let mut value =
                                Expression::JSXElement(element.clone_in(self.ast.allocator()));
                            read_once.push(self.read_once(&mut value));
                            *child = JSXChild::ExpressionContainer(JSXExpressionContainer::boxed(
                                SPAN,
                                value.into(),
                                &self.ast,
                            ));
                        }
                        JSXChild::Fragment(fragment) => {
                            let mut value =
                                Expression::JSXFragment(fragment.clone_in(self.ast.allocator()));
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
                        elem.clone_in(self.ast.allocator()),
                        &self.ast,
                    ))
                });
                self.pending_replacement = Some(call_with_values(&self.ast, values, element));
            }
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
    fn test_stylex_named_and_unrelated_imports() {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("test.ts").unwrap();
        let mut program = Parser::new(&allocator, "import { create, props as sxProps, keyframes, unknown } from '@stylexjs/stylex'; import value from 'other';", source_type).parse().program;
        let mut visitor =
            DevupVisitor::new(&allocator, "test.ts", "@devup-ui/react", Vec::new(), None);

        visitor.visit_program(&mut program);

        assert_eq!(
            visitor.stylex_named_imports.get("create"),
            Some(&StylexFunction::Create)
        );
        assert_eq!(
            visitor.stylex_named_imports.get("sxProps"),
            Some(&StylexFunction::Props)
        );
        assert_eq!(
            visitor.stylex_named_imports.get("keyframes"),
            Some(&StylexFunction::Keyframes)
        );
        assert!(!visitor.stylex_named_imports.contains_key("unknown"));
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
