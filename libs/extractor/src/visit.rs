use crate::component::ExportVariableKind;
use crate::extract_style::ExtractCss;
use crate::gen_class_name::gen_class_names;
use crate::prop_modify_utils::{modify_prop_object, modify_props};
use crate::style_extractor::{
    ExtractResult, extract_style_from_expression, extract_style_from_jsx_attr,
};
use crate::{ExtractStyleProp, ExtractStyleValue, StyleProperty};
use css::short_to_long;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::ImportDeclarationSpecifier::{self, ImportSpecifier};
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Argument, BindingPatternKind, CallExpression, Expression, ImportDeclaration,
    ImportOrExportKind, JSXAttributeValue, JSXElement, JSXElementName, Program, PropertyKey,
    Statement, VariableDeclarator, WithClause,
};
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::{
    walk_call_expression, walk_expression, walk_import_declaration, walk_jsx_element, walk_program,
    walk_variable_declarator,
};
use strum::IntoEnumIterator;

use crate::utils::jsx_expression_to_number;
use oxc_ast::AstBuilder;
use oxc_span::SPAN;
use std::collections::{HashMap, HashSet};

pub struct DevupVisitor<'a> {
    pub ast: AstBuilder<'a>,
    imports: HashMap<String, ExportVariableKind>,
    import_object: Option<String>,
    jsx_imports: HashMap<String, String>,
    css_imports: HashMap<String, String>,
    jsx_object: Option<String>,
    package: String,
    css_file: String,
    pub styles: HashSet<ExtractStyleValue>,
}

impl<'a> DevupVisitor<'a> {
    pub fn new(allocator: &'a Allocator, package: &str, css_file: &str) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            imports: HashMap::new(),
            jsx_imports: HashMap::new(),
            package: package.to_string(),
            css_file: css_file.to_string(),
            styles: HashSet::new(),
            import_object: None,
            jsx_object: None,
            css_imports: HashMap::new(),
        }
    }
}

impl<'a> VisitMut<'a> for DevupVisitor<'a> {
    fn visit_program(&mut self, it: &mut Program<'a>) {
        walk_program(self, it);
        if !self.styles.is_empty() {
            it.body.insert(
                0,
                Statement::ImportDeclaration(
                    self.ast.alloc_import_declaration::<Option<WithClause>>(
                        SPAN,
                        None,
                        self.ast
                            .string_literal(SPAN, self.ast.atom(&self.css_file), None),
                        None,
                        None,
                        ImportOrExportKind::Value,
                    ),
                ),
            );
        }

        for i in (0..it.body.len()).rev() {
            if let Statement::ImportDeclaration(decl) = &it.body[i] {
                if decl.source.value == self.package && decl.specifiers.iter().all(|s| s.is_empty())
                {
                    it.body.remove(i);
                }
            }
        }
    }
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_expression(self, it);

        if let Expression::CallExpression(call) = it {
            let css_import_key = if let Expression::Identifier(ident) = &call.callee {
                Some(ident.name.to_string())
            } else if let Expression::StaticMemberExpression(member) = &call.callee
                && let Expression::Identifier(ident) = &member.object
            {
                Some(format!("{}.{}", ident.name, member.property.name))
            } else {
                None
            };

            if let Some(css_import_key) = css_import_key
                && self.css_imports.contains_key(&css_import_key)
            {
                if call.arguments.is_empty() {
                    *it = Expression::StringLiteral(self.ast.alloc_string_literal(
                        SPAN,
                        self.ast.atom(""),
                        None,
                    ));
                } else if call.arguments.len() == 1 {
                    if let ExtractResult::Extract {
                        styles: Some(mut styles),
                        style_order,
                        ..
                    } = extract_style_from_expression(
                        &self.ast,
                        None,
                        call.arguments[0].to_expression_mut(),
                        0,
                        None,
                    ) {
                        // css can not reachable
                        // ExtractResult::ExtractStyleWithChangeTag(styles, _)
                        let class_name = gen_class_names(&self.ast, &mut styles, style_order);

                        self.styles.extend(
                            styles
                                .into_iter()
                                // already set style order
                                .flat_map(|ex| ex.extract()),
                        );
                        if let Some(cls) = class_name {
                            *it = cls;
                        } else {
                            *it = Expression::StringLiteral(self.ast.alloc_string_literal(
                                SPAN,
                                self.ast.atom(""),
                                None,
                            ));
                        }
                    } else {
                        *it = Expression::StringLiteral(self.ast.alloc_string_literal(
                            SPAN,
                            self.ast.atom(""),
                            None,
                        ));
                    }
                }
            }
        } else if let Expression::TaggedTemplateExpression(tag) = it {
            if let Expression::Identifier(ident) = &tag.tag
                && self.css_imports.contains_key(ident.name.as_str())
            {
                let css_str = tag
                    .quasi
                    .quasis
                    .iter()
                    .map(|quasi| quasi.value.raw.as_str())
                    .collect::<Vec<&str>>()
                    .join("");
                let css = ExtractStyleValue::Css(ExtractCss {
                    css: css_str.trim().to_string(),
                });

                if let StyleProperty::ClassName(cls) = css.extract() {
                    *it = Expression::StringLiteral(self.ast.alloc_string_literal(
                        SPAN,
                        self.ast.atom(&cls),
                        None,
                    ));
                }
                self.styles.insert(css);
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
                let mut tag = Expression::StringLiteral(self.ast.alloc_string_literal(
                    SPAN,
                    self.ast.atom(kind.to_tag().unwrap_or("div")),
                    None,
                ));
                let mut props_styles = vec![];
                let mut style_order = None;
                let mut style_vars = None;
                if let ExtractResult::Extract {
                    styles,
                    tag: _tag,
                    style_order: _style_order,
                    style_vars: _style_vars,
                } = extract_style_from_expression(
                    &self.ast,
                    None,
                    it.arguments[1].to_expression_mut(),
                    0,
                    None,
                ) {
                    style_order = _style_order;
                    styles.into_iter().for_each(|mut ex| {
                        props_styles.append(&mut ex);
                    });
                    if let Some(t) = _tag {
                        tag = t;
                    }
                    style_vars = _style_vars;
                }

                for ex in kind.extract().into_iter().rev() {
                    props_styles.push(ExtractStyleProp::Static(ex));
                }

                for style in props_styles.iter().rev() {
                    self.styles.extend(style.extract().into_iter().map(|mut s| {
                        style_order.into_iter().for_each(|order| {
                            s.set_style_order(order);
                        });
                        s
                    }));
                }
                if let Expression::ObjectExpression(obj) = it.arguments[1].to_expression_mut() {
                    modify_prop_object(
                        &self.ast,
                        &mut obj.properties,
                        &mut props_styles,
                        style_order,
                        style_vars,
                    );
                }

                it.arguments[0] = Argument::from(tag);
            }
        }
        walk_call_expression(self, it);
    }
    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        if let Some(Expression::CallExpression(call)) = &it.init {
            if call.arguments.len() != 1 {
                return;
            }
            if let (Expression::Identifier(ident), Argument::StringLiteral(arg)) =
                (&call.callee, &call.arguments[0])
                && ident.name == "require"
            {
                if arg.value == "react/jsx-runtime" {
                    if let BindingPatternKind::BindingIdentifier(ident) = &it.id.kind {
                        self.jsx_object = Some(ident.name.to_string());
                    } else if let BindingPatternKind::ObjectPattern(object) = &it.id.kind {
                        for prop in &object.properties {
                            if let PropertyKey::StaticIdentifier(ident) = &prop.key
                                && let Some(k) = prop
                                    .value
                                    .get_binding_identifier()
                                    .map(|id| id.name.to_string())
                            {
                                self.jsx_imports.insert(k, ident.name.to_string());
                            }
                        }
                    }
                } else if arg.value == self.package {
                    if let BindingPatternKind::BindingIdentifier(ident) = &it.id.kind {
                        self.import_object = Some(ident.name.to_string());
                    } else if let BindingPatternKind::ObjectPattern(object) = &it.id.kind {
                        for prop in &object.properties {
                            if let PropertyKey::StaticIdentifier(ident) = &prop.key
                                && let Ok(kind) = ExportVariableKind::try_from(
                                    prop.value
                                        .get_binding_identifier()
                                        .map(|id| id.name.to_string())
                                        .unwrap_or("".to_string()),
                                )
                            {
                                self.imports.insert(ident.name.to_string(), kind);
                            }
                        }
                    }
                }
            }
        }

        walk_variable_declarator(self, it);
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
            return;
        }

        if it.source.value == self.package
            && let Some(specifiers) = &mut it.specifiers
        {
            for i in (0..specifiers.len()).rev() {
                match &specifiers[i] {
                    ImportSpecifier(import) => {
                        if let Ok(kind) = ExportVariableKind::try_from(import.imported.to_string())
                        {
                            self.imports.insert(import.local.to_string(), kind);
                            specifiers.remove(i);
                        } else if import.imported.to_string() == "css" {
                            self.css_imports
                                .insert(import.local.to_string(), it.source.value.to_string());
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
                        self.css_imports.insert(
                            format!("{}.{}", import_default_specifier.local, "css"),
                            it.source.value.to_string(),
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
                        self.css_imports.insert(
                            format!("{}.{}", import_namespace_specifier.local, "css"),
                            it.source.value.to_string(),
                        );
                    }
                }
            }
            return;
        }

        walk_import_declaration(self, it);
    }
    fn visit_jsx_element(&mut self, elem: &mut JSXElement<'a>) {
        walk_jsx_element(self, elem);
        // after run to convert css literal

        let opening_element = &mut elem.opening_element;
        let component_name = &opening_element.name.to_string();
        if let Some(kind) = self.imports.get(component_name) {
            let attrs = &mut opening_element.attributes;
            let mut tag_name = Expression::StringLiteral(self.ast.alloc_string_literal(
                SPAN,
                kind.to_tag().unwrap_or("div"),
                None,
            ));
            let mut props_styles = vec![];

            // extract ExtractStyleProp and remain style and class name, just extract
            let mut duplicate_set = HashSet::new();
            let mut style_order = None;
            let mut style_vars = None;
            for i in (0..attrs.len()).rev() {
                let mut attr = attrs.remove(i);
                if let Attribute(attr) = &mut attr
                    && let Identifier(name) = &attr.name
                {
                    let name = short_to_long(&name.name);
                    if duplicate_set.contains(&name) {
                        continue;
                    }
                    duplicate_set.insert(name.clone());
                    if name == "styleOrder" {
                        style_order =
                            jsx_expression_to_number(attr.value.as_ref().unwrap()).map(|n| n as u8);
                        continue;
                    }
                    if name == "styleVars" {
                        if let Some(value) = attr.value.as_ref()
                            && let JSXAttributeValue::ExpressionContainer(expr) = value
                        {
                            style_vars =
                                Some(expr.expression.to_expression().clone_in(self.ast.allocator));
                        }
                        continue;
                    }

                    if let Some(at) = &mut attr.value {
                        if let ExtractResult::Extract { styles, tag, .. } =
                            extract_style_from_jsx_attr(&self.ast, &name, at, None)
                        {
                            styles.into_iter().for_each(|mut ex| {
                                ex.reverse();
                                props_styles.append(&mut ex);
                            });
                            if let Some(t) = tag {
                                tag_name = t;
                            }
                            continue;
                        }
                    }
                }
                attrs.insert(i, attr);
            }

            kind.extract()
                .into_iter()
                .rev()
                .for_each(|ex| props_styles.push(ExtractStyleProp::Static(ex)));

            modify_props(&self.ast, attrs, &mut props_styles, style_order, style_vars);

            props_styles
                .iter()
                .rev()
                .for_each(|style| self.styles.extend(style.extract()));
            // modify!!

            if let Some(tag) = match tag_name {
                Expression::StringLiteral(str) => Some(str.value.as_str()),
                Expression::TemplateLiteral(literal) => Some(literal.quasis[0].value.raw.as_str()),
                _ => None,
            } {
                let ident = JSXElementName::Identifier(
                    self.ast.alloc_jsx_identifier(SPAN, self.ast.atom(tag)),
                );

                if let Some(el) = &mut elem.closing_element {
                    el.name = ident.clone_in(self.ast.allocator);
                }
                opening_element.name = ident;
            }
        }
    }
}
