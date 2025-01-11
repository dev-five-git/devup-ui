use crate::component::ExportVariableKind;
use crate::prop_extract_utils::extract_style_prop;
use crate::prop_modify_utils::modify_props;
use crate::{ExtractCss, ExtractStyleProp, ExtractStyleValue, StyleProperty};
use oxc_allocator::Allocator;
use oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier;
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Expression, ImportDeclaration, ImportOrExportKind, JSXElement, Program, Statement,
    TaggedTemplateExpression, TemplateElementValue, WithClause,
};
use oxc_ast::visit::walk_mut::{
    walk_import_declaration, walk_jsx_element, walk_program, walk_tagged_template_expression,
};
use oxc_ast::{AstBuilder, VisitMut};
use oxc_span::SPAN;
use std::collections::HashMap;

pub struct DevupVisitor<'a> {
    pub ast: AstBuilder<'a>,
    imports: HashMap<String, ExportVariableKind>,
    package: String,
    css_file: String,
    pub styles: Vec<ExtractStyleValue>,
}

impl<'a> DevupVisitor<'a> {
    pub fn new(allocator: &'a Allocator, package: &str, css_file: &str) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            imports: HashMap::new(),
            package: package.to_string(),
            css_file: css_file.to_string(),
            styles: vec![],
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
            let mut props_styles = vec![];

            // extract ExtractStyleProp and remain style and class name, just extract
            for i in (0..attrs.len()).rev() {
                let attr = &attrs[i];
                if let Attribute(attr) = attr {
                    if let Identifier(name) = &attr.name {
                        let name = name.to_string();
                        // ignore style and className
                        if name == "style" || name == "className" {
                            continue;
                        }
                        if let Some(value) = &attr.value {
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
        }
    }
    fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
        if it.source.value != self.package {
            walk_import_declaration(self, it);
            return;
        }
        if let Some(specifiers) = &it.specifiers {
            for specifier in specifiers {
                if let ImportSpecifier(import) = specifier {
                    self.imports.insert(
                        import.local.to_string(),
                        ExportVariableKind::from(import.imported.to_string()),
                    );
                }
            }
        }
    }
}
