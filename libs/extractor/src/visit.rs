use crate::as_visit::AsVisitor;
use crate::component::ExportVariableKind;
use crate::css_utils::{css_to_style_literal, keyframes_to_keyframes_style, optimize_css_block};
use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::extract_css::ExtractCss;
use crate::extract_style::extract_keyframes::ExtractKeyframes;
use crate::extractor::KeyframesExtractResult;
use crate::extractor::extract_keyframes_from_expression::extract_keyframes_from_expression;
use crate::extractor::extract_style_from_stylex::extract_stylex_namespace_styles;
use crate::extractor::{
    ExtractResult, GlobalExtractResult,
    extract_global_style_from_expression::extract_global_style_from_expression,
    extract_style_from_expression::{LiteralHandling, extract_style_from_expression},
    extract_style_from_jsx::extract_style_from_jsx,
    extract_style_from_styled::extract_style_from_styled,
};
use crate::gen_class_name::{gen_class_names, merge_expression_for_class_name};
use crate::prop_modify_utils::{modify_prop_object, modify_props};
use crate::stylex::{StylexDynamicInfo, StylexFunction, StylexNamespaceValue};
use crate::util_type::UtilType;
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::disassemble_property;
use css::is_special_property::is_special_property;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::ImportDeclarationSpecifier::{self, ImportSpecifier};
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, ImportDeclaration, ImportOrExportKind,
    JSXAttributeItem, JSXAttributeValue, JSXChild, JSXElement, ObjectPropertyKind, Program,
    PropertyKey, PropertyKind, Statement, VariableDeclarator, WithClause,
};
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::{
    walk_call_expression, walk_expression, walk_import_declaration, walk_jsx_element, walk_program,
    walk_variable_declarator, walk_variable_declarators,
};
use strum::IntoEnumIterator;

use crate::utils::{
    ParsedStyleOrder, expression_to_style_order, get_string_by_property_key,
    jsx_expression_to_style_order,
};
use oxc_ast::AstBuilder;
use oxc_span::SPAN;
use rustc_hash::{FxHashMap, FxHashSet};
use std::rc::Rc;

pub struct DevupVisitor<'a> {
    pub ast: AstBuilder<'a>,
    filename: String,
    imports: FxHashMap<String, ExportVariableKind>,
    import_object: Option<String>,
    jsx_imports: FxHashMap<String, String>,
    util_imports: FxHashMap<String, Rc<UtilType>>,
    jsx_object: Option<String>,
    package: String,
    split_filename: Option<String>,
    pub css_files: Vec<String>,
    pub styles: FxHashSet<ExtractStyleValue>,
    styled_import: Option<String>,
    /// Tracked StyleX default/namespace import name (e.g., `stylex` from `import stylex from '...'`)
    stylex_import: Option<String>,
    /// Tracked StyleX named imports (e.g., `create` from `import { create } from '...'`)
    stylex_named_imports: FxHashMap<String, StylexFunction>,
    /// Pending StyleX namespace map from the most recent stylex.create() call.
    /// Set in visit_expression, consumed in visit_variable_declarator.
    stylex_pending_create: Option<FxHashMap<String, StylexNamespaceValue>>,
    /// Maps variable names to their namespace→className mappings from stylex.create().
    /// e.g., "styles" → { "base" → "a b", "active" → "c" }
    stylex_namespaces: FxHashMap<String, FxHashMap<String, StylexNamespaceValue>>,
    /// Pending keyframe animation name from most recent stylex.keyframes() call.
    stylex_pending_keyframe_name: Option<String>,
    /// Maps variable names to their keyframe animation names.
    /// e.g., "fadeIn" → "a-a"
    stylex_keyframe_names: FxHashMap<String, String>,
    /// Pending JSXFragment children from dynamic `as` prop resolution.
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
            css_files,
            styles: FxHashSet::default(),
            import_object: None,
            jsx_object: None,
            util_imports: FxHashMap::default(),
            split_filename,
            styled_import: None,
            stylex_import: None,
            stylex_named_imports: FxHashMap::default(),
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
            && let Some(StylexFunction::Create) = self.stylex_named_imports.get(ident.name.as_str())
        {
            return true;
        }
        false
    }

    /// Check if a callee expression is a `stylex.props(...)` or named `props(...)` call.
    fn is_stylex_props_call(&self, callee: &Expression) -> bool {
        // Check namespace/default call: stylex.props(...)
        if let Some(stylex_name) = &self.stylex_import
            && let Expression::StaticMemberExpression(member) = callee
            && let Expression::Identifier(ident) = &member.object
            && ident.name.as_str() == stylex_name.as_str()
            && member.property.name.as_str() == "props"
        {
            return true;
        }
        // Check named import call: props(...)
        if let Expression::Identifier(ident) = callee
            && let Some(StylexFunction::Props) = self.stylex_named_imports.get(ident.name.as_str())
        {
            return true;
        }
        false
    }

    /// Check if a callee is stylex.keyframes() or named keyframes() call.
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
            && let Some(StylexFunction::Keyframes) =
                self.stylex_named_imports.get(ident.name.as_str())
        {
            return true;
        }
        false
    }

    /// Resolve stylex.props() arguments to className expressions and style properties.
    /// Returns (class_exprs, style_props) where style_props are CSS variable assignments
    /// from dynamic namespace calls like `styles.bar(h)`.
    fn resolve_stylex_props_args(
        &self,
        arguments: &mut oxc_allocator::Vec<'a, Argument<'a>>,
    ) -> (Vec<Expression<'a>>, Vec<ObjectPropertyKind<'a>>) {
        let mut class_exprs: Vec<Expression<'a>> = vec![];
        let mut style_props: Vec<ObjectPropertyKind<'a>> = vec![];

        for arg in arguments.iter() {
            let expr = arg.to_expression();
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

    /// Resolve a dynamic namespace call like `styles.bar(h)` to (className, style_props).
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
            let class_expr =
                self.ast
                    .expression_string_literal(SPAN, self.ast.str(&info.class_name), None);

            let mut props = vec![];
            for (param_idx, var_name) in &info.css_vars {
                if let Some(arg) = call.arguments.get(*param_idx) {
                    let arg_expr = arg.to_expression().clone_in(self.ast.allocator);
                    props.push(self.ast.object_property_kind_object_property(
                        SPAN,
                        PropertyKind::Init,
                        PropertyKey::StringLiteral(self.ast.alloc_string_literal(
                            SPAN,
                            self.ast.str(var_name),
                            None,
                        )),
                        arg_expr,
                        false,
                        false,
                        false,
                    ));
                }
            }

            Some((class_expr, props))
        } else {
            None
        }
    }

    /// Resolve a single stylex.props() argument to a className expression.
    fn resolve_stylex_arg(&self, expr: &Expression<'a>) -> Option<Expression<'a>> {
        match expr {
            // styles.base → StaticMemberExpression
            Expression::StaticMemberExpression(member) => {
                if let Expression::Identifier(obj) = &member.object
                    && let Some(ns_map) = self.stylex_namespaces.get(obj.name.as_str())
                    && let Some(StylexNamespaceValue::Static(cn)) =
                        ns_map.get(member.property.name.as_str())
                    && !cn.is_empty()
                {
                    return Some(
                        self.ast
                            .expression_string_literal(SPAN, self.ast.str(cn), None),
                    );
                }
                None
            }
            // isActive && styles.active → LogicalExpression(And)
            Expression::LogicalExpression(logical)
                if logical.operator == oxc_ast::ast::LogicalOperator::And =>
            {
                // The right side should be the namespace reference
                if let Some(class_expr) = self.resolve_stylex_arg(&logical.right) {
                    // Build: condition ? " className" : ""
                    let condition = logical.left.clone_in(self.ast.allocator);
                    Some(self.ast.expression_conditional(
                        SPAN,
                        condition,
                        class_expr,
                        self.ast.expression_string_literal(SPAN, "", None),
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
                        let test = cond.test.clone_in(self.ast.allocator);
                        Some(self.ast.expression_conditional(SPAN, test, cons, alt))
                    }
                    (Some(cons), None) => {
                        let test = cond.test.clone_in(self.ast.allocator);
                        Some(self.ast.expression_conditional(
                            SPAN,
                            test,
                            cons,
                            self.ast.expression_string_literal(SPAN, "", None),
                        ))
                    }
                    (None, Some(alt)) => {
                        let test = cond.test.clone_in(self.ast.allocator);
                        Some(self.ast.expression_conditional(
                            SPAN,
                            self.ast.expression_unary(
                                SPAN,
                                oxc_ast::ast::UnaryOperator::LogicalNot,
                                test,
                            ),
                            alt,
                            self.ast.expression_string_literal(SPAN, "", None),
                        ))
                    }
                    (None, None) => None,
                }
            }
            // false, null, undefined, 0, "" → falsy, skip
            Expression::BooleanLiteral(b) if !b.value => None,
            Expression::NullLiteral(_) => None,
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
                    Statement::ImportDeclaration(
                        self.ast.alloc_import_declaration::<Option<WithClause>>(
                            SPAN,
                            None,
                            self.ast.string_literal(SPAN, self.ast.str(css_file), None),
                            None,
                            None,
                            ImportOrExportKind::Value,
                        ),
                    ),
                );
            }
        }

        for i in (0..it.body.len()).rev() {
            if let Statement::ImportDeclaration(decl) = &it.body[i]
                && decl.source.value == self.package
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
            let tag_or_call = if let Expression::TaggedTemplateExpression(tag) = it {
                Some(&tag.tag)
            } else if let Expression::CallExpression(call) = it {
                Some(&call.callee)
            } else {
                None
            };

            let is_styled = if let Some(tag_or_call) = tag_or_call {
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
                self.styles
                    .extend(result.styles.into_iter().flat_map(|ex| ex.extract()));
                *it = new_expr;
            }
        }

        // Handle StyleX: stylex.create({...}) calls
        if let Expression::CallExpression(call) = it
            && self.is_stylex_create_call(&call.callee)
            && call.arguments.len() == 1
        {
            let arg = call.arguments[0].to_expression_mut();
            let namespaces =
                extract_stylex_namespace_styles(&self.ast, arg, &self.stylex_keyframe_names);

            let mut namespace_map: FxHashMap<String, StylexNamespaceValue> = FxHashMap::default();
            let mut properties = oxc_allocator::Vec::new_in(self.ast.allocator);
            for (ns_name, mut styles, css_vars, include_refs) in namespaces {
                let class_name =
                    gen_class_names(&self.ast, &mut styles, None, self.split_filename.as_deref());
                self.styles
                    .extend(styles.into_iter().flat_map(|ex| ex.extract()));

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
                                class_name_str = format!("{} {}", included_class, class_name_str);
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
                    self.ast
                        .expression_string_literal(SPAN, self.ast.str(&class_name_str), None)
                } else {
                    class_name.unwrap_or_else(|| {
                        self.ast
                            .expression_string_literal(SPAN, self.ast.str(""), None)
                    })
                };

                properties.push(self.ast.object_property_kind_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StringLiteral(self.ast.alloc_string_literal(
                        SPAN,
                        self.ast.str(&ns_name),
                        None,
                    )),
                    value,
                    false,
                    false,
                    false,
                ));
            }

            self.stylex_pending_create = Some(namespace_map);
            *it = self.ast.expression_object(SPAN, properties);
        }

        // Handle StyleX: stylex.keyframes({...}) calls
        if let Expression::CallExpression(call) = it
            && self.is_stylex_keyframes_call(&call.callee)
            && call.arguments.len() == 1
        {
            let arg = call.arguments[0].to_expression_mut();
            let KeyframesExtractResult { keyframes } =
                extract_keyframes_from_expression(&self.ast, arg);
            let name = keyframes
                .extract(self.split_filename.as_deref())
                .to_string();
            self.styles.insert(ExtractStyleValue::Keyframes(keyframes));
            self.stylex_pending_keyframe_name = Some(name.clone());
            *it = self
                .ast
                .expression_string_literal(SPAN, self.ast.str(&name), None);
        }

        // Handle StyleX: stylex.props(...) calls
        if let Expression::CallExpression(call) = it
            && self.is_stylex_props_call(&call.callee)
        {
            let (class_exprs, style_props) = self.resolve_stylex_props_args(&mut call.arguments);

            // Build className expression using existing merge utility
            let class_name_expr = merge_expression_for_class_name(&self.ast, class_exprs)
                .unwrap_or_else(|| self.ast.expression_string_literal(SPAN, "", None));

            // Build replacement: { className: <expr>, style?: { ... } }
            let mut props = oxc_allocator::Vec::new_in(self.ast.allocator);
            props.push(self.ast.object_property_kind_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StaticIdentifier(self.ast.alloc_identifier_name(SPAN, "className")),
                class_name_expr,
                false,
                false,
                false,
            ));

            // Add style property for dynamic CSS variables
            if !style_props.is_empty() {
                let style_obj = self.ast.expression_object(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(style_props, self.ast.allocator),
                );
                props.push(self.ast.object_property_kind_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StaticIdentifier(self.ast.alloc_identifier_name(SPAN, "style")),
                    style_obj,
                    false,
                    false,
                    false,
                ));
            }

            *it = self.ast.expression_object(SPAN, props);
        }

        if let Expression::CallExpression(call) = it {
            let util_import_key = if let Expression::Identifier(ident) = &call.callee {
                Some(ident.name.to_string())
            } else if let Expression::StaticMemberExpression(member) = &call.callee
                && let Expression::Identifier(ident) = &member.object
            {
                Some(format!("{}.{}", ident.name, member.property.name))
            } else {
                None
            };

            if let Some(util_import_key) = util_import_key
                && let Some(util_type) = self.util_imports.get(&util_import_key)
            {
                if call.arguments.len() != 1 {
                    *it = match util_type.as_ref() {
                        UtilType::Css | UtilType::Keyframes => {
                            self.ast
                                .expression_string_literal(SPAN, self.ast.str(""), None)
                        }
                        UtilType::GlobalCss => {
                            self.ast.expression_identifier(SPAN, self.ast.str(""))
                        }
                    };
                } else {
                    let r = util_type.as_ref();
                    *it = if let UtilType::Css = r {
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
                            self.ast
                                .expression_string_literal(SPAN, self.ast.str(""), None)
                        } else {
                            // css can not reachable
                            let class_name = gen_class_names(
                                &self.ast,
                                &mut styles,
                                style_order,
                                self.split_filename.as_deref(),
                            );

                            // already set style order
                            self.styles
                                .extend(styles.into_iter().flat_map(|ex| ex.extract()));
                            if let Some(cls) = class_name {
                                cls
                            } else {
                                self.ast
                                    .expression_string_literal(SPAN, self.ast.str(""), None)
                            }
                        }
                    } else if let UtilType::Keyframes = r {
                        let KeyframesExtractResult { keyframes } =
                            extract_keyframes_from_expression(
                                &self.ast,
                                if let Argument::SpreadElement(spread) = &mut call.arguments[0] {
                                    &mut spread.argument
                                } else {
                                    call.arguments[0].to_expression_mut()
                                },
                            );

                        let name = keyframes
                            .extract(self.split_filename.as_deref())
                            .to_string();
                        self.styles.insert(ExtractStyleValue::Keyframes(keyframes));
                        self.ast
                            .expression_string_literal(SPAN, self.ast.str(&name), None)
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
                            ex.extract()
                        }));
                        self.ast.expression_identifier(SPAN, self.ast.str(""))
                    }
                }
            }
        } else if let Expression::TaggedTemplateExpression(tag) = it
            && let Expression::Identifier(ident) = &tag.tag
            && let Some(css_type) = self.util_imports.get(ident.name.as_str())
        {
            let css_str = {
                let mut s = String::new();
                for quasi in tag.quasi.quasis.iter() {
                    s.push_str(quasi.value.raw.as_str());
                }
                s
            };
            let r = css_type.as_ref();
            *it = if let UtilType::Css = r {
                let styles = css_to_style_literal(&tag.quasi, 0, &None);
                let class_name = gen_class_names(
                    &self.ast,
                    &mut styles
                        .iter()
                        .map(|ex| ExtractStyleProp::Static(ex.clone().into()))
                        .collect::<Vec<_>>(),
                    None,
                    self.split_filename.as_deref(),
                );

                self.styles.extend(styles.into_iter().map(|ex| ex.into()));
                if let Some(cls) = class_name {
                    cls
                } else {
                    self.ast
                        .expression_string_literal(SPAN, self.ast.str(""), None)
                }
                // already set style order
            } else if let UtilType::Keyframes = r {
                let keyframes = ExtractKeyframes {
                    keyframes: keyframes_to_keyframes_style(&css_str),
                };
                let name = keyframes
                    .extract(self.split_filename.as_deref())
                    .to_string();

                self.styles.insert(ExtractStyleValue::Keyframes(keyframes));
                self.ast
                    .expression_string_literal(SPAN, self.ast.str(&name), None)
            } else {
                let optimized_css = optimize_css_block(&css_str);
                if !optimized_css.is_empty() {
                    let css = ExtractStyleValue::Css(ExtractCss {
                        css: optimized_css,
                        file: self.filename.clone(),
                    });
                    self.styles.insert(css);
                }
                self.ast.expression_identifier(SPAN, self.ast.str(""))
            }
        }

        // Replace JSXElement with JSXFragment when dynamic `as` prop produced an empty name
        if let Some(children) = self.pending_fragment_children.take() {
            *it = self.ast.expression_jsx_fragment(
                SPAN,
                self.ast.jsx_opening_fragment(SPAN),
                children,
                self.ast.jsx_closing_fragment(SPAN),
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
            && !it.arguments.is_empty()
        {
            let expr = it.arguments[0].to_expression();
            let element_kind = if let Expression::Identifier(ident) = expr {
                self.imports.get(ident.name.as_str()).cloned()
            } else if let Expression::StaticMemberExpression(member) = expr
                && let Expression::Identifier(ident) = &member.object
                && self.import_object.as_deref() == Some(ident.name.as_str())
            {
                ExportVariableKind::try_from(member.property.name.to_string()).ok()
            } else {
                None
            };
            if let Some(kind) = element_kind
                && it.arguments.len() > 1
            {
                // Pre-scan: detect conditional styleOrder before extract_style_from_expression
                // consumes the property (which only handles static values)
                let parsed_style_order =
                    if let Expression::ObjectExpression(obj) = it.arguments[1].to_expression() {
                        obj.properties.iter().find_map(|prop| {
                            if let ObjectPropertyKind::ObjectProperty(p) = prop
                                && let Some(name) = get_string_by_property_key(&p.key)
                                && name == "styleOrder"
                            {
                                Some(expression_to_style_order(&p.value, self.ast.allocator))
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                    .unwrap_or(ParsedStyleOrder::None);

                let mut tag =
                    self.ast
                        .expression_string_literal(SPAN, self.ast.str(kind.to_tag()), None);
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
                        .map(|s| s.clone_in(self.ast.allocator))
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
                                condition.clone_in(self.ast.allocator),
                                &mut alt_props_styles,
                                *alternate,
                            )),
                        );
                        self.styles.extend(tailwind_styles);
                    }

                    // Collect styles from both branches for CSS output
                    props_styles.iter().rev().for_each(|style| {
                        self.styles.extend(style.extract().into_iter().map(|mut s| {
                            if let Some(order) = consequent {
                                s.set_style_order(*order);
                            }
                            s
                        }))
                    });
                    alt_props_styles.iter().rev().for_each(|style| {
                        self.styles.extend(style.extract().into_iter().map(|mut s| {
                            if let Some(order) = alternate {
                                s.set_style_order(*order);
                            }
                            s
                        }))
                    });
                } else {
                    let style_order = parsed_style_order.as_static();
                    props_styles.iter().rev().for_each(|style| {
                        self.styles.extend(style.extract().into_iter().map(|mut s| {
                            style_order.into_iter().for_each(|order| {
                                s.set_style_order(order);
                            });
                            s
                        }))
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
        } else if it.source.value == self.package
            && let Some(specifiers) = &mut it.specifiers
        {
            for i in (0..specifiers.len()).rev() {
                match &specifiers[i] {
                    ImportSpecifier(import) => {
                        let imported_str = import.imported.to_string();
                        if let Ok(kind) = ExportVariableKind::from_str(&imported_str) {
                            self.imports.insert(import.local.to_string(), kind);
                            specifiers.remove(i);
                        } else if let Ok(kind) = UtilType::from_str(&imported_str) {
                            self.util_imports
                                .insert(import.local.to_string(), Rc::new(kind));
                            specifiers.remove(i);
                        } else if imported_str == "styled" {
                            self.styled_import = Some(import.local.to_string());
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
                            let func = match imported.as_str() {
                                "create" => Some(StylexFunction::Create),
                                "props" => Some(StylexFunction::Props),
                                "keyframes" => Some(StylexFunction::Keyframes),
                                "firstThatWorks" => Some(StylexFunction::FirstThatWorks),
                                "defineVars" => Some(StylexFunction::DefineVars),
                                "createTheme" => Some(StylexFunction::CreateTheme),
                                "include" => Some(StylexFunction::Include),
                                _ => None,
                            };
                            if let Some(func) = func {
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
    fn visit_jsx_element(&mut self, elem: &mut JSXElement<'a>) {
        walk_jsx_element(self, elem);
        // after run to convert css literal
        let component_name = &elem.opening_element.name.to_string();
        if let Some(kind) = self.imports.get(component_name) {
            let attrs = &mut elem.opening_element.attributes;
            let mut tag_name = self
                .ast
                .expression_string_literal(SPAN, kind.to_tag(), None);
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
                    let property_name = name.name.to_string();
                    for name in disassemble_property(&property_name) {
                        if !duplicate_set.contains(&name) {
                            duplicate_set.insert(name.clone());
                            if property_name == "styleOrder" {
                                parsed_style_order = jsx_expression_to_style_order(
                                    attr.value.as_ref().unwrap(),
                                    self.ast.allocator,
                                );
                            } else if property_name == "props" {
                                if let Some(value) = attr.value.as_ref()
                                    && let JSXAttributeValue::ExpressionContainer(expr) = value
                                    && let Some(expression) = expr.expression.as_expression()
                                {
                                    props = Some(expression.clone_in(self.ast.allocator));
                                }
                            } else if property_name == "styleVars" {
                                if let Some(value) = attr.value.as_ref()
                                    && let JSXAttributeValue::ExpressionContainer(expr) = value
                                    && let Some(expression) = expr.expression.as_expression()
                                {
                                    style_vars = Some(expression.clone_in(self.ast.allocator));
                                }
                            } else if let Some(at) = &mut attr.value {
                                let ExtractResult { styles, tag, .. } =
                                    extract_style_from_jsx(&self.ast, &name, at);
                                props_styles.extend(styles.into_iter().rev());
                                tag_name = tag.unwrap_or(tag_name);
                            }
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
                    if !styles.is_empty() {
                        props_styles.extend(styles.into_iter().rev());
                    } else {
                        attrs.insert(i, attr);
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
                    .map(|s| s.clone_in(self.ast.allocator))
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
                        condition.clone_in(self.ast.allocator),
                        &mut alt_props_styles,
                        *alternate,
                    )),
                );
                self.styles.extend(tailwind_styles_con);

                // Collect styles from both branches for CSS output
                props_styles.iter().rev().for_each(|style| {
                    self.styles.extend(style.extract().into_iter().map(|mut s| {
                        if let Some(order) = consequent {
                            s.set_style_order(*order);
                        }
                        s
                    }))
                });
                alt_props_styles.iter().rev().for_each(|style| {
                    self.styles.extend(style.extract().into_iter().map(|mut s| {
                        if let Some(order) = alternate {
                            s.set_style_order(*order);
                        }
                        s
                    }))
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
                    .iter()
                    .rev()
                    .for_each(|style| self.styles.extend(style.extract()));
            }

            if let Some(tag) = if let Expression::StringLiteral(str) = tag_name {
                Some(str.value.as_str())
            } else if let Expression::TemplateLiteral(literal) = tag_name {
                Some(literal.quasis[0].value.raw.as_str())
            } else {
                let mut v = AsVisitor::new(self.ast.allocator, elem.clone_in(self.ast.allocator));
                let mut el = self.ast.expression_statement(SPAN, tag_name);
                v.visit_expression_statement(&mut el);
                let mut children = oxc_allocator::Vec::new_in(self.ast.allocator);
                children.push(JSXChild::ExpressionContainer(
                    self.ast.alloc_jsx_expression_container(
                        SPAN,
                        el.expression.clone_in(self.ast.allocator).into(),
                    ),
                ));
                self.pending_fragment_children = Some(children);
                None
            } {
                let ident = self
                    .ast
                    .jsx_element_name_identifier(SPAN, self.ast.str(tag));

                elem.opening_element.name = ident.clone_in(self.ast.allocator);
                if let Some(el) = &mut elem.closing_element {
                    el.name = ident
                }
            }
        }
    }
}
