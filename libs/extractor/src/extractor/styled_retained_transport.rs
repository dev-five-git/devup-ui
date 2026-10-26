use super::super::{RENDERED, StyledCapability, identifier, with_property, without_props};
use super::Renderer;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::SPAN;

pub(in super::super) struct Transport<'b, 'a> {
    pub capability: Option<&'b StyledCapability>,
    pub replacement: Option<&'b Expression<'a>>,
    pub forwarded: Expression<'a>,
}

pub(in super::super) fn transport<'a>(
    ast: &AstBuilder<'a>,
    renderer: Option<&Renderer<'a>>,
    transport: Transport<'_, 'a>,
) -> (Expression<'a>, Expression<'a>, Expression<'a>) {
    match renderer {
        None => (
            identifier(ast, RENDERED),
            transport.forwarded,
            identifier(ast, "forwardedAs"),
        ),
        Some(renderer) => {
            let mut controls = Vec::new();
            if let Some(capability) = transport.capability {
                capability.omit(&mut controls);
            }
            let raw = with_property(
                ast,
                without_props(ast, &controls),
                "forwardedAs",
                identifier(ast, "forwardedAs"),
            );
            let raw = Expression::new_object_expression(
                SPAN,
                oxc_allocator::Vec::from_array_in(
                    [
                        ObjectPropertyKind::new_spread_property(SPAN, raw, ast),
                        renderer.capability.classes.property(
                            ast,
                            transport
                                .replacement
                                .map(|value| value.clone_in(ast.allocator())),
                        ),
                        renderer
                            .capability
                            .forwarded
                            .property(ast, Some(transport.forwarded)),
                    ],
                    ast,
                ),
                ast,
            );
            (
                renderer.target.clone_in(ast.allocator()),
                raw,
                identifier(ast, RENDERED),
            )
        }
    }
}
