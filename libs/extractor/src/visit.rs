use crate::component::ExportVariableKind;
use crate::object_prop_extract_utils::{extract_from_style_value, extract_object_from_jsx_attr};
use crate::prop_extract_utils::{extract_style_prop, extract_style_prop_from_express};
use crate::prop_modify_utils::{modify_prop_object, modify_props};
use crate::utils::is_special_property;
use crate::{ExtractCss, ExtractStyleProp, ExtractStyleValue, StyleProperty};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier;
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Argument, BindingPatternKind, CallExpression, Expression, ImportDeclaration,
    ImportOrExportKind, JSXAttributeValue, JSXElement, JSXElementName, Program, Statement,
    TaggedTemplateExpression, TemplateElementValue, VariableDeclarator, WithClause,
};
use oxc_ast::visit::walk_mut::{
    walk_call_expression, walk_import_declaration, walk_jsx_element, walk_program,
    walk_tagged_template_expression, walk_variable_declarator,
};
use oxc_ast::{AstBuilder, VisitMut};
use oxc_span::SPAN;
use std::collections::HashMap;

pub struct DevupVisitor<'a> {
    pub ast: AstBuilder<'a>,
    imports: HashMap<String, ExportVariableKind>,
    import_object: Option<String>,
    jsx_imports: HashMap<String, String>,
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
                    let mut tag = kind.to_tag().unwrap_or("div");
                    if it.arguments.len() > 1 {
                        if let Argument::ObjectExpression(ref mut object) = &mut it.arguments[1] {
                            let mut collected_props_styles = vec![];
                            for i in (0..object.properties.len()).rev() {
                                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(prop) =
                                    &object.properties[i]
                                {
                                    if let oxc_ast::ast::PropertyKey::StaticIdentifier(ident) =
                                        &prop.key
                                    {
                                        let name = ident.name.to_string();
                                        // ignore special attributes
                                        if is_special_property(&name) {
                                            continue;
                                        }
                                        if name == "typography" {
                                            if let Expression::StringLiteral(ident) = &prop.value {
                                                collected_props_styles.push(
                                                    ExtractStyleProp::Static(
                                                        ExtractStyleValue::Typography(
                                                            ident.value.to_string(),
                                                        ),
                                                    ),
                                                );
                                            }
                                            object.properties.remove(i);
                                            continue;
                                        }
                                        if name == "as" {
                                            if let Expression::StringLiteral(ident) = &prop.value {
                                                tag = ident.value.as_str();
                                            }
                                            object.properties.remove(i);
                                            continue;
                                        }
                                        if name.starts_with("_") {
                                            let media = name.trim_start_matches('_');
                                            if let Some(props_styles) = extract_from_style_value(
                                                &self.ast,
                                                &prop.value,
                                                Some(media),
                                            ) {
                                                for style in props_styles.into_iter().rev() {
                                                    collected_props_styles.push(style);
                                                }
                                                object.properties.remove(i);
                                            }
                                        } else {
                                            let prop_styles = extract_style_prop_from_express(
                                                &self.ast,
                                                &name,
                                                &prop.value,
                                                0,
                                                None,
                                            );
                                            if let Some(prop_styles) = prop_styles {
                                                collected_props_styles.push(prop_styles);
                                                object.properties.remove(i);
                                            }
                                        }
                                    }
                                }
                            }
                            for ex in kind.extract().into_iter().rev() {
                                collected_props_styles.push(ExtractStyleProp::Static(ex));
                            }
                            for ex in collected_props_styles.iter().rev() {
                                self.styles.append(&mut ex.extract());
                            }

                            modify_prop_object(
                                &self.ast,
                                &mut object.properties,
                                collected_props_styles,
                            );
                        }
                    }

                    it.arguments[0] = Argument::StringLiteral(self.ast.alloc_string_literal(
                        SPAN,
                        self.ast.atom(tag),
                        None,
                    ));
                }
            }
        }

        walk_call_expression(self, it);
    }
    fn visit_tagged_template_expression(&mut self, it: &mut TaggedTemplateExpression<'a>) {
        if let Expression::Identifier(ident) = &it.tag {
            if ident.name != "css" {
                walk_tagged_template_expression(self, it);
                return;
            }

            let css_str = it
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
                let mut ve = oxc_allocator::Vec::new_in(self.ast.allocator);
                ve.push(self.ast.template_element(
                    SPAN,
                    false,
                    TemplateElementValue {
                        cooked: None,
                        raw: self.ast.atom(cls.as_str()),
                    },
                ));
                it.quasi.quasis = ve;
            }
            self.styles.push(css);
            return;
        }
        walk_tagged_template_expression(self, it);
    }
    fn visit_jsx_element(&mut self, elem: &mut JSXElement<'a>) {
        walk_jsx_element(self, elem);
        // after run to convert css literal

        let opening_element = &mut elem.opening_element;
        let component_name = &opening_element.name.to_string();
        if let Some(kind) = self.imports.get(component_name) {
            let attrs = &mut opening_element.attributes;
            let mut tag_name = kind.to_tag().unwrap_or("div");
            let mut props_styles = vec![];

            // extract ExtractStyleProp and remain style and class name, just extract
            for i in (0..attrs.len()).rev() {
                let attr = &attrs[i];
                if let Attribute(attr) = attr {
                    if let Identifier(name) = &attr.name {
                        let name = name.to_string();

                        // ignore special attributes
                        if is_special_property(&name) {
                            continue;
                        }
                        if name == "typography" {
                            if let Some(JSXAttributeValue::StringLiteral(ident)) = &attr.value {
                                props_styles.push(ExtractStyleProp::Static(
                                    ExtractStyleValue::Typography(ident.value.to_string()),
                                ));
                            }
                            attrs.remove(i);
                            continue;
                        }
                        if name == "as" {
                            if let Some(JSXAttributeValue::StringLiteral(ident)) = &attr.value {
                                tag_name = ident.value.as_str()
                            }
                            attrs.remove(i);
                            continue;
                        }

                        if let Some(value) = &attr.value {
                            // media query
                            if name.starts_with("_") {
                                if let Some(prop_styles) = &mut extract_object_from_jsx_attr(
                                    &self.ast,
                                    value,
                                    Some(name.trim_start_matches('_')),
                                ) {
                                    props_styles.append(prop_styles);
                                    attrs.remove(i);
                                }
                                continue;
                            }
                            let prop_styles = extract_style_prop(&self.ast, name, value);
                            if let Some(prop_styles) = prop_styles {
                                props_styles.push(prop_styles);
                                attrs.remove(i);
                            }
                        }
                    }
                }
            }

            for ex in kind.extract().into_iter().rev() {
                props_styles.push(ExtractStyleProp::Static(ex));
            }
            for style in props_styles.iter().rev() {
                self.styles.append(&mut style.extract());
            }
            // modify!!
            modify_props(&self.ast, attrs, props_styles);

            // change tag name
            let ident = JSXElementName::Identifier(self.ast.alloc_jsx_identifier(SPAN, tag_name));
            opening_element.name = ident.clone_in(self.ast.allocator);
            if let Some(el) = &mut elem.closing_element {
                el.name = ident;
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
                                    if let oxc_ast::ast::PropertyKey::StaticIdentifier(ident) =
                                        &prop.key
                                    {
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
                                    if let oxc_ast::ast::PropertyKey::StaticIdentifier(ident) =
                                        &prop.key
                                    {
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
        if let Some(specifiers) = &mut it.specifiers {
            for i in (0..specifiers.len()).rev() {
                if let ImportSpecifier(import) = &specifiers[i] {
                    if let Ok(kind) = ExportVariableKind::try_from(import.imported.to_string()) {
                        self.imports.insert(import.local.to_string(), kind);

                        // remove specifier
                        specifiers.remove(i);
                    }
                }
            }
        }
    }
}
