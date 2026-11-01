use super::super::{
    ATTRS_CONTEXT, ATTRS_VALUE, Base, Execution, STYLED_BASE, StyledJsx, StyledRender,
    call_with_values, create_styled_component, identifier, member, merge_attrs_value, named_arrow,
    resolve_attrs, spread_objects, with_property, wrap_direct_call,
};
use super::expression;
use crate::utils::readable_code as code;
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, Ident},
    builder::AstBuilder,
};
use oxc_span::SPAN;

pub(super) fn stage_element<'a>(
    ast: &AstBuilder<'a>,
    element: (Expression<'a>, Expression<'a>),
    helper: &mut Option<StyledJsx>,
) -> Expression<'a> {
    let (target, props) = element;
    match helper {
        Some(helper) => {
            let raw = code(&props);
            let rendered = helper.render(
                ast,
                target,
                (
                    expression(ast, &format!("{STYLED_BASE}.className")),
                    expression(ast, &format!("{STYLED_BASE}.style")),
                    expression(ast, &format!("{STYLED_BASE}.as")),
                ),
                identifier(ast, STYLED_BASE),
            );
            expression(ast, &format!("({STYLED_BASE}=>{})({raw})", code(&rendered)))
        }
        None => call_with_values(
            ast,
            vec![(STYLED_BASE.to_string(), target)],
            expression(ast, &format!("<DevupStyled {{...{}}}/>", code(&props))),
        ),
    }
}

pub(super) fn stage_attrs<'a>(
    ast: &AstBuilder<'a>,
    attrs: &[Expression<'a>],
    stage: Attrs<'_>,
) -> Expression<'a> {
    let mut merged = expression(ast, stage.input);
    for attr in attrs {
        if stage.plain {
            let spread = spread_objects(ast, merged, attr.clone_in(ast.allocator()));
            let style = Expression::new_conditional_expression(
                SPAN,
                expression(ast, stage.hybrid),
                spread_objects(
                    ast,
                    Expression::new_object_expression(SPAN, oxc_allocator::Vec::new_in(ast), ast),
                    member(ast, ATTRS_CONTEXT, "style"),
                ),
                member(ast, ATTRS_CONTEXT, "style"),
                ast,
            );
            let merged_style = with_property(ast, identifier(ast, ATTRS_CONTEXT), "style", style);
            merged = wrap_direct_call(
                ast,
                &named_arrow(ast, ATTRS_CONTEXT, merged_style),
                &[spread],
            );
            continue;
        }
        let resolved = spread_objects(
            ast,
            Expression::new_object_expression(SPAN, oxc_allocator::Vec::new_in(ast), ast),
            resolve_attrs(ast, attr.clone_in(ast.allocator())),
        );
        let merge = call_with_values(
            ast,
            vec![(ATTRS_VALUE.to_string(), resolved)],
            merge_attrs_value(ast),
        );
        merged = wrap_direct_call(ast, &named_arrow(ast, ATTRS_CONTEXT, merge), &[merged]);
    }
    merged
}

pub(super) struct Attrs<'s> {
    pub input: &'s str,
    pub hybrid: &'s str,
    pub plain: bool,
}

pub(super) fn terminal<'a>(
    ast: &AstBuilder<'a>,
    target: (&Base<'a>, &str),
    helper: &mut Option<StyledJsx>,
) -> Expression<'a> {
    let base = Base {
        name: target.0.name.clone(),
        target: Some(Expression::new_identifier(
            SPAN,
            Ident::from_in(target.1, ast.allocator()),
            ast,
        )),
        styles: None,
        bound: None,
    };
    create_styled_component(
        ast,
        &base,
        StyledRender {
            class_name: &None,
            style_vars: &None,
            bindings: Vec::new(),
            withheld: &[],
            capability: None,
            execution: &Execution::Replay,
            markers: None,
        },
        helper,
    )
}
