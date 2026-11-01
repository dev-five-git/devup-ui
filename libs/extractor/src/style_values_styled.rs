use oxc_allocator::TakeIn;
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_ast_visit::VisitMut;

use super::{Reads, StyleValue, StyleValues};
use crate::utils::unwrap_syntax_only_mut;

impl<'a> StyleValues<'a> {
    /// Keep whole-rule selection operands distinct from ordinary class-string values.
    pub(crate) fn read_in_styled(&self, ast: &AstBuilder<'a>, expression: &mut Expression<'a>) {
        let Expression::CallExpression(call) = expression else {
            self.read_in(ast, expression);
            return;
        };
        self.read_in(ast, &mut call.callee);
        for argument in &mut call.arguments {
            let expression = match argument {
                oxc_ast::ast::Argument::SpreadElement(spread) => &mut spread.argument,
                argument => argument.to_expression_mut(),
            };
            let Some((_, returned)) =
                crate::css_prop::render_function(unwrap_syntax_only_mut(expression))
            else {
                if let Some(mut reads) = self.reads(ast, false) {
                    reads.rule_source(expression);
                }
                continue;
            };
            let mut rules = returned.take_in(ast);
            self.read_in(ast, expression);
            if let Some(mut reads) = self.reads(ast, false) {
                reads.rule_source(&mut rules);
            }
            if let Some((_, returned)) =
                crate::css_prop::render_function(unwrap_syntax_only_mut(expression))
            {
                *returned = rules;
            }
        }
    }
}

impl<'a> Reads<'_, 'a> {
    fn rule_source(&mut self, expression: &mut Expression<'a>) {
        let expression = unwrap_syntax_only_mut(expression);
        if matches!(self.value(expression), Some(StyleValue::Class(_, Some(_)))) {
            return;
        }
        match expression {
            Expression::StringLiteral(literal) if self.calls.contains_key(&literal.span.start) => {}
            Expression::ConditionalExpression(branch) => {
                self.visit_expression(&mut branch.test);
                self.rule_source(&mut branch.consequent);
                self.rule_source(&mut branch.alternate);
            }
            Expression::LogicalExpression(logical) => {
                self.rule_source(&mut logical.left);
                self.rule_source(&mut logical.right);
            }
            Expression::ArrayExpression(array) => {
                for element in &mut array.elements {
                    match element {
                        oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                            self.rule_source(&mut spread.argument);
                        }
                        element => {
                            if let Some(expression) = element.as_expression_mut() {
                                self.rule_source(expression);
                            }
                        }
                    }
                }
            }
            _ => {
                let source = std::mem::replace(&mut self.source, true);
                self.visit_expression(expression);
                self.source = source;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::{CloneIn, GetAllocator};
    use oxc_ast::ast::Statement;

    #[test]
    fn rule_source_when_array_spread_contains_a_known_class_recovers_its_object() {
        // Given
        let arena = oxc_allocator::Allocator::default();
        let ast = AstBuilder::new(&arena);
        let parsed = oxc_parser::Parser::new(
            &arena,
            "const mixin={color:'red'};styled.div([... [mixin]]);",
            oxc_span::SourceType::tsx(),
        )
        .parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let scoping = std::rc::Rc::new(
            oxc_semantic::SemanticBuilder::new()
                .build(&parsed.program)
                .semantic
                .into_scoping(),
        );
        let symbol = scoping
            .get_root_binding("mixin".into())
            .unwrap_or_else(|| panic!("mixin binding"));
        let Statement::VariableDeclaration(declaration) = &parsed.program.body[0] else {
            panic!("declaration")
        };
        let mut values = StyleValues::new(scoping);
        values.insert(symbol, StyleValue::Class("generated".into(), None));
        values.rules.insert(
            symbol,
            declaration.declarations[0]
                .init
                .as_ref()
                .unwrap_or_else(|| panic!("initializer"))
                .clone_in_with_semantic_ids(ast.allocator()),
        );
        let Statement::ExpressionStatement(statement) = &parsed.program.body[1] else {
            panic!("styled call")
        };
        let mut expression = statement
            .expression
            .clone_in_with_semantic_ids(ast.allocator());
        // When
        values.read_in_styled(&ast, &mut expression);
        // Then
        let Expression::CallExpression(call) = &expression else {
            panic!("call")
        };
        let Expression::ArrayExpression(array) = call.arguments[0].to_expression() else {
            panic!("array")
        };
        let oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) = &array.elements[0] else {
            panic!("spread")
        };
        let Expression::ArrayExpression(inner) = &spread.argument else {
            panic!("spread array")
        };
        assert!(matches!(
            inner.elements[0].as_expression(),
            Some(Expression::ObjectExpression(_))
        ));
        assert!(!crate::utils::readable_code(&expression).contains("generated"));
    }
}
