//! Ordinary-property callback discovery and admission-view rewriting.
use crate::{
    css_utils::{reads_unmapped_theme, theme_var_reference, unmapped_theme_error},
    utils::{unwrap_syntax_only, unwrap_syntax_only_mut, wrap_direct_call},
};
use oxc_ast::{
    ast::{Expression, ObjectExpression, ObjectPropertyKind},
    builder::AstBuilder,
};

pub(super) fn contains(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        match unwrap_syntax_only(&property.value) {
            Expression::ObjectExpression(inner) => contains(inner),
            function @ (Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)) => theme_var_reference(function).is_none(),
            _ => false,
        }
    })
}

pub(super) fn rewrite<'a>(
    builder: &AstBuilder<'a>,
    object: &mut ObjectExpression<'a>,
) -> Result<(), (u32, String)> {
    for property in &mut object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        match unwrap_syntax_only_mut(&mut property.value) {
            Expression::ObjectExpression(inner) => rewrite(builder, inner)?,
            function @ (Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_))
                if theme_var_reference(function).is_none() =>
            {
                if reads_unmapped_theme(function) {
                    return Err(unmapped_theme_error("styled", function));
                }
                *function =
                    wrap_direct_call(builder, function, &[super::identifier(builder, "rest")]);
            }
            _ => {}
        }
    }
    Ok(())
}
