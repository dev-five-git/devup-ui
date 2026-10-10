mod capture;
mod capture_edges;
mod elements;
mod generation;
mod naming;
mod normalization;
mod operands;
mod origins;
mod presence_edges;
mod residual_operands;

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

pub(crate) fn expression<'a>(allocator: &'a Allocator, source: &'a str) -> Expression<'a> {
    let parsed = Parser::new(allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression fixture");
    };
    crate::utils::unwrap_syntax_only(&statement.expression).clone_in(allocator)
}

fn dynamic(property: &str, identifier: &str) -> crate::ExtractStyleValue {
    crate::ExtractStyleValue::Dynamic(
        crate::extract_style::extract_dynamic_style::ExtractDynamicStyle::new(
            property, 0, identifier, None,
        ),
    )
}

pub(crate) fn code(expression: &Expression<'_>) -> String {
    crate::utils::expression_to_code(expression)
        .trim()
        .trim_end_matches(';')
        .to_string()
}

#[test]
#[should_panic(expected = "expression fixture")]
fn residual_expression_fixture_rejects_valid_declaration_statement() {
    // Given: valid TS, but not the expression-statement fixture contract.
    let allocator = Allocator::default();
    // When / Then: the helper intentionally rejects the statement shape.
    expression(&allocator, "let value = 1;");
}

mod spread_alignment;
