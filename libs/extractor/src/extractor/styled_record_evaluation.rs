use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::{GetSpan, SPAN};

use crate::utils::is_same_expression;

/// Ordered reads shared by the class and variable expressions of one render.
pub(super) struct Evaluations<'b, 'a> {
    pub builder: &'b AstBuilder<'a>,
    source: String,
    pub bindings: Vec<(String, Expression<'a>)>,
    next: usize,
}

impl<'b, 'a> Evaluations<'b, 'a> {
    pub const fn new(builder: &'b AstBuilder<'a>, source: String) -> Self {
        Self {
            builder,
            source,
            bindings: Vec::new(),
            next: 0,
        }
    }

    pub fn capture(&mut self, value: &Expression<'a>, guard: &Expression<'a>) -> Expression<'a> {
        let name = self.fresh_name();
        let value = self.choose(guard, value, &self.undefined());
        self.bindings.push((name.clone(), value));
        Expression::new_identifier(
            SPAN,
            oxc_ast::ast::Ident::from_in(name.as_str(), self.builder.allocator()),
            self.builder,
        )
    }

    pub fn fresh_name(&mut self) -> String {
        loop {
            let name = format!("__devupRecord{}", self.next);
            self.next += 1;
            if !self.source.contains(&name) {
                return name;
            }
        }
    }

    pub fn boolean(&self, value: bool) -> Expression<'a> {
        Expression::new_boolean_literal(SPAN, value, self.builder)
    }

    pub fn undefined(&self) -> Expression<'a> {
        Expression::new_identifier(SPAN, "undefined", self.builder)
    }

    pub fn choose(
        &self,
        test: &Expression<'a>,
        yes: &Expression<'a>,
        no: &Expression<'a>,
    ) -> Expression<'a> {
        let clone = |value: &Expression<'a>| value.clone_in(self.builder.allocator());
        match (test, yes, no) {
            (Expression::BooleanLiteral(value), _, _) => clone(if value.value { yes } else { no }),
            (_, Expression::BooleanLiteral(yes), Expression::BooleanLiteral(no))
                if yes.value == no.value =>
            {
                self.boolean(yes.value)
            }
            _ if is_same_expression(yes, no) => clone(yes),
            _ => Expression::new_conditional_expression(
                test.span(),
                clone(test),
                clone(yes),
                clone(no),
                self.builder,
            ),
        }
    }
}

fn visit_style_value_expression<'a>(
    builder: &AstBuilder<'a>,
    style: &mut crate::ExtractStyleValue,
    visit: &mut impl FnMut(&mut Expression<'a>),
) {
    use crate::ExtractStyleValue;
    match style {
        ExtractStyleValue::Dynamic(value) => {
            let mut expression = Expression::new_identifier(
                SPAN,
                oxc_ast::ast::Ident::from_in(value.identifier(), builder.allocator()),
                builder,
            );
            visit(&mut expression);
            *value.identifier_mut() = crate::utils::readable_code(&expression);
        }
        // Metadata values hold no expression to visit. LayerOrder is listed for exhaustiveness only:
        // public styled records cannot reach it because final collection synthesizes layer order.
        ExtractStyleValue::Static(_)
        | ExtractStyleValue::Typography(_)
        | ExtractStyleValue::Css(_)
        | ExtractStyleValue::Import(_)
        | ExtractStyleValue::FontFace(_)
        | ExtractStyleValue::LayerOrder(_)
        | ExtractStyleValue::Keyframes(_) => {}
    }
}

pub(super) fn visit_style_expressions<'a>(
    builder: &AstBuilder<'a>,
    style: &mut crate::ExtractStyleProp<'a>,
    visit: &mut impl FnMut(&mut Expression<'a>),
) {
    use crate::ExtractStyleProp;
    match style {
        ExtractStyleProp::Static(value) => visit_style_value_expression(builder, value, visit),
        ExtractStyleProp::Unreadable { .. } => {}
        ExtractStyleProp::StaticArray(styles) => {
            for style in styles {
                visit_style_expressions(builder, style, visit);
            }
        }
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            visit(condition);
            for style in consequent.iter_mut().chain(alternate.iter_mut()) {
                visit_style_expressions(builder, style, visit);
            }
        }
        ExtractStyleProp::Enum { condition, map } => {
            visit(condition);
            for style in map.values_mut().flatten() {
                visit_style_expressions(builder, style, visit);
            }
        }
        ExtractStyleProp::Expression { expression, styles } => {
            visit(expression);
            for style in styles {
                visit_style_value_expression(builder, style, visit);
            }
        }
        ExtractStyleProp::MemberExpression { expression, map } => {
            visit(expression);
            for style in map.values_mut() {
                visit_style_expressions(builder, style, visit);
            }
        }
    }
}

pub(super) fn bind_render<'a>(
    builder: &AstBuilder<'a>,
    bindings: Vec<(String, Expression<'a>)>,
    body: Expression<'a>,
) -> Expression<'a> {
    use oxc_allocator::Vec as ArenaVec;
    use oxc_ast::ast::{
        ArrowFunctionBody, BindingPattern, FormalParameterKind, FormalParameters, Statement,
        VariableDeclaration, VariableDeclarationKind, VariableDeclarator,
    };
    if bindings.is_empty() {
        return body;
    }
    // Sibling declarations preserve dependent read order without one scope per capture.
    let mut statements = ArenaVec::with_capacity_in(bindings.len() + 1, builder);
    for (name, value) in bindings {
        let name = oxc_ast::ast::Ident::from_in(name.as_str(), builder.allocator());
        // Sequence evaluation prevents declaration-name inference for anonymous definitions.
        let value = Expression::new_sequence_expression(
            SPAN,
            ArenaVec::from_array_in(
                [
                    Expression::new_numeric_literal(
                        SPAN,
                        0.0,
                        None,
                        oxc_syntax::number::NumberBase::Decimal,
                        builder,
                    ),
                    value,
                ],
                builder,
            ),
            builder,
        );
        let declaration = VariableDeclarator::new(
            SPAN,
            BindingPattern::new_binding_identifier(SPAN, name, builder),
            None,
            Some(value),
            false,
            builder,
        );
        statements.push(Statement::VariableDeclaration(VariableDeclaration::boxed(
            SPAN,
            VariableDeclarationKind::Const,
            ArenaVec::from_array_in([declaration], builder),
            false,
            builder,
        )));
    }
    statements.push(Statement::new_return_statement(SPAN, Some(body), builder));
    let arrow = Expression::new_arrow_function_expression(
        SPAN,
        false,
        None,
        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            ArenaVec::new_in(builder),
            None,
            builder,
        ),
        None,
        ArrowFunctionBody::new_function_body(SPAN, ArenaVec::new_in(builder), statements, builder),
        builder,
    );
    Expression::new_call_expression(
        SPAN,
        Expression::new_parenthesized_expression(SPAN, arrow, builder),
        None,
        ArenaVec::new_in(builder),
        false,
        builder,
    )
}

#[cfg(test)]
#[path = "styled_record_evaluation_visitor_tests.rs"]
mod visitor_tests;

#[cfg(test)]
#[path = "styled_record_evaluation_depth_tests.rs"]
mod depth_tests;

#[cfg(test)]
#[path = "styled_record_evaluation_tests.rs"]
mod tests;
