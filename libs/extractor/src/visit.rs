use crate::as_visit::AsVisitor;
use crate::component::ExportVariableKind;
use crate::css_utils::{css_to_style_literal, keyframes_to_keyframes_style, optimize_css_block};
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
    extract_style_from_expression::{LiteralHandling, extract_style_from_expression},
    extract_style_from_jsx::extract_style_from_jsx,
    extract_style_from_styled::extract_style_from_styled,
};
use crate::gen_class_name::{gen_class_names, merge_expression_for_class_name};
use crate::prop_modify_utils::{convert_class_name, modify_prop_object, modify_props};
use crate::stylex::{StylexDynamicInfo, StylexFunction, StylexNamespaceValue, css_variable_block};
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
    Argument, BindingPattern, CallExpression, ChainElement, ComputedMemberExpression, Expression,
    ExpressionStatement, FormalParameterKind, FormalParameters, IdentifierName, ImportDeclaration,
    ImportOrExportKind, JSXAttributeItem, JSXAttributeValue, JSXChild, JSXClosingFragment,
    JSXElement, JSXElementName, JSXExpressionContainer, JSXOpeningFragment, ObjectPropertyKind,
    Program, PropertyKey, PropertyKind, Statement, StaticMemberExpression, Str, StringLiteral,
    VariableDeclarator,
};
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::{
    walk_call_expression, walk_expression, walk_import_declaration, walk_jsx_element, walk_program,
    walk_variable_declarator, walk_variable_declarators,
};
use strum::IntoEnumIterator;

use crate::utils::{
    ParsedStyleOrder, expression_to_style_order, get_str_by_property_key,
    get_string_by_literal_expression, get_string_by_property_key, jsx_expression_to_style_order,
    unwrap_syntax_only,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use rustc_hash::{FxHashMap, FxHashSet};
use std::rc::Rc;

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
    styled_import: Option<String>,
    /// Tracked `StyleX` default/namespace import name (e.g., `stylex` from `import stylex from '...'`)
    stylex_import: Option<String>,
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
    /// Pending `JSXFragment` children from dynamic `as` prop resolution.
    /// Set in `visit_jsx_element`, consumed in `visit_expression` to replace
    /// `Expression::JSXElement` with `Expression::JSXFragment`.
    pending_fragment_children: Option<oxc_allocator::Vec<'a, JSXChild<'a>>>,
}

impl<'a> DevupVisitor<'a> {
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
            import_object: None,
            jsx_object: None,
            util_imports: FxHashMap::default(),
            split_filename,
            styled_import: None,
            stylex_import: None,
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
            pending_fragment_children: None,
        }
    }
}

impl<'a> DevupVisitor<'a> {
    /// Check if a callee expression is a `stylex.create(...)` or named `create(...)` call.
    fn is_stylex_create_call(&self, callee: &Expression) -> bool {
        // Check namespace/default call: stylex.create(...)
        if let Some(stylex_name) = &self.stylex_import
            && let Expression::StaticMemberExpression(member) = callee
            && let Expression::Identifier(ident) = &member.object
            && ident.name.as_str() == stylex_name.as_str()
            && member.property.name.as_str() == "create"
        {
            return true;
        }
        // Check named import call: create(...)
        if let Expression::Identifier(ident) = callee
            && matches!(
                self.stylex_named_imports.get(ident.name.as_str()),
                Some(StylexFunction::Create)
            )
        {
            return true;
        }
        false
    }

    /// Resolve a `stylex.props(...)` / `stylex.attrs(...)` callee to the class attribute
    /// it produces. `props()` targets React (`className`), `attrs()` targets raw HTML
    /// (`class`); everything else about the two calls is identical.
    fn stylex_class_attribute(&self, callee: &Expression) -> Option<&'static str> {
        // Check namespace/default call: stylex.props(...)
        if let Some(stylex_name) = &self.stylex_import
            && let Expression::StaticMemberExpression(member) = callee
            && let Expression::Identifier(ident) = &member.object
            && ident.name.as_str() == stylex_name.as_str()
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

    /// Check if a callee resolves to the given `StyleX` API, through either the
    /// namespace form (`stylex.defineVars`) or a named import.
    fn is_stylex_call(&self, callee: &Expression, function: &StylexFunction) -> bool {
        if let Some(stylex_name) = &self.stylex_import
            && let Expression::StaticMemberExpression(member) = callee
            && let Expression::Identifier(ident) = &member.object
            && ident.name.as_str() == stylex_name.as_str()
        {
            return StylexFunction::from_export_name(member.property.name.as_str())
                .is_some_and(|found| &found == function);
        }
        if let Expression::Identifier(ident) = callee {
            return self.stylex_named_imports.get(ident.name.as_str()) == Some(function);
        }
        false
    }

    /// Check if a callee is `stylex.keyframes()` or named `keyframes()` call.
    fn is_stylex_keyframes_call(&self, callee: &Expression) -> bool {
        if let Some(stylex_name) = &self.stylex_import
            && let Expression::StaticMemberExpression(member) = callee
            && let Expression::Identifier(ident) = &member.object
            && ident.name.as_str() == stylex_name.as_str()
            && member.property.name.as_str() == "keyframes"
        {
            return true;
        }
        if let Expression::Identifier(ident) = callee
            && matches!(
                self.stylex_named_imports.get(ident.name.as_str()),
                Some(StylexFunction::Keyframes)
            )
        {
            return true;
        }
        false
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
            // `...spread` carries no statically resolvable namespace reference.
            let Some(expr) = arg.as_expression() else {
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
            for (param_idx, var_name) in &info.css_vars {
                if let Some(arg) = call.arguments.get(*param_idx)
                    && let Some(arg_expr) = arg.as_expression()
                {
                    let arg_expr = arg_expr.clone_in(self.ast.allocator());
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
            Expression::new_identifier(SPAN, "", &self.ast)
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
            // styles.base → StaticMemberExpression
            Expression::StaticMemberExpression(member) => self.resolve_stylex_static_member(member),
            // colorStyles[color] / styles['base'] → ComputedMemberExpression
            Expression::ComputedMemberExpression(member) => {
                self.resolve_stylex_computed_member(member)
            }
            // darkTheme → Identifier bound to a stylex.createTheme() class
            Expression::Identifier(ident) => self
                .stylex_theme_classes
                .get(ident.name.as_str())
                .and_then(|class_name| self.stylex_class_literal(class_name)),
            // styles?.base / styles?.[color] → ChainExpression
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::StaticMemberExpression(member) => {
                    self.resolve_stylex_static_member(member)
                }
                ChainElement::ComputedMemberExpression(member) => {
                    self.resolve_stylex_computed_member(member)
                }
                _ => None,
            },
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
            Expression::NumericLiteral(n) if n.value == 0.0 => None,
            Expression::StringLiteral(s) if s.value.is_empty() => None,
            // Anything else we can't resolve → skip
            _ => None,
        }
    }
}

impl<'a> VisitMut<'a> for DevupVisitor<'a> {
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
                && let Some(css_import_key) = self.util_imports.get(ident.name.as_str())
                && let Some(name) = id.get_binding_identifier().map(|id| id.name.to_string())
            {
                self.util_imports.insert(name, css_import_key.clone());
            }
        }
        walk_variable_declarators(self, it);
    }

    fn visit_program(&mut self, it: &mut Program<'a>) {
        walk_program(self, it);
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
        walk_expression(self, it);

        // Handle styled function calls
        if let Some(styled_name) = &self.styled_import {
            let (tag_or_call, argument_count) = match it {
                Expression::TaggedTemplateExpression(tag) => (Some(&tag.tag), 0),
                Expression::CallExpression(call) => (Some(&call.callee), call.arguments.len()),
                _ => (None, 0),
            };

            let is_styled = if let Some(tag_or_call) = tag_or_call.map(unwrap_syntax_only) {
                if let Expression::StaticMemberExpression(member) = tag_or_call {
                    if let Expression::Identifier(ident) = &member.object {
                        ident.name.as_str() == styled_name.as_str()
                    } else {
                        false
                    }
                } else if let Expression::CallExpression(call) = tag_or_call {
                    if let Expression::Identifier(ident) = &call.callee {
                        ident.name.as_str() == styled_name.as_str()
                    } else {
                        false
                    }
                } else if let Expression::Identifier(ident) = tag_or_call {
                    // styled("div", { ... }) puts the tag in the arguments, so the callee is
                    // the bare identifier. One argument is the curried creator `styled("div")`,
                    // which only becomes a component once its result is called.
                    ident.name.as_str() == styled_name.as_str() && argument_count == 2
                } else {
                    false
                }
            } else {
                false
            };

            if is_styled {
                let (result, new_expr) = extract_style_from_styled(
                    &self.ast,
                    it,
                    self.split_filename.as_deref(),
                    &self.imports,
                );
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
            && self.is_stylex_create_call(&call.callee)
            && let [arg] = call.arguments.as_mut_slice()
            && let Some(arg) = arg.as_expression_mut()
        {
            let namespaces = extract_stylex_namespace_styles(
                arg,
                &self.stylex_keyframe_names,
                &self.stylex_var_refs,
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
                    if let Some(ns) = self.stylex_namespaces.get(&inc_ref.var_name)
                        && let Some(ns_value) = ns.get(&inc_ref.member_name)
                    {
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
                let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                    continue;
                };
                let (Some(key), Some(value)) = (
                    get_string_by_property_key(&prop.key),
                    get_string_by_literal_expression(&prop.value),
                ) else {
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
            let mut assignments = vec![];
            let mut properties = oxc_allocator::Vec::new_in(&self.ast);
            for prop in &obj.properties {
                let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                    continue;
                };
                let Some(key) = get_string_by_property_key(&prop.key) else {
                    continue;
                };
                let value = get_string_by_literal_expression(&prop.value);
                if publishes_values && value.is_none() {
                    continue;
                }
                let variable = format!(
                    "--{}",
                    keyframes_to_keyframes_name(
                        &format!("sxv-{}-{key}", self.filename),
                        self.split_filename.as_deref(),
                    )
                );
                if let Some(value) = value {
                    assignments.push((variable.clone(), value.into_owned()));
                }
                properties.push(self.string_property(&key, &format!("var({variable})")));
                contract.insert(key, variable);
            }
            if publishes_values && !assignments.is_empty() {
                self.styles.insert(ExtractStyleValue::Css(ExtractCss {
                    css: css_variable_block(":root", &assignments),
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
            let assignments: Vec<(String, String)> = obj
                .properties
                .iter()
                .filter_map(|prop| {
                    let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                        return None;
                    };
                    let key = get_string_by_property_key(&prop.key)?;
                    let value = get_string_by_literal_expression(&prop.value)?;
                    Some((contract.get(&key)?.clone(), value.into_owned()))
                })
                .collect();
            let class_name = keyframes_to_keyframes_name(
                &format!("sxt-{}-{}", self.filename, contract_ident.name),
                self.split_filename.as_deref(),
            );
            if !assignments.is_empty() {
                self.styles.insert(ExtractStyleValue::Css(ExtractCss {
                    css: css_variable_block(&format!(".{class_name}"), &assignments),
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
            && let [arg] = call.arguments.as_mut_slice()
            && let Some(arg) = arg.as_expression_mut()
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
            let declarations = extract_stylex_declarations(arg);
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
            && self.is_stylex_keyframes_call(&call.callee)
            && let [arg] = call.arguments.as_mut_slice()
            && let Some(arg) = arg.as_expression_mut()
        {
            let KeyframesExtractResult { keyframes } =
                extract_keyframes_from_expression(&self.ast, arg);
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
        // create/keyframes call is one whose argument could not be read statically.
        if let Expression::CallExpression(call) = it
            && (self.is_stylex_create_call(&call.callee)
                || self.is_stylex_keyframes_call(&call.callee))
        {
            eprintln!(
                "[stylex] ERROR: stylex.create()/keyframes() require exactly one object literal argument. Spread arguments cannot be resolved at build time."
            );
        }

        if let Expression::CallExpression(call) = it {
            let util_type = if let Expression::Identifier(ident) = &call.callee {
                self.util_imports.get(ident.name.as_str())
            } else if let Expression::StaticMemberExpression(member) = &call.callee
                && let Expression::Identifier(ident) = &member.object
                && !self.util_imports.is_empty()
            {
                let obj = ident.name.as_str();
                let prop = member.property.name.as_str();
                let mut key = String::with_capacity(obj.len() + 1 + prop.len());
                key.push_str(obj);
                key.push('.');
                key.push_str(prop);
                self.util_imports.get(key.as_str())
            } else {
                None
            };

            if let Some(util_type) = util_type {
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
                        let KeyframesExtractResult { keyframes } =
                            extract_keyframes_from_expression(
                                &self.ast,
                                if let Argument::SpreadElement(spread) = &mut call.arguments[0] {
                                    &mut spread.argument
                                } else {
                                    call.arguments[0].to_expression_mut()
                                },
                            );

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
                        // already set style order
                        self.styles.extend(styles.into_iter().flat_map(|mut ex| {
                            if let ExtractStyleProp::Static(css) = &mut ex {
                                css.set_style_order(style_order.unwrap_or(0));
                            }
                            ex.into_extract()
                        }));
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
                    self.styles.extend(styles.into_iter().flat_map(|mut ex| {
                        if let ExtractStyleProp::Static(css) = &mut ex {
                            css.set_style_order(style_order.unwrap_or(0));
                        }
                        ex.into_extract()
                    }));
                    *it = self.global_css_result(util_type.is_component());
                } else {
                    *it = match util_type.as_ref() {
                        UtilType::Css | UtilType::Keyframes => {
                            Expression::new_string_literal(SPAN, "", None, &self.ast)
                        }
                        global => self.global_css_result(global.is_component()),
                    };
                }
            }
        } else if let Expression::TaggedTemplateExpression(tag) = it
            && let Expression::Identifier(ident) = &tag.tag
            && let Some(css_type) = self.util_imports.get(ident.name.as_str())
        {
            // Only the Keyframes and GlobalCss arms need the concatenated quasi string
            let build_css_str = || {
                let mut s = String::new();
                for quasi in &tag.quasi.quasis {
                    s.push_str(quasi.value.raw.as_str());
                }
                s
            };
            let r = css_type.as_ref();
            *it = if matches!(r, UtilType::Css) {
                let mut style_props = css_to_style_literal(&tag.quasi, 0, &None)
                    .into_iter()
                    .map(|ex| ExtractStyleProp::Static(ex.into()))
                    .collect::<Vec<_>>();
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
                if let Some(cls) = class_name {
                    cls
                } else {
                    Expression::new_string_literal(SPAN, "", None, &self.ast)
                }
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

        // Replace JSXElement with JSXFragment when dynamic `as` prop produced an empty name
        if let Some(children) = self.pending_fragment_children.take() {
            *it = Expression::new_jsx_fragment(
                SPAN,
                JSXOpeningFragment::new(SPAN, &self.ast),
                children,
                JSXClosingFragment::new(SPAN, &self.ast),
                &self.ast,
            );
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

        walk_variable_declarator(self, it);

        // Phase 4c: Check for destructuring of stylex.create()
        if self.stylex_pending_create.is_some() && it.id.get_binding_identifier().is_none() {
            eprintln!(
                "[stylex] ERROR: Destructuring stylex.create() is not supported. Assign the result to a single variable (e.g., `const styles = stylex.create({{...}})`)."
            );
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
                        if let Ok(kind) = imported_str.parse::<ExportVariableKind>() {
                            self.imports.insert(import.local.to_string(), kind);
                            specifiers.remove(i);
                        } else if let Some(kind) = UtilType::from_str_opt(&imported_str) {
                            self.util_imports
                                .insert(import.local.to_string(), Rc::new(kind));
                            specifiers.remove(i);
                        } else if imported_str == "styled" {
                            self.styled_import = Some(import.local.to_string());
                            specifiers.remove(i);
                        } else if imported_str == "Global" {
                            self.global_style_components
                                .insert(import.local.to_string());
                            specifiers.remove(i);
                        }
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(
                        import_default_specifier,
                    ) => {
                        for kind in ExportVariableKind::iter() {
                            self.imports.insert(
                                format!("{}.{}", import_default_specifier.local, kind),
                                kind,
                            );
                        }
                        self.util_imports.insert(
                            format!("{}.{}", import_default_specifier.local, "css"),
                            Rc::new(UtilType::Css),
                        );

                        self.util_imports.insert(
                            format!("{}.{}", import_default_specifier.local, "globalCss"),
                            Rc::new(UtilType::GlobalCss),
                        );
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                        import_namespace_specifier,
                    ) => {
                        for kind in ExportVariableKind::iter() {
                            self.imports.insert(
                                format!("{}.{}", import_namespace_specifier.local, kind),
                                kind,
                            );
                        }
                        self.util_imports.insert(
                            format!("{}.{}", import_namespace_specifier.local, "css"),
                            Rc::new(UtilType::Css),
                        );
                        self.util_imports.insert(
                            format!("{}.{}", import_namespace_specifier.local, "globalCss"),
                            Rc::new(UtilType::GlobalCss),
                        );
                    }
                }
            }
        } else if it.source.value == "@stylexjs/stylex" {
            if let Some(specifiers) = &it.specifiers {
                for specifier in specifiers {
                    match specifier {
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(default_spec) => {
                            self.stylex_import = Some(default_spec.local.name.to_string());
                        }
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns_spec) => {
                            self.stylex_import = Some(ns_spec.local.name.to_string());
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
            JSXElementName::Identifier(id) => Some(id.name.as_str()),
            JSXElementName::IdentifierReference(id) => Some(id.name.as_str()),
            _ => None,
        } && self.global_style_components.contains(name)
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
                    let GlobalExtractResult {
                        styles,
                        style_order,
                    } = extract_global_style_from_expression(&self.ast, expression, &self.filename);
                    self.styles.extend(styles.into_iter().flat_map(|mut ex| {
                        if let ExtractStyleProp::Static(css) = &mut ex {
                            css.set_style_order(style_order.unwrap_or(0));
                        }
                        ex.into_extract()
                    }));
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
            let attrs = &mut elem.opening_element.attributes;
            let mut tag_name = Expression::new_string_literal(
                SPAN,
                Str::from_in(kind.to_tag(), self.ast.allocator()),
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
                                let ExtractResult { styles, tag, .. } =
                                    extract_style_from_jsx(&self.ast, &disassembled, at);
                                props_styles.extend(styles.into_iter().rev());
                                tag_name = tag.unwrap_or(tag_name);
                            }
                            duplicate_set.insert(disassembled);
                        }
                    }
                } else if let JSXAttributeItem::SpreadAttribute(spread) = &mut attr {
                    // Extract styles from spread attributes (e.g., {...{"@media": {...}}})
                    let ExtractResult { styles, .. } = extract_style_from_expression(
                        &self.ast,
                        None,
                        &mut spread.argument,
                        0,
                        &None,
                        LiteralHandling::ExpandResponsiveThemeToken,
                    );
                    if styles.is_empty() {
                        attrs.insert(i, attr);
                    } else {
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

            if let Some(tag) = if let Expression::StringLiteral(str) = tag_name {
                Some(str.value.as_str())
            } else if let Expression::TemplateLiteral(literal) = tag_name {
                Some(literal.quasis[0].value.raw.as_str())
            } else {
                let mut v =
                    AsVisitor::new(self.ast.allocator(), elem.clone_in(self.ast.allocator()));
                let mut el = ExpressionStatement::new(SPAN, tag_name, &self.ast);
                v.visit_expression_statement(&mut el);
                let mut children = oxc_allocator::Vec::new_in(&self.ast);
                children.push(JSXChild::ExpressionContainer(
                    JSXExpressionContainer::boxed(
                        SPAN,
                        el.expression.clone_in(self.ast.allocator()).into(),
                        &self.ast,
                    ),
                ));
                self.pending_fragment_children = Some(children);
                None
            } {
                let ident = JSXElementName::new_identifier(
                    SPAN,
                    Str::from_in(tag, self.ast.allocator()),
                    &self.ast,
                );

                elem.opening_element.name = ident.clone_in(self.ast.allocator());
                if let Some(el) = &mut elem.closing_element {
                    el.name = ident;
                }
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
