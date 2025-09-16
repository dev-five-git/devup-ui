use crate::component::ExportVariableKind;
use crate::css_utils::{css_to_style, keyframes_to_keyframes_style, optimize_css_block};
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
};
use crate::gen_class_name::gen_class_names;
use crate::prop_modify_utils::{modify_prop_object, modify_props};
use crate::util_type::UtilType;
use crate::utils::get_string_by_literal_expression;
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::disassemble_property;
use css::is_special_property::is_special_property;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::ImportDeclarationSpecifier::{self, ImportSpecifier};
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Argument, BindingPatternKind, CallExpression, Expression, ImportDeclaration,
    ImportOrExportKind, JSXAttributeValue, JSXElement, Program, PropertyKey, Statement,
    VariableDeclarator, WithClause,
};
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::{
    walk_call_expression, walk_expression, walk_expression_statement, walk_import_declaration,
    walk_jsx_element, walk_object_property, walk_program, walk_string_literal,
    walk_variable_declarator, walk_variable_declarators,
};
use strum::IntoEnumIterator;

use oxc_ast::AstBuilder;
use oxc_span::SPAN;
use std::collections::{HashMap, HashSet};

pub struct AsVisitor<'a> {
    ast: AstBuilder<'a>,
    element: JSXElement<'a>,
}

impl<'a> AsVisitor<'a> {
    pub fn new(allocator: &'a Allocator, element: JSXElement<'a>) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            element,
        }
    }
}

fn change_element_name<'a>(ast: &AstBuilder<'a>, element: &mut JSXElement<'a>, element_name: &str) {
    let element_name = ast.jsx_element_name_identifier(SPAN, ast.atom(element_name));
    element.opening_element.name = element_name.clone_in(ast.allocator);
    if let Some(el) = &mut element.closing_element {
        el.name = element_name;
    }
}

impl<'a> VisitMut<'a> for AsVisitor<'a> {
    fn visit_expression(&mut self, it: &mut oxc_ast::ast::Expression<'a>) {
        if let Some(element_name) = get_string_by_literal_expression(it) {
            let mut element = self.element.clone_in(self.ast.allocator);
            change_element_name(&self.ast, &mut element, &element_name);
            *it = Expression::JSXElement(self.ast.alloc(element));
            return;
        } else if let Expression::Identifier(ident) = it {
            let element_name = ident.name.to_string();
            if element_name == "undefined" {
                return;
            }
            let mut element = self.element.clone_in(self.ast.allocator);
            change_element_name(&self.ast, &mut element, &element_name);
            *it = Expression::JSXElement(self.ast.alloc(element));
            return;
        } else if let Expression::ConditionalExpression(conditional) = it {
            self.visit_expression(&mut conditional.consequent);
            self.visit_expression(&mut conditional.alternate);
            return;
        } else if let Expression::ComputedMemberExpression(member) = it {
            self.visit_expression(&mut member.object);
            return;
        }
        walk_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &mut oxc_ast::ast::ObjectProperty<'a>) {
        self.visit_expression(&mut it.value);
    }

    fn visit_spread_element(&mut self, _: &mut oxc_ast::ast::SpreadElement<'a>) {
        // spread be mantained
    }
}
