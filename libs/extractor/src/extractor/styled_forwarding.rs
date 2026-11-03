use super::{StyledDefinition, withheld};
use crate::styled_reads::{Forward, passes};
use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        BindingPattern, BindingProperty, BindingRestElement, Expression, FormalParameter,
        FormalParameterKind, FormalParameterRest, FormalParameters, PropertyKey, Str,
        StringLiteral, TSTypeAnnotation, TSTypeParameterDeclaration,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;
use std::borrow::Cow;

#[derive(Clone, Copy)]
pub(crate) enum Site {
    LocalJsx,
    Renderer,
}

pub(crate) fn denied<'n>(
    definition: &StyledDefinition<'_>,
    names: impl Iterator<Item = Cow<'n, str>>,
    site: Site,
) -> Vec<String> {
    // Renderer transport preserves unknown ordinary props; local JSX input validation is stricter.
    let withheld = withheld(
        &definition.reads,
        definition.native(),
        definition.forward.as_ref(),
    );
    let default = Forward::Always(true);
    names
        .filter(|name| {
            let name = name.as_ref();
            !definition.takes(name)
                && match site {
                    Site::LocalJsx => true,
                    Site::Renderer => {
                        definition.forward.is_some()
                            || withheld.iter().any(|withheld| withheld == name)
                            || !passes(name, definition.native(), Some(&default))
                    }
                }
        })
        .map(Cow::into_owned)
        .collect()
}

/// Filter a forwarded copy using the same destructuring carrier as styled renders.
pub(crate) fn without_props<'a>(
    builder: &AstBuilder<'a>,
    props: Expression<'a>,
    withheld: &[String],
) -> Expression<'a> {
    if withheld.is_empty() {
        return props;
    }
    let properties = withheld.iter().enumerate().map(|(index, name)| {
        BindingProperty::new(
            SPAN,
            PropertyKey::StringLiteral(StringLiteral::boxed(
                SPAN,
                Str::from_in(name.as_str(), builder.allocator()),
                None,
                builder,
            )),
            BindingPattern::new_binding_identifier(
                SPAN,
                Str::from_in(format!("__devupOmit{index}").as_str(), builder.allocator()),
                builder,
            ),
            false,
            false,
            builder,
        )
    });
    let pattern = BindingPattern::new_object_pattern(
        SPAN,
        oxc_allocator::Vec::from_iter_in(properties, builder),
        Some(BindingRestElement::boxed(
            SPAN,
            BindingPattern::new_binding_identifier(SPAN, "__devupDom", builder),
            builder,
        )),
        builder,
    );
    let parameter = FormalParameter::new(
        SPAN,
        oxc_allocator::Vec::new_in(builder),
        pattern,
        None::<oxc_allocator::Box<TSTypeAnnotation<'a>>>,
        None::<oxc_allocator::Box<Expression<'a>>>,
        false,
        None,
        false,
        false,
        builder,
    );
    let arrow = Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<TSTypeParameterDeclaration<'a>>>,
        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            oxc_allocator::Vec::from_iter_in([parameter], builder),
            None::<oxc_allocator::Box<FormalParameterRest<'a>>>,
            builder,
        ),
        None::<oxc_allocator::Box<TSTypeAnnotation<'a>>>,
        Expression::new_identifier(SPAN, "__devupDom", builder).into(),
        builder,
    );
    crate::utils::wrap_direct_call(
        builder,
        &Expression::new_parenthesized_expression(SPAN, arrow, builder),
        &[props],
    )
}
