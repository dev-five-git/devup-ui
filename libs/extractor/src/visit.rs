use crate::component::ExportVariableKind;
use crate::extract_style::ExtractCss;
use crate::gen_class_name::gen_class_names;
use crate::prop_modify_utils::{modify_prop_object, modify_props};
use crate::style_extractor::{
    extract_style_from_expression, extract_style_from_jsx_attr, ExtractResult,
};
use crate::{ExtractStyleProp, ExtractStyleValue, StyleProperty};
use css::sort_to_long;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier;
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Argument, BindingPatternKind, CallExpression, Expression, ImportDeclaration,
    ImportOrExportKind, JSXElement, JSXElementName, Program, PropertyKey, Statement,
    VariableDeclarator, WithClause,
};
use oxc_ast::visit::walk_mut::{
    walk_call_expression, walk_expression, walk_import_declaration, walk_jsx_element, walk_program,
    walk_variable_declarator,
};

use oxc_ast::{AstBuilder, VisitMut};
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
    pub styles: Vec<ExtractStyleValue>,
}

impl<'a> DevupVisitor<'a> {
    pub fn new(allocator: &'a Allocator, package: &str, css_file: &str) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            imports: HashMap::new(),
            jsx_imports: HashMap::new(),
            package: package.to_string(),
            css_file: css_file.to_string(),
            styles: vec![],
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
                            .string_literal(SPAN, self.css_file.to_string(), None),
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
            if let Expression::Identifier(ident) = &call.callee {
                if self.css_imports.contains_key(ident.name.as_str()) && call.arguments.len() == 1 {
                    match extract_style_from_expression(
                        &self.ast,
                        None,
                        call.arguments[0].to_expression_mut(),
                        0,
                        None,
                    ) {
                        ExtractResult::ExtractStyle(styles)
                        | ExtractResult::ExtractStyleWithChangeTag(styles, _) => {
                            let class_name = gen_class_names(&self.ast, &styles);
                            let mut styles = styles
                                .into_iter()
                                .flat_map(|ex| ex.extract())
                                .collect::<Vec<_>>();

                            self.styles.append(&mut styles);
                            if let Some(cls) = class_name {
                                *it = cls;
                            } else {
                                *it = Expression::StringLiteral(self.ast.alloc_string_literal(
                                    SPAN,
                                    "".to_string(),
                                    None,
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else if let Expression::TaggedTemplateExpression(tag) = it {
            if let Expression::Identifier(ident) = &tag.tag {
                if self.css_imports.contains_key(ident.name.as_str()) {
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
                        *it = Expression::StringLiteral(
                            self.ast.alloc_string_literal(SPAN, cls, None),
                        );
                    }
                    self.styles.push(css);
                }
            }
        }
    }
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        let jsx = if let Expression::Identifier(ident) = &it.callee {
            self.jsx_imports
                .get(&ident.name.to_string())
                .map(|s| s.to_string())
        } else if let Expression::StaticMemberExpression(member) = &it.callee {
            if let Expression::Identifier(ident) = &member.object {
                if self.jsx_object == Some(ident.name.to_string()) {
                    Some(member.property.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        if let Some(j) = jsx {
            if (j == "jsx" || j == "jsxs") && it.arguments.len() > 0 {
                let expr = it.arguments[0].to_expression();
                let element_kind = if let Expression::Identifier(ident) = expr {
                    self.imports.get(ident.name.as_str()).cloned()
                } else if let Expression::StaticMemberExpression(member) = expr {
                    if let Expression::Identifier(ident) = &member.object {
                        if self.import_object == Some(ident.name.to_string()) {
                            ExportVariableKind::try_from(member.property.name.to_string()).ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(kind) = element_kind {
                    if it.arguments.len() > 1 {
                        let mut tag = Expression::StringLiteral(self.ast.alloc_string_literal(
                            SPAN,
                            kind.to_tag().unwrap_or("div"),
                            None,
                        ));
                        let mut props_styles = vec![];
                        match extract_style_from_expression(
                            &self.ast,
                            None,
                            it.arguments[1].to_expression_mut(),
                            0,
                            None,
                        ) {
                            ExtractResult::ExtractStyle(mut styles) => {
                                props_styles.append(&mut styles);
                            }
                            ExtractResult::ExtractStyleWithChangeTag(mut styles, t) => {
                                tag = t;
                                props_styles.append(&mut styles);
                            }
                            ExtractResult::Maintain => {}
                            ExtractResult::Remove => {}
                            ExtractResult::ChangeTag(t) => {
                                tag = t;
                            }
                        }

                        for ex in kind.extract().into_iter().rev() {
                            props_styles.push(ExtractStyleProp::Static(ex));
                        }

                        for style in props_styles.iter().rev() {
                            self.styles.append(&mut style.extract());
                        }
                        if let Expression::ObjectExpression(obj) =
                            it.arguments[1].to_expression_mut()
                        {
                            modify_prop_object(&self.ast, &mut obj.properties, &props_styles);
                        }

                        it.arguments[0] = Argument::from(tag);
                    }
                }
            }
        }
        walk_call_expression(self, it);
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
            for i in (0..attrs.len()).rev() {
                let mut attr = attrs.remove(i);
                let mut rm = false;
                if let Attribute(ref mut attr) = &mut attr {
                    if let Identifier(name) = &attr.name {
                        let name = sort_to_long(name.name.as_str());
                        if duplicate_set.contains(&name) {
                            continue;
                        }
                        duplicate_set.insert(name.clone());
                        if let Some(at) = &mut attr.value {
                            rm = match extract_style_from_jsx_attr(&self.ast, &name, at, None) {
                                ExtractResult::Maintain => false,
                                ExtractResult::Remove => true,
                                ExtractResult::ExtractStyle(mut styles) => {
                                    styles.reverse();
                                    props_styles.append(&mut styles);
                                    true
                                }
                                ExtractResult::ChangeTag(tag) => {
                                    tag_name = tag;
                                    true
                                }
                                ExtractResult::ExtractStyleWithChangeTag(mut styles, tag) => {
                                    styles.reverse();
                                    props_styles.append(&mut styles);
                                    tag_name = tag;
                                    true
                                }
                            }
                        }
                    }
                }
                if !rm {
                    attrs.insert(i, attr);
                }
            }

            for ex in kind.extract().into_iter().rev() {
                props_styles.push(ExtractStyleProp::Static(ex));
            }
            for style in props_styles.iter().rev() {
                self.styles.append(&mut style.extract());
            }
            // modify!!
            modify_props(&self.ast, attrs, &props_styles);

            match tag_name {
                Expression::StringLiteral(str) => {
                    // change tag name
                    let ident = JSXElementName::Identifier(
                        self.ast.alloc_jsx_identifier(SPAN, str.value.as_str()),
                    );
                    opening_element.name = ident.clone_in(self.ast.allocator);
                    if let Some(el) = &mut elem.closing_element {
                        el.name = ident;
                    }
                }
                Expression::TemplateLiteral(literal) => {
                    let ident = JSXElementName::Identifier(
                        self.ast
                            .alloc_jsx_identifier(SPAN, literal.quasis[0].value.raw.as_str()),
                    );
                    opening_element.name = ident.clone_in(self.ast.allocator);
                    if let Some(el) = &mut elem.closing_element {
                        el.name = ident;
                    }
                }

                _ => {}
            }
        }
    }
    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        if let Some(Expression::CallExpression(call)) = &it.init {
            if call.arguments.len() == 1 {
                if let (Expression::Identifier(ident), Argument::StringLiteral(arg)) =
                    (&call.callee, &call.arguments[0])
                {
                    if ident.name == "require" {
                        if arg.value == "react/jsx-runtime" {
                            if let BindingPatternKind::BindingIdentifier(ident) = &it.id.kind {
                                self.jsx_object = Some(ident.name.to_string());
                            } else if let BindingPatternKind::ObjectPattern(object) = &it.id.kind {
                                for prop in &object.properties {
                                    if let PropertyKey::StaticIdentifier(ident) = &prop.key {
                                        if let Some(k) = prop
                                            .value
                                            .get_binding_identifier()
                                            .map(|id| id.name.to_string())
                                        {
                                            self.jsx_imports.insert(k, ident.name.to_string());
                                        }
                                    }
                                }
                            }
                        } else if arg.value == self.package {
                            if let BindingPatternKind::BindingIdentifier(ident) = &it.id.kind {
                                self.import_object = Some(ident.name.to_string());
                            } else if let BindingPatternKind::ObjectPattern(object) = &it.id.kind {
                                for prop in &object.properties {
                                    if let PropertyKey::StaticIdentifier(ident) = &prop.key {
                                        if let Ok(kind) = ExportVariableKind::try_from(
                                            prop.value
                                                .get_binding_identifier()
                                                .map(|id| id.name.to_string())
                                                .unwrap_or("".to_string()),
                                        ) {
                                            self.imports.insert(ident.name.to_string(), kind);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        walk_variable_declarator(self, it);
    }
    fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
        walk_import_declaration(self, it);
        if it.source.value != self.package && it.source.value == "react/jsx-runtime" {
            if let Some(specifiers) = &it.specifiers {
                for specifier in specifiers {
                    if let ImportSpecifier(import) = specifier {
                        self.jsx_imports
                            .insert(import.local.to_string(), import.imported.to_string());
                    }
                }
            }

            return;
        }

        if it.source.value == self.package {
            if let Some(specifiers) = &mut it.specifiers {
                for i in (0..specifiers.len()).rev() {
                    if let ImportSpecifier(import) = &specifiers[i] {
                        if let Ok(kind) = ExportVariableKind::try_from(import.imported.to_string())
                        {
                            self.imports.insert(import.local.to_string(), kind);

                            // remove specifier
                            specifiers.remove(i);
                        } else if import.imported.to_string() == "css" {
                            self.css_imports
                                .insert(import.local.to_string(), it.source.value.to_string());
                            specifiers.remove(i);
                        }
                    }
                }
            }
        }
    }
}
