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
            final_walk: false,
            types_member_object: false,
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
            final_walk: true,
            types_member_object: false,
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
    final_walk: bool,
    types_member_object: bool,
}

impl<'a> Visit<'a> for LocalCalls<'_, 'a> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        if self.final_walk
            && matches!(
                self.visitor.bindings.stylex_function(expression),
                Some(
                    crate::stylex::StylexFunction::FirstThatWorks
                        | crate::stylex::StylexFunction::Include
                )
            )
            || self.final_walk
                && !self.types_member_object
                && self.visitor.bindings.stylex_function(expression)
                    == Some(crate::stylex::StylexFunction::Types)
        {
            self.errors.push((oxc_span::GetSpan::span(expression).start, build_time_error(
                "StyleX helper", &readable_code(expression),
                "an unconsumed helper function-value escape cannot be compiled exactly; call the helper directly inside its supported enclosing API",
            )));
        }
        walk::walk_expression(self, expression);
    }

    fn visit_static_member_expression(
        &mut self,
        member: &oxc_ast::ast::StaticMemberExpression<'a>,
    ) {
        if self.final_walk
            && crate::stylex::is_types_method(
                &member.object,
                member.property.name.as_str(),
                &|callee| self.visitor.bindings.stylex_function(callee),
            )
        {
            self.errors.push((member.span.start, build_time_error(
                "stylex.types", &format!("{}.{}", readable_code(&member.object), member.property.name),
                "a wrapper-method function-value escape cannot be compiled exactly; call types.* directly inside a consumed style or variable value",
            )));
        }
        if self
            .visitor
            .stylex_namespace(&member.object)
            .is_some_and(|namespace| {
                matches!(
                    namespace.get(member.property.name.as_str()),
                    Some(StylexNamespaceValue::Dynamic(_))
                )
            })
        {
            self.errors.push((member.span.start, build_time_error("stylex.props", &format!("{}.{}", readable_code(&member.object), member.property.name), "an uncalled dynamic namespace or function-value alias cannot be compiled exactly; pass a direct `styles.name(value)` argument to props/attrs instead of escaping the function")));
        }
        self.visit_member_object(&member.object);
        self.visit_identifier_name(&member.property);
    }

    fn visit_computed_member_expression(
        &mut self,
        member: &oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        if self.final_walk
            && self.visitor.bindings.stylex_function(&member.object)
                == Some(crate::stylex::StylexFunction::Types)
        {
            self.errors.push((member.span.start, build_time_error(
                "stylex.types", &format!("{}[{}]", readable_code(&member.object), readable_code(&member.expression)),
                "a wrapper-method function-value escape cannot be compiled exactly; call types.* directly inside a consumed style or variable value",
            )));
        }
        if self
            .visitor
            .stylex_namespace(&member.object)
            .is_some_and(|namespace| {
                match crate::utils::get_string_by_literal_expression(&member.expression) {
                    Some(key) => matches!(
                        namespace.get(key.as_ref()),
                        Some(StylexNamespaceValue::Dynamic(_))
                    ),
                    None => namespace
                        .values()
                        .any(|value| matches!(value, StylexNamespaceValue::Dynamic(_))),
                }
            })
        {
            self.errors.push((member.span.start, build_time_error("stylex.props", &format!("{}[{}]", readable_code(&member.object), readable_code(&member.expression)), "an uncalled dynamic namespace or function-value alias cannot be compiled exactly; pass a direct `styles.name(value)` argument to props/attrs instead of escaping the function")));
        }
        self.visit_member_object(&member.object);
        self.visit_expression(&member.expression);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if self.final_walk
            && let Some(error) =
                crate::stylex::validation::unconsumed_helper_error(call, &|callee| {
                    self.visitor.bindings.stylex_function(callee)
                })
        {
            self.errors.push(error);
        }
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

impl<'a> LocalCalls<'_, 'a> {
    fn visit_member_object(&mut self, object: &Expression<'a>) {
        let previous = self.types_member_object;
        self.types_member_object = self.visitor.bindings.stylex_function(object)
            == Some(crate::stylex::StylexFunction::Types);
        self.visit_expression(object);
        self.types_member_object = previous;
    }
}
