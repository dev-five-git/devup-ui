use crate::utils::{get_str_by_property_key, js_number_literal, keeps_bare_number};
use oxc_allocator::{Allocator, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        CallExpression, Expression, ImportDeclarationSpecifier, ObjectPropertyKind, Program,
        Statement, Str,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

struct Units<'a> {
    ast: AstBuilder<'a>,
    calls: Vec<String>,
}

impl<'a> Units<'a> {
    fn rules(&self, expression: &mut Expression<'a>) {
        match expression {
            Expression::ObjectExpression(object) => {
                for property in &mut object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && let Some(key) = get_str_by_property_key(&property.key)
                    {
                        self.value(&key, &mut property.value);
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &mut array.elements {
                    if let Some(expression) = element.as_expression_mut() {
                        self.rules(expression);
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.rules(&mut conditional.consequent);
                self.rules(&mut conditional.alternate);
            }
            Expression::LogicalExpression(logical) => self.rules(&mut logical.right),
            Expression::ParenthesizedExpression(inner) => self.rules(&mut inner.expression),
            _ => {}
        }
    }

    fn value(&self, key: &str, expression: &mut Expression<'a>) {
        if let Some(number) = js_number_literal(expression) {
            if number != 0.0 && !keeps_bare_number(key) {
                *expression = Expression::new_string_literal(
                    expression.span(),
                    Str::from_in(format!("{number}px").as_str(), self.ast.allocator()),
                    None,
                    &self.ast,
                );
            }
            return;
        }
        match expression {
            Expression::ObjectExpression(_) if key != "vars" => self.rules(expression),
            Expression::ConditionalExpression(conditional) => {
                self.value(key, &mut conditional.consequent);
                self.value(key, &mut conditional.alternate);
            }
            Expression::LogicalExpression(logical) => self.value(key, &mut logical.right),
            Expression::ParenthesizedExpression(inner) => self.value(key, &mut inner.expression),
            _ => {}
        }
    }
}

impl<'a> VisitMut<'a> for Units<'a> {
    fn visit_call_expression(&mut self, call: &mut CallExpression<'a>) {
        if matches!(&call.callee, Expression::Identifier(identifier) if self.calls.iter().any(|name| name == identifier.name.as_str()))
        {
            for argument in &mut call.arguments {
                if let Some(expression) = argument.as_expression_mut() {
                    self.rules(expression);
                }
            }
        }
        walk_mut::walk_call_expression(self, call);
    }
}

/// Preserve Emotion units for values reduced by the constant inliner.
pub(crate) fn pixelify<'a>(allocator: &'a Allocator, program: &mut Program<'a>, original: &str) {
    let source_allocator = Allocator::default();
    let source = Parser::new(&source_allocator, original, SourceType::tsx())
        .parse()
        .program;
    let mut calls = Vec::new();
    for statement in &source.body {
        if let Statement::ImportDeclaration(import) = statement
            && import.source.value == "@emotion/css"
        {
            for specifier in import.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(named) = specifier
                    && matches!(
                        named.imported.name().as_str(),
                        "css" | "keyframes" | "injectGlobal"
                    )
                {
                    calls.push(named.local.name.to_string());
                }
            }
        }
    }
    Units {
        ast: AstBuilder::new(allocator),
        calls,
    }
    .visit_program(program);
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_codegen::Codegen;
    use rstest::rstest;

    #[rstest]
    #[case(
        r"css([{ padding: 2 }, [{ margin: -3 }], , ...rest, null]);",
        r"css([{ padding: '2px' }, [{ margin: '-3px' }], , ...rest, null]);"
    )]
    #[case(
        r"css(flag ? { padding: 2 } : { margin: 3 });",
        r"css(flag ? { padding: '2px' } : { margin: '3px' });"
    )]
    #[case(
        r"css(flag && ({ padding: 2 }));",
        r"css(flag && ({ padding: '2px' }));"
    )]
    #[case(
        r"css({ padding: flag ? 2 : -3 });",
        r"css({ padding: flag ? '2px' : '-3px' });"
    )]
    #[case(r"css({ padding: flag && 2 });", r"css({ padding: flag && '2px' });")]
    #[case(
        r"css({ padding: (flag ? 2 : 3) });",
        r"css({ padding: (flag ? '2px' : '3px') });"
    )]
    #[case(
        r"keyframes({ from: { margin: 2 }, to: { margin: 0 } }); injectGlobal({ body: { padding: 3 } });",
        r"keyframes({ from: { margin: '2px' }, to: { margin: 0 } }); injectGlobal({ body: { padding: '3px' } });"
    )]
    #[case(
        r"css({ padding: 0, opacity: 0.5, lineHeight: 2, vars: { size: 3 }, margin: '4', ...rest, [key]: 5 }); cx({ picked: 2 }); other({ padding: 3 });",
        r"css({ padding: 0, opacity: 0.5, lineHeight: 2, vars: { size: 3 }, margin: '4', ...rest, [key]: 5 }); cx({ picked: 2 }); other({ padding: 3 });"
    )]
    fn preserves_emotion_units_when_rules_contain_nested_expressions(
        #[case] body: &str,
        #[case] expected_body: &str,
    ) {
        // Given
        let imports = "import {css,keyframes,injectGlobal,cx} from '@emotion/css';";
        let source = format!("{imports}{body}");
        let expected = format!("{imports}{expected_body}");
        let allocator = Allocator::default();
        let mut parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let expected_allocator = Allocator::default();
        let expected_program =
            Parser::new(&expected_allocator, &expected, SourceType::tsx()).parse();
        assert_eq!(expected_program.diagnostics.len(), 0);

        // When
        pixelify(&allocator, &mut parsed.program, &source);

        // Then
        assert_eq!(
            Codegen::new().build(&parsed.program).code,
            Codegen::new().build(&expected_program.program).code,
        );
    }
}
