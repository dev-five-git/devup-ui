use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKind},
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use rustc_hash::FxHashMap;

#[path = "local_shape_captures_w27.rs"]
mod captures;
pub(super) use captures::collect as collect_captures;

/// Convert a captured length once, leaving zero and nonnumeric values untouched.
pub(super) fn number_length<'a>(builder: &AstBuilder<'a>, leaf: Expression<'a>) -> Expression<'a> {
    use oxc_ast::ast::{BindingPattern, FormalParameter, FormalParameterKind, FormalParameters};
    use oxc_span::SPAN;
    use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
    let value = || Expression::new_identifier(SPAN, "__devupValue", builder);
    let condition = Expression::new_logical_expression(
        SPAN,
        Expression::new_binary_expression(
            SPAN,
            Expression::new_unary_expression(SPAN, UnaryOperator::Typeof, value(), builder),
            BinaryOperator::StrictEquality,
            Expression::new_string_literal(SPAN, "number", None, builder),
            builder,
        ),
        LogicalOperator::And,
        Expression::new_binary_expression(
            SPAN,
            value(),
            BinaryOperator::StrictInequality,
            Expression::new_numeric_literal(
                SPAN,
                0.0,
                None,
                oxc_syntax::number::NumberBase::Decimal,
                builder,
            ),
            builder,
        ),
        builder,
    );
    let body = Expression::new_conditional_expression(
        SPAN,
        condition,
        Expression::new_binary_expression(
            SPAN,
            value(),
            BinaryOperator::Addition,
            Expression::new_string_literal(SPAN, "px", None, builder),
            builder,
        ),
        value(),
        builder,
    );
    let mut params = oxc_allocator::Vec::new_in(builder);
    params.push(FormalParameter::new(
        SPAN,
        oxc_allocator::Vec::new_in(builder),
        BindingPattern::new_binding_identifier(SPAN, "__devupValue", builder),
        None,
        None,
        false,
        None,
        false,
        false,
        builder,
    ));
    let arrow = Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            params,
            None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
            builder,
        ),
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body.into(),
        builder,
    );
    crate::utils::wrap_direct_call(builder, &arrow, &[leaf])
}

pub(super) fn has_tdz(
    expression: &Expression<'_>,
    scoping: &Scoping,
    ends: &FxHashMap<u32, u32>,
) -> bool {
    struct TemporalReads<'s> {
        scoping: &'s Scoping,
        ends: &'s FxHashMap<u32, u32>,
        invalid: bool,
    }
    impl<'a> Visit<'a> for TemporalReads<'_> {
        fn visit_arrow_function_expression(
            &mut self,
            _: &oxc_ast::ast::ArrowFunctionExpression<'a>,
        ) {
        }

        fn visit_function(
            &mut self,
            _: &oxc_ast::ast::Function<'a>,
            _: oxc_syntax::scope::ScopeFlags,
        ) {
        }

        fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
            match crate::utils::unwrap_syntax_only(&call.callee) {
                Expression::ArrowFunctionExpression(arrow) => {
                    walk::walk_arrow_function_expression(self, arrow);
                }
                Expression::FunctionExpression(function) => {
                    walk::walk_function(self, function, oxc_syntax::scope::ScopeFlags::empty());
                }
                _ => self.visit_expression(&call.callee),
            }
            for argument in &call.arguments {
                self.visit_argument(argument);
            }
        }
        fn visit_identifier_reference(
            &mut self,
            identifier: &oxc_ast::ast::IdentifierReference<'a>,
        ) {
            if let Some(reference) = identifier.reference_id.get()
                && let Some(symbol) = self.scoping.get_reference(reference).symbol_id()
                && self
                    .ends
                    .get(&self.scoping.symbol_span(symbol).start)
                    .is_some_and(|end| identifier.span.start < *end)
            {
                self.invalid = true;
            }
            walk::walk_identifier_reference(self, identifier);
        }
    }
    let mut reads = TemporalReads {
        scoping,
        ends,
        invalid: false,
    };
    reads.visit_expression(expression);
    reads.invalid
}

pub(super) fn prepare<'a>(
    builder: &AstBuilder<'a>,
    expression: &Expression<'a>,
    fold: &mut impl FnMut(&Expression<'a>) -> Option<Expression<'a>>,
) -> Option<Expression<'a>> {
    let expression = crate::utils::unwrap_syntax_only(expression);
    match expression {
        Expression::ObjectExpression(object) => {
            let mut object = object.clone_in_with_semantic_ids(builder.allocator());
            for property in &mut object.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return None;
                };
                if property.computed
                    || property.method
                    || property.kind != PropertyKind::Init
                    || property
                        .key
                        .static_name()
                        .is_none_or(|key| key == "__proto__")
                {
                    return None;
                }
                property.value = prepare(builder, &property.value, fold)?;
            }
            Some(Expression::ObjectExpression(object))
        }
        Expression::ArrayExpression(array) => {
            let mut array = array.clone_in_with_semantic_ids(builder.allocator());
            let elements =
                std::mem::replace(&mut array.elements, oxc_allocator::Vec::new_in(builder));
            for element in elements {
                match element {
                    ArrayExpressionElement::Elision(elision) => array
                        .elements
                        .push(ArrayExpressionElement::Elision(elision)),
                    ArrayExpressionElement::SpreadElement(spread) => {
                        let Expression::ArrayExpression(mut inner) =
                            prepare(builder, &spread.argument, fold)?
                        else {
                            return None;
                        };
                        array.elements.extend(inner.elements.drain(..));
                    }
                    element => array
                        .elements
                        .push(prepare(builder, element.as_expression()?, fold)?.into()),
                }
            }
            Some(Expression::ArrayExpression(array))
        }
        expression => Some(
            fold(expression)
                .unwrap_or_else(|| expression.clone_in_with_semantic_ids(builder.allocator())),
        ),
    }
}

/// Read captured runtime leaves through the original object, so moving a shape
/// into styles never evaluates its initializer expressions a second time.
pub(super) fn expand<'a>(
    builder: &AstBuilder<'a>,
    shape: &Expression<'a>,
    root: &Expression<'a>,
) -> Expression<'a> {
    let mut result = shape.clone_in_with_semantic_ids(builder.allocator());
    match &mut result {
        Expression::ObjectExpression(object) => {
            for property in &mut object.properties {
                if let ObjectPropertyKind::ObjectProperty(property) = property
                    && let Some(name) = property.key.static_name()
                {
                    let key = Expression::new_string_literal(
                        property.key.span(),
                        builder.allocator().alloc_str(&name),
                        None,
                        builder,
                    );
                    let member = Expression::new_computed_member_expression(
                        property.value.span(),
                        root.clone_in_with_semantic_ids(builder.allocator()),
                        key,
                        false,
                        builder,
                    );
                    property.value = if css::is_special_property::is_special_property(&name)
                        || matches!(name.as_ref(), "as" | "props" | "styleVars")
                    {
                        member
                    } else {
                        expand(builder, &property.value, &member)
                    };
                    property.shorthand = false;
                }
            }
            result
        }
        Expression::ArrayExpression(array) => {
            for (index, element) in array.elements.iter_mut().enumerate() {
                if let Some(value) = element.as_expression() {
                    let key = Expression::new_string_literal(
                        value.span(),
                        builder.allocator().alloc_str(&index.to_string()),
                        None,
                        builder,
                    );
                    let member = Expression::new_computed_member_expression(
                        value.span(),
                        root.clone_in_with_semantic_ids(builder.allocator()),
                        key,
                        false,
                        builder,
                    );
                    *element = expand(builder, value, &member).into();
                }
            }
            result
        }
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => result,
        _ => {
            if super::written_out(&result).is_some() {
                result
            } else {
                root.clone_in_with_semantic_ids(builder.allocator())
            }
        }
    }
}
