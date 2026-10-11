use super::super::roots::ClassConditional;
use super::{ClassPlacement, emit_branches, normalize_conditional, static_class};
use crate::composition::normalised::NormalisedProp;
use oxc_allocator::GetAllocator;
use oxc_ast::builder::AstBuilder;

pub(crate) fn emit_normalised<'a, E: ClassConditional<'a>>(
    ast: &AstBuilder<'a>,
    prop: &mut NormalisedProp<'a, E>,
    placement: ClassPlacement<'_>,
) -> Option<E> {
    match prop {
        NormalisedProp::Static(style) => static_class(ast, style, placement).map(E::from_string),
        NormalisedProp::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            let sides = emit_branches(ast, (consequent, alternate), placement);
            Some(normalize_conditional(ast, condition, sides).into_payload())
        }
        NormalisedProp::Supplied { expression, .. } => {
            Some(expression.clone_payload(ast.allocator()))
        }
        NormalisedProp::Diagnostic { .. } | NormalisedProp::Unreadable { .. } => None,
    }
}
