use crate::ImportAlias;
use crate::as_visit::AsVisitor;
use crate::component::ExportVariableKind;
use crate::css_utils::{css_to_style_literal, keyframes_to_keyframes_style, optimize_css_block};
use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::extract_css::ExtractCss;
use crate::extract_style::extract_keyframes::ExtractKeyframes;
use crate::extractor::KeyframesExtractResult;
use crate::extractor::extract_keyframes_from_expression::extract_keyframes_from_expression;
use crate::extractor::{
    ExtractResult, GlobalExtractResult,
    extract_global_style_from_expression::extract_global_style_from_expression,
    extract_style_from_expression::extract_style_from_expression,
    extract_style_from_jsx::extract_style_from_jsx,
    extract_style_from_styled::extract_style_from_styled,
};
use crate::gen_class_name::gen_class_names;
use crate::prop_modify_utils::{modify_prop_object, modify_props};
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
    JSXAttributeItem, JSXAttributeValue, JSXChild, JSXElement, Program, Statement,
    VariableDeclarator, WithClause,
};
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::{
    walk_call_expression, walk_expression, walk_import_declaration, walk_jsx_element, walk_program,
    walk_variable_declarator, walk_variable_declarators,
};
use strum::IntoEnumIterator;

use crate::utils::{get_string_by_property_key, jsx_expression_to_number};
use oxc_ast::AstBuilder;
use oxc_span::SPAN;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub struct DevupVisitor<'a> {
    pub ast: AstBuilder<'a>,
    filename: String,
    imports: HashMap<String, ExportVariableKind>,
    import_object: Option<String>,
    jsx_imports: HashMap<String, String>,
    util_imports: HashMap<String, Rc<UtilType>>,
    jsx_object: Option<String>,
    package: String,
    split_filename: Option<String>,
    pub css_files: Vec<String>,
    pub styles: HashSet<ExtractStyleValue>,
    styled_import: Option<String>,
    /// Import aliases for redirecting imports from other CSS-in-JS libraries
    import_aliases: HashMap<String, ImportAlias>,
}

impl<'a> DevupVisitor<'a> {
    pub fn new(
        allocator: &'a Allocator,
        filename: &str,
        package: &str,
        css_files: Vec<String>,
        split_filename: Option<String>,
        import_aliases: HashMap<String, ImportAlias>,
    ) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            filename: filename.to_string(),
            imports: HashMap::new(),
            jsx_imports: HashMap::new(),
            package: package.to_string(),
            css_files,
            styles: HashSet::new(),
            import_object: None,
            jsx_object: None,
            util_imports: HashMap::new(),
            split_filename,
            styled_import: None,
            import_aliases,
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
                            self.ast.string_literal(SPAN, self.ast.atom(css_file), None),
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
                && decl.specifiers.iter().all(|s| s.is_empty())
            {
                // Remove empty imports from main package and aliased packages
                let source = decl.source.value.as_str();
                if source == self.package || self.import_aliases.contains_key(source) {
                    it.body.remove(i);
                }
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
                                .expression_string_literal(SPAN, self.ast.atom(""), None)
                        }
                        UtilType::GlobalCss => {
                            self.ast.expression_identifier(SPAN, self.ast.atom(""))
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
                        );

                        if styles.is_empty() {
                            self.ast
                                .expression_string_literal(SPAN, self.ast.atom(""), None)
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
                                    .expression_string_literal(SPAN, self.ast.atom(""), None)
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
                            .expression_string_literal(SPAN, self.ast.atom(&name), None)
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
                        self.ast.expression_identifier(SPAN, self.ast.atom(""))
                    }
                }
            }
        } else if let Expression::TaggedTemplateExpression(tag) = it
            && let Expression::Identifier(ident) = &tag.tag
            && let Some(css_type) = self.util_imports.get(ident.name.as_str())
        {
            let css_str = tag
                .quasi
                .quasis
                .iter()
                .map(|quasi| quasi.value.raw.to_string())
                .collect::<String>();
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
                        .expression_string_literal(SPAN, self.ast.atom(""), None)
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
                    .expression_string_literal(SPAN, self.ast.atom(&name), None)
            } else {
                let optimized_css = optimize_css_block(&css_str);
                if !optimized_css.is_empty() {
                    let css = ExtractStyleValue::Css(ExtractCss {
                        css: optimized_css,
                        file: self.filename.clone(),
                    });
                    self.styles.insert(css);
                }
                self.ast.expression_identifier(SPAN, self.ast.atom(""))
            }
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
                && self.import_object == Some(ident.name.to_string())
            {
                ExportVariableKind::try_from(member.property.name.to_string()).ok()
            } else {
                None
            };
            if let Some(kind) = element_kind
                && it.arguments.len() > 1
            {
                let mut tag =
                    self.ast
                        .expression_string_literal(SPAN, self.ast.atom(kind.to_tag()), None);
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
                props_styles.iter().rev().for_each(|style| {
                    self.styles.extend(style.extract().into_iter().map(|mut s| {
                        style_order.into_iter().for_each(|order| {
                            s.set_style_order(order);
                        });
                        s
                    }))
                });

                if let Expression::ObjectExpression(obj) = it.arguments[1].to_expression_mut() {
                    modify_prop_object(
                        &self.ast,
                        &mut obj.properties,
                        &mut props_styles,
                        style_order,
                        style_vars,
                        props,
                        self.split_filename.as_deref(),
                    );
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
    }
    fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
        let source_value = it.source.value.as_str();

        // Check for import aliases (e.g., @emotion/styled, styled-components, @vanilla-extract/css)
        // Instead of modifying AST (which has lifetime issues), we just track the imports internally
        if let Some(alias) = self.import_aliases.get(source_value).cloned() {
            if let Some(specifiers) = &mut it.specifiers {
                match alias {
                    ImportAlias::DefaultToNamed(named_export) => {
                        // Transform default import to named import
                        // e.g., `import styled from '@emotion/styled'` → register as `styled` named import
                        for i in (0..specifiers.len()).rev() {
                            if let ImportDeclarationSpecifier::ImportDefaultSpecifier(
                                default_spec,
                            ) = &specifiers[i]
                            {
                                let local_name = default_spec.local.to_string();
                                // Register as styled import (for styled-components/emotion compatibility)
                                if named_export == "styled" {
                                    self.styled_import = Some(local_name);
                                }
                                specifiers.remove(i);
                            }
                        }
                    }
                    ImportAlias::NamedToNamed => {
                        // Named imports stay as named imports (1:1 mapping)
                        // e.g., `import { style } from '@vanilla-extract/css'` → treat as `import { style } from '@devup-ui/react'`
                        // Process like regular package imports
                        for i in (0..specifiers.len()).rev() {
                            if let ImportSpecifier(import) = &specifiers[i] {
                                if let Ok(kind) =
                                    ExportVariableKind::try_from(import.imported.to_string())
                                {
                                    self.imports.insert(import.local.to_string(), kind);
                                    specifiers.remove(i);
                                } else if let Ok(kind) =
                                    UtilType::try_from(import.imported.to_string())
                                {
                                    self.util_imports
                                        .insert(import.local.to_string(), Rc::new(kind));
                                    specifiers.remove(i);
                                } else if import.imported.to_string() == "styled" {
                                    self.styled_import = Some(import.local.to_string());
                                    specifiers.remove(i);
                                }
                            }
                        }
                    }
                }
            }
        } else if source_value != self.package && source_value == "react/jsx-runtime" {
            if let Some(specifiers) = &it.specifiers {
                for specifier in specifiers {
                    if let ImportSpecifier(import) = specifier {
                        self.jsx_imports
                            .insert(import.local.to_string(), import.imported.to_string());
                    }
                }
            }
        } else if source_value == self.package {
            if let Some(specifiers) = &mut it.specifiers {
                for i in (0..specifiers.len()).rev() {
                    match &specifiers[i] {
                        ImportSpecifier(import) => {
                            if let Ok(kind) =
                                ExportVariableKind::try_from(import.imported.to_string())
                            {
                                self.imports.insert(import.local.to_string(), kind);
                                specifiers.remove(i);
                            } else if let Ok(kind) = UtilType::try_from(import.imported.to_string())
                            {
                                self.util_imports
                                    .insert(import.local.to_string(), Rc::new(kind));
                                specifiers.remove(i);
                            } else if import.imported.to_string() == "styled" {
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
            let mut duplicate_set = HashSet::new();
            let mut style_order = None;
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
                                style_order =
                                    jsx_expression_to_number(attr.value.as_ref().unwrap())
                                        .map(|n| n as u8);
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

            modify_props(
                &self.ast,
                attrs,
                &mut props_styles,
                style_order,
                style_vars,
                props,
                self.split_filename.as_deref(),
            );

            props_styles
                .iter()
                .rev()
                .for_each(|style| self.styles.extend(style.extract()));
            // modify!!

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
                *elem = self.ast.jsx_element(
                    SPAN,
                    self.ast.alloc_jsx_opening_element(
                        SPAN,
                        self.ast
                            .jsx_element_name_identifier(SPAN, self.ast.atom("")),
                        Some(self.ast.alloc_ts_type_parameter_instantiation(
                            SPAN,
                            oxc_allocator::Vec::new_in(self.ast.allocator),
                        )),
                        oxc_allocator::Vec::new_in(self.ast.allocator),
                    ),
                    children,
                    Some(
                        self.ast.alloc_jsx_closing_element(
                            SPAN,
                            self.ast
                                .jsx_element_name_identifier(SPAN, self.ast.atom("")),
                        ),
                    ),
                );
                None
            } {
                let ident = self
                    .ast
                    .jsx_element_name_identifier(SPAN, self.ast.atom(tag));

                elem.opening_element.name = ident.clone_in(self.ast.allocator);
                if let Some(el) = &mut elem.closing_element {
                    el.name = ident
                }
            }
        }
    }
}
