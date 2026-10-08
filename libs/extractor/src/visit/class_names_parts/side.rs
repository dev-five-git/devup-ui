use super::{
    CloneIn, Expression, GetAllocator, KnownStyles, LocalKnownPart, LocalKnownSide, LocalParts,
    LocalSource, Text, UnaryOperator, unwrap_syntax_only,
};
use crate::gen_class_name::emission::merge_roots;

impl<'a, S: LocalSource<'a>> LocalParts<'_, '_, 'a, S> {
    pub(in crate::visit) fn known_side_local(
        &self,
        expression: &Expression<'a>,
        text: Text,
    ) -> Option<LocalKnownSide<'a, S::Class>> {
        let visitor = self.visitor;
        if let Some(finite) = visitor.style_values.finite(expression) {
            return Some(LocalKnownSide::Styles(vec![KnownStyles::Finite(
                finite.clone(),
                expression.clone_in_with_semantic_ids(visitor.ast.allocator()),
            )]));
        }
        let expression = unwrap_syntax_only(expression);
        let text = if text == Text::Arguments && crate::css_utils::literal::is_rule_text(expression)
        {
            Text::Rules
        } else {
            text
        };
        if let Some(styles) = visitor.style_values.styles(expression) {
            return Some(LocalKnownSide::Styles(vec![KnownStyles::Known(
                styles.to_vec(),
            )]));
        }
        match expression {
            Expression::StringLiteral(literal)
                if text == Text::Rules && literal.value.trim().is_empty() =>
            {
                Some(LocalKnownSide::Empty)
            }
            Expression::ObjectExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
                if text == Text::Rules || matches!(expression, Expression::ObjectExpression(_)) =>
            {
                Some(LocalKnownSide::Styles(vec![KnownStyles::Rules(
                    expression.clone_in_with_semantic_ids(visitor.ast.allocator()),
                )]))
            }
            Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => {
                Some(LocalKnownSide::Empty)
            }
            Expression::Identifier(identifier) if identifier.name == "undefined" => {
                Some(LocalKnownSide::Empty)
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void => {
                Some(LocalKnownSide::Empty)
            }
            Expression::CallExpression(call)
                if visitor.class_names_text(&call.callee).is_some() =>
            {
                self.folded_side_local(expression, text)
            }
            Expression::ArrayExpression(_) if !visitor.class_names_scope.is_empty() => {
                self.folded_side_local(expression, text)
            }
            expression => self
                .source
                .class(&visitor.ast, expression)
                .map(LocalKnownSide::Class),
        }
    }

    fn folded_side_local(
        &self,
        expression: &Expression<'a>,
        text: Text,
    ) -> Option<LocalKnownSide<'a, S::Class>> {
        let mut parts = Vec::new();
        self.known_parts_local(expression, &mut parts, text)?;
        let mut styles = Vec::new();
        let mut classes = Vec::new();
        for part in parts {
            match part {
                LocalKnownPart::Styles(side) => styles.extend(side),
                LocalKnownPart::Class(class) => classes.push(class),
                LocalKnownPart::Conditional { .. } => return None,
            }
        }
        match (styles.is_empty(), classes.is_empty()) {
            (true, true) => Some(LocalKnownSide::Empty),
            (false, true) => Some(LocalKnownSide::Styles(styles)),
            (true, false) => merge_roots(&self.visitor.ast, classes).map(LocalKnownSide::Class),
            (false, false) => None,
        }
    }
}
