use crate::composition::normalised::NormalisedProp;
use crate::extract_style::style_property::StyleProperty;
use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{
    ComputedMemberExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str,
    StringLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;

use super::roots::products::Generated;
use super::roots::{ClassConditional, ClassPayload};

mod conditional;
mod merge;
mod normalised;

pub(crate) use conditional::{class_lookup_root, normalize_conditional};
pub(crate) use merge::merge_roots;
pub(crate) use normalised::emit_normalised;

#[derive(Clone, Copy)]
pub(crate) struct ClassPlacement<'s> {
    pub(crate) order: Option<u8>,
    pub(crate) filename: Option<&'s str>,
}

pub(crate) fn gen_normalised<'a, E: ClassConditional<'a>>(
    ast: &AstBuilder<'a>,
    props: &mut [NormalisedProp<'a, E>],
    placement: ClassPlacement<'_>,
) -> Option<E> {
    crate::extractor::extract_style_from_expression::yield_normalised_typography(props);
    merge_roots(
        ast,
        props
            .iter_mut()
            .filter_map(|prop| emit_normalised(ast, prop, placement))
            .rev(),
    )
}

pub(crate) fn emit_prop<'a, E: ClassPayload<'a>>(
    ast: &AstBuilder<'a>,
    prop: &mut ExtractStyleProp<'a, E>,
    placement: ClassPlacement<'_>,
) -> Option<Generated<'a, E>> {
    match prop {
        ExtractStyleProp::Enum { map, condition } => {
            let properties = map.iter_mut().filter_map(|(key, values)| {
                merge_roots(
                    ast,
                    values
                        .iter_mut()
                        .filter_map(|value| emit_prop(ast, value, placement)),
                )
                .map(|class| class_property(ast, key, class))
            });
            let object = Expression::new_object_expression(
                SPAN,
                oxc_allocator::Vec::from_iter_in(properties, ast),
                ast,
            );
            let member = ComputedMemberExpression::boxed(
                SPAN,
                object,
                condition.clone_in(ast.allocator()),
                false,
                ast,
            );
            Some(Generated::Lookup(class_lookup_root(ast, member)))
        }
        ExtractStyleProp::Static(style) => {
            static_class(ast, style, placement).map(Generated::String)
        }
        ExtractStyleProp::StaticArray(props) => merge_roots(
            ast,
            props
                .iter_mut()
                .filter_map(|prop| emit_prop(ast, prop, placement)),
        ),
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            let sides = emit_branches(ast, (consequent, alternate), placement);
            Some(normalize_conditional(ast, condition, sides).into_generated())
        }
        ExtractStyleProp::Expression { expression, .. } => Some(Generated::Supplied(
            expression.clone_payload(ast.allocator()),
        )),
        ExtractStyleProp::Unreadable { .. } | ExtractStyleProp::Diagnostic { .. } => None,
        ExtractStyleProp::MemberExpression { map, expression } => {
            let properties = map.iter_mut().filter_map(|(key, value)| {
                emit_prop(ast, value.as_mut(), placement)
                    .map(|class| class_property(ast, key, class))
            });
            let member = ComputedMemberExpression::boxed(
                SPAN,
                Expression::new_object_expression(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(properties, ast),
                    ast,
                ),
                expression.clone_in(ast.allocator()),
                false,
                ast,
            );
            Some(Generated::Lookup(class_lookup_root(ast, member)))
        }
    }
}

fn static_class<'a>(
    ast: &AstBuilder<'a>,
    style: &mut ExtractStyleValue,
    placement: ClassPlacement<'_>,
) -> Option<oxc_allocator::Box<'a, StringLiteral<'a>>> {
    if let Some(order) = placement.order {
        style.set_style_order(order);
    }
    style.extract(placement.filename).map(|style| {
        let value = Str::from_in(
            &match style {
                StyleProperty::ClassName(class) => class,
                StyleProperty::Variable { class_name, .. } => class_name,
            },
            ast.allocator(),
        );
        StringLiteral::boxed(SPAN, value, None, ast)
    })
}

fn class_property<'a, E: ClassPayload<'a>>(
    ast: &AstBuilder<'a>,
    key: &str,
    class: Generated<'a, E>,
) -> ObjectPropertyKind<'a> {
    ObjectPropertyKind::new_object_property(
        SPAN,
        PropertyKind::Init,
        PropertyKey::StringLiteral(StringLiteral::boxed(
            SPAN,
            Str::from_in(key, ast.allocator()),
            None,
            ast,
        )),
        class.into_expression(),
        false,
        false,
        false,
        ast,
    )
}

type BranchSides<'consequent, 'alternate, 'a, E> = (
    &'consequent mut Option<Box<ExtractStyleProp<'a, E>>>,
    &'alternate mut Option<Box<ExtractStyleProp<'a, E>>>,
);

fn emit_branches<'a, E: ClassPayload<'a>>(
    ast: &AstBuilder<'a>,
    sides: BranchSides<'_, '_, 'a, E>,
    placement: ClassPlacement<'_>,
) -> (Generated<'a, E>, Generated<'a, E>) {
    let consequent = sides
        .0
        .as_mut()
        .and_then(|prop| emit_prop(ast, prop.as_mut(), placement))
        .unwrap_or_else(|| Generated::String(StringLiteral::boxed(SPAN, "", None, ast)));
    let alternate = sides
        .1
        .as_mut()
        .and_then(|prop| emit_prop(ast, prop.as_mut(), placement))
        .unwrap_or_else(|| Generated::String(StringLiteral::boxed(SPAN, "", None, ast)));
    (consequent, alternate)
}
