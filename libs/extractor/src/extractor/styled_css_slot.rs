use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        BindingIdentifier, ComputedMemberExpression, Expression, IdentifierName,
        IdentifierReference, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXIdentifier,
        ObjectPropertyKind, Program, PropertyKey, PropertyKind, Str,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{
    Visit,
    walk::{walk_computed_member_expression, walk_property_key},
};
use oxc_span::SPAN;
use oxc_syntax::operator::LogicalOperator;
use rustc_hash::FxHashSet;

use super::styled_capability::StyledCapability;
use super::styled_jsx::StyledJsx;
use crate::utils::unwrap_syntax_only;

pub(crate) struct StyledContext<'s> {
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

#[derive(Default)]
struct Names(FxHashSet<String>);

fn static_key_name<'a>(expression: &Expression<'a>) -> Option<Str<'a>> {
    match unwrap_syntax_only(expression) {
        Expression::StringLiteral(literal) => Some(literal.value),
        Expression::TemplateLiteral(template) => template.single_quasi(),
        Expression::TSTypeAssertion(assertion) => static_key_name(&assertion.expression),
        _ => None,
    }
}

impl<'a> Visit<'a> for Names {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.0.insert(it.name.to_string());
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.0.insert(it.name.to_string());
    }

    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        self.0.insert(it.name.to_string());
    }

    fn visit_jsx_identifier(&mut self, it: &JSXIdentifier<'a>) {
        self.0.insert(it.name.to_string());
    }

    fn visit_property_key(&mut self, it: &PropertyKey<'a>) {
        if let Some(name) = it
            .as_expression()
            .and_then(static_key_name)
            .map(|name| name.to_string())
            .or_else(|| it.static_name().map(std::borrow::Cow::into_owned))
        {
            self.0.insert(name);
        }
        walk_property_key(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if let Some(name) = static_key_name(&it.expression).or_else(|| it.static_property_name()) {
            self.0.insert(name.to_string());
        }
        walk_computed_member_expression(self, it);
    }
}

impl StyledCssSlot {
    /// Reserve a private prop before extraction can remove authored names or keys.
    pub(crate) fn reserve(program: &Program<'_>) -> Self {
        Self::reserve_named(program, "__devupCss")
    }

    pub(crate) fn reserve_named(program: &Program<'_>, prefix: &str) -> Self {
        let mut names = Names::default();
        names.visit_program(program);
        let mut suffix = 0;
        loop {
            let name = format!("{prefix}{suffix}");
            if !names.0.contains(&name) {
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
