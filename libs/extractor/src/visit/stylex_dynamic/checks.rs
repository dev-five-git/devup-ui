use crate::visit::{
    Argument, Attribute, CallExpression, DevupVisitor, Expression, FxHashSet, Identifier,
    JSXAttributeItem, JSXElement, Program, StylexNamespaceValue, build_time_error, readable_code,
    unwrap_syntax_only,
};
use oxc_ast_visit::{Visit, walk};

impl<'a> DevupVisitor<'a> {
    pub(in crate::visit) fn reject_nested_stylex_calls(
        &mut self,
        expression: &Expression<'a>,
    ) -> bool {
        if matches!(unwrap_syntax_only(expression), Expression::CallExpression(call) if self.stylex_dynamic_info(&call.callee).is_some())
        {
            return false;
        }
        let mut calls = LocalCalls {
            visitor: self,
            errors: vec![],
        };
        calls.visit_expression(expression);
        let errors = calls.errors;
        let rejected = !errors.is_empty();
        self.errors.extend(errors);
        rejected
    }

    pub(in crate::visit) fn reject_remaining_stylex_calls(&mut self, program: &Program<'a>) {
        let mut calls = LocalCalls {
            visitor: self,
            errors: vec![],
        };
        calls.visit_program(program);
        self.errors.extend(calls.errors);
    }

    pub(in crate::visit) fn reject_stylex_dynamic_jsx_combination(
        &mut self,
        elem: &JSXElement<'a>,
        kept: &FxHashSet<String>,
    ) {
        let attributes = &elem.opening_element.attributes;
        if !attributes.iter().any(|attribute| matches!(attribute,
            Attribute(attribute) if matches!(&attribute.name, Identifier(name) if crate::visit::order::is_style(&name.name, kept)))) {
            return;
        }
        for attribute in attributes {
            if let JSXAttributeItem::SpreadAttribute(spread) = attribute
                && let Expression::CallExpression(call) = unwrap_syntax_only(&spread.argument)
                && self.stylex_class_attribute(&call.callee).is_some()
                && call.arguments.iter().filter_map(Argument::as_expression).any(|expr| {
                    matches!(unwrap_syntax_only(expr), Expression::CallExpression(call) if self.stylex_dynamic_info(&call.callee).is_some())
                })
            {
                self.errors.push((spread.span.start, build_time_error("StyleX JSX spread", &readable_code(&spread.argument), "dynamic call and explicit style combinations cannot be compiled exactly with keyed precedence here; use one styling source on this element")));
            }
        }
    }
}

struct LocalCalls<'v, 'a> {
    visitor: &'v DevupVisitor<'a>,
    errors: Vec<(u32, String)>,
}

impl<'a> Visit<'a> for LocalCalls<'_, 'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let object = match unwrap_syntax_only(&call.callee) {
            Expression::StaticMemberExpression(member) => Some(&member.object),
            Expression::ComputedMemberExpression(member) => Some(&member.object),
            _ => None,
        };
        if object.is_some_and(|object| {
            self.visitor
                .stylex_namespace(object)
                .is_some_and(|namespace| {
                    namespace
                        .values()
                        .any(|value| matches!(value, StylexNamespaceValue::Dynamic(_)))
                })
        }) {
            self.errors.push((call.span.start, build_time_error("stylex.props", &readable_code(&call.callee), "nested/conditional or unresolved namespace calls cannot be compiled exactly here; pass a direct `styles.name(value)` argument with a literal namespace key")));
        }
        walk::walk_call_expression(self, call);
    }
}
