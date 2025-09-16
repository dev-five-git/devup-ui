use crate::utils::get_string_by_literal_expression;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, JSXElement};
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::walk_expression;

use oxc_ast::AstBuilder;
use oxc_span::SPAN;

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
