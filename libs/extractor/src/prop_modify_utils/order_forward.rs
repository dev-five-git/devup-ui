use oxc_ast::ast::{
    BindingPattern, BindingProperty, BindingRestElement, Expression, FormalParameter,
    FormalParameterKind, FormalParameters, ObjectPropertyKind, PropertyKey, PropertyKind,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;

/// Copy a styling-origin spread without forwarding its order metadata.
pub(crate) fn without_order_props<'a>(
    ast: &AstBuilder<'a>,
    value: Expression<'a>,
) -> Expression<'a> {
    let properties = ["styleOrder", "style-order"]
        .into_iter()
        .enumerate()
        .map(|(index, key)| {
            BindingProperty::new(
                SPAN,
                PropertyKey::new_string_literal(SPAN, key, None, ast),
                BindingPattern::new_binding_identifier(
                    SPAN,
                    if index == 0 {
                        "__devupOrder"
                    } else {
                        "__devupKebabOrder"
                    },
                    ast,
                ),
                false,
                false,
                ast,
            )
        });
    let pattern = BindingPattern::new_object_pattern(
        SPAN,
        oxc_allocator::Vec::from_iter_in(properties, ast),
        Some(BindingRestElement::boxed(
            SPAN,
            BindingPattern::new_binding_identifier(SPAN, "__devupProps", ast),
            ast,
        )),
        ast,
    );
    let parameter = FormalParameter::new(
        SPAN,
        oxc_allocator::Vec::new_in(ast),
        pattern,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        None::<oxc_allocator::Box<Expression<'a>>>,
        false,
        None,
        false,
        false,
        ast,
    );
    let arrow = Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            oxc_allocator::Vec::from_array_in([parameter], ast),
            None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
            ast,
        ),
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        Expression::new_identifier(SPAN, "__devupProps", ast).into(),
        ast,
    );
    let prototype = ObjectPropertyKind::new_object_property(
        SPAN,
        PropertyKind::Init,
        PropertyKey::new_static_identifier(SPAN, "__proto__", ast),
        Expression::new_null_literal(SPAN, ast),
        false,
        false,
        false,
        ast,
    );
    let copy = Expression::new_object_expression(
        SPAN,
        oxc_allocator::Vec::from_array_in(
            [
                prototype,
                ObjectPropertyKind::new_spread_property(SPAN, value, ast),
            ],
            ast,
        ),
        ast,
    );
    crate::utils::wrap_direct_call(
        ast,
        &Expression::new_parenthesized_expression(SPAN, arrow, ast),
        &[copy],
    )
}
