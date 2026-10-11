use super::{
    Allocator, AstBuilder, BinaryOperator, CloneIn, ComputedMemberExpression, DevupVisitor,
    Expression, GetAllocator, KnownStyles, LogicalOperator, SPAN, StaticMemberExpression,
    StringLiteral, Text, UnaryOperator, coalesce_keeps_left, unwrap_syntax_only,
};
use crate::gen_class_name::roots::{ClassConditional, ClassPayload};
use oxc_ast::ast::{ConditionalExpression, ObjectPropertyKind, PropertyKind};

mod choices;
mod logical;
mod side;
mod source;
mod source_roots;

pub(super) use source::{CapturedSource, LocalSource, UncapturedSource};
pub(super) use source_roots::{LocalClass, LocalOutput};

pub(super) enum LocalKnownPart<'a, C> {
    Styles(Vec<KnownStyles<'a>>),
    Conditional {
        test: Expression<'a>,
        consequent: Vec<KnownStyles<'a>>,
        alternate: Vec<KnownStyles<'a>>,
    },
    Class(C),
}

pub(super) enum LocalKnownSide<'a, C> {
    Styles(Vec<KnownStyles<'a>>),
    Class(C),
    Empty,
}

pub(super) struct LocalParts<'v, 's, 'a, S: LocalSource<'a>> {
    visitor: &'v DevupVisitor<'a>,
    source: &'s S,
}

impl<'v, 's, 'a, S: LocalSource<'a>> LocalParts<'v, 's, 'a, S> {
    pub(super) const fn new(visitor: &'v DevupVisitor<'a>, source: &'s S) -> Self {
        Self { visitor, source }
    }

    pub(super) fn known_parts_local(
        &self,
        expression: &Expression<'a>,
        parts: &mut Vec<LocalKnownPart<'a, S::Source>>,
        text: Text,
    ) -> Option<()> {
        let visitor = self.visitor;
        if let Some(array) = visitor.style_values.arrays(expression) {
            for (index, finite) in array.iter().enumerate() {
                let saved = Expression::new_computed_member_expression(
                    SPAN,
                    expression.clone_in_with_semantic_ids(visitor.ast.allocator()),
                    Expression::new_string_literal(
                        SPAN,
                        visitor.ast.allocator().alloc_str(&index.to_string()),
                        None,
                        &visitor.ast,
                    ),
                    false,
                    &visitor.ast,
                );
                parts.push(LocalKnownPart::Styles(vec![KnownStyles::Finite(
                    finite.clone(),
                    saved,
                )]));
            }
            return Some(());
        }
        if let Some(styles) = visitor.style_values.styles(unwrap_syntax_only(expression)) {
            parts.push(LocalKnownPart::Styles(vec![KnownStyles::Known(
                styles.to_vec(),
            )]));
            return Some(());
        }
        if let Some(finite) = visitor.style_values.finite(expression) {
            parts.push(LocalKnownPart::Styles(vec![KnownStyles::Finite(
                finite.clone(),
                expression.clone_in_with_semantic_ids(visitor.ast.allocator()),
            )]));
            return Some(());
        }
        match unwrap_syntax_only(expression) {
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    self.known_parts_local(element.as_expression()?, parts, text)?;
                }
                Some(())
            }
            Expression::LogicalExpression(logical) => self.logical_parts(logical, parts, text),
            Expression::ConditionalExpression(conditional) => {
                let mut yes = Vec::new();
                let mut no = Vec::new();
                self.known_parts_local(&conditional.consequent, &mut yes, text)?;
                self.known_parts_local(&conditional.alternate, &mut no, text)?;
                parts.extend(self.guarded_parts_local(&conditional.test, yes));
                let inverse = Expression::new_unary_expression(
                    SPAN,
                    UnaryOperator::LogicalNot,
                    conditional.test.clone_in(visitor.ast.allocator()),
                    &visitor.ast,
                );
                parts.extend(self.guarded_parts_local(&inverse, no));
                Some(())
            }
            Expression::CallExpression(call)
                if visitor.class_names_text(&call.callee).is_some() =>
            {
                let text = visitor.class_names_text(&call.callee)?;
                for argument in &call.arguments {
                    self.known_parts_local(argument.as_expression()?, parts, text)?;
                }
                Some(())
            }
            expression => {
                match self.known_side_local(expression, text)? {
                    LocalKnownSide::Styles(side) => parts.push(LocalKnownPart::Styles(side)),
                    LocalKnownSide::Class(class) => {
                        let class = match expression {
                            Expression::ComputedMemberExpression(member)
                                if text == Text::Classes
                                    && matches!(
                                        unwrap_syntax_only(&member.object),
                                        Expression::ObjectExpression(object)
                                            if object.properties.iter().all(|property| matches!(
                                                property,
                                                ObjectPropertyKind::ObjectProperty(property)
                                                    if property.kind == PropertyKind::Init
                                                        && !property.method
                                                        && !property.computed
                                                        && property.key.static_name().is_some_and(|key| key != "__proto__")
                                                        && matches!(unwrap_syntax_only(&property.value), Expression::StringLiteral(_))
                                            ))
                                    ) =>
                            {
                                let has_tag = Expression::new_binary_expression(
                                    SPAN,
                                    Expression::new_unary_expression(
                                        SPAN,
                                        UnaryOperator::Typeof,
                                        member
                                            .expression
                                            .clone_in_with_semantic_ids(visitor.ast.allocator()),
                                        &visitor.ast,
                                    ),
                                    BinaryOperator::StrictEquality,
                                    Expression::new_string_literal(
                                        SPAN,
                                        "string",
                                        None,
                                        &visitor.ast,
                                    ),
                                    &visitor.ast,
                                );
                                S::Source::from_conditional(ConditionalExpression::boxed(
                                    SPAN,
                                    has_tag,
                                    class.string_class(&visitor.ast).into_expression(),
                                    Expression::new_string_literal(SPAN, "", None, &visitor.ast),
                                    &visitor.ast,
                                ))
                            }
                            _ => class,
                        };
                        parts.push(LocalKnownPart::Class(class));
                    }
                    LocalKnownSide::Empty => {}
                }
                Some(())
            }
        }
    }
}
