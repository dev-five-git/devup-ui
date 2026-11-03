use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        Expression, IdentifierName, JSXAttributeItem, JSXAttributeName, JSXAttributeValue,
        ObjectPropertyKind, Program, PropertyKey, PropertyKind, Str,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::operator::LogicalOperator;
use rustc_hash::FxHashSet;

use super::styled_capability::StyledCapability;
use super::styled_jsx::StyledJsx;

pub(crate) struct StyledContext<'s> {
    pub filename: &'s str,
    pub jsx: &'s mut Option<StyledJsx>,
    pub capability: Option<&'s StyledCapability>,
}

pub(crate) enum CssPropLowering {
    Retained,
    Inline(String),
    Slot(StyledCssSlot),
}

#[derive(Clone)]
pub(crate) struct StyledCssSlot(String);

impl StyledCssSlot {
    /// Reserve a private prop before extraction can remove authored names or keys.
    pub(crate) fn reserve(program: &Program<'_>) -> Self {
        Self::reserve_named(program, "__devupCss")
    }

    pub(crate) fn reserve_named(program: &Program<'_>, prefix: &str) -> Self {
        let mut names = FxHashSet::default();
        crate::utils::ReservedNames::CssSlot(&mut names).reserve(program);
        let mut suffix = 0;
        loop {
            let name = format!("{prefix}{suffix}");
            if !names.contains(&name) {
                return Self(name);
            }
            suffix += 1;
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.0
    }

    pub(crate) fn classes<'a>(
        &self,
        ast: &AstBuilder<'a>,
        own: Option<&Expression<'a>>,
    ) -> Expression<'a> {
        Expression::new_logical_expression(
            SPAN,
            Expression::new_static_member_expression(
                SPAN,
                Expression::new_identifier(SPAN, "rest", ast),
                IdentifierName::new(SPAN, Str::from_in(self.name(), ast.allocator()), ast),
                false,
                ast,
            ),
            LogicalOperator::Coalesce,
            own.map_or_else(
                || Expression::new_string_literal(SPAN, "", None, ast),
                |own| own.clone_in(ast.allocator()),
            ),
            ast,
        )
    }

    pub(crate) fn attribute<'a>(
        &self,
        ast: &AstBuilder<'a>,
        classes: Option<Expression<'a>>,
    ) -> JSXAttributeItem<'a> {
        JSXAttributeItem::new_attribute(
            SPAN,
            JSXAttributeName::new_identifier(SPAN, Str::from_in(self.name(), ast.allocator()), ast),
            Some(JSXAttributeValue::new_expression_container(
                SPAN,
                classes
                    .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast))
                    .into(),
                ast,
            )),
            ast,
        )
    }

    pub(crate) fn property<'a>(
        &self,
        ast: &AstBuilder<'a>,
        classes: Option<Expression<'a>>,
    ) -> ObjectPropertyKind<'a> {
        ObjectPropertyKind::new_object_property(
            SPAN,
            PropertyKind::Init,
            PropertyKey::new_static_identifier(
                SPAN,
                Str::from_in(self.name(), ast.allocator()),
                ast,
            ),
            classes.unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast)),
            false,
            false,
            false,
            ast,
        )
    }
}
