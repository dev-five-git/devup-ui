use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::{ast::Statement, builder::AstBuilder};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::{Expression, Preparation, direct_lookup};
use crate::{extractor::styled_record_evaluation::Evaluations, utils::unwrap_syntax_only};

#[rstest]
#[case::leading_zero("({'05':'red','5':'blue'})['05'];", true)]
#[case::canonical_integer("({'05':'red','5':'blue'})['5'];", true)]
#[case::decimal("({'0.50':'red','0.5':'blue'})['0.50'];", true)]
#[case::canonical_decimal("({'0.50':'red','0.5':'blue'})['0.5'];", true)]
#[case::ts_as_index("({color:'red'})['color' as const];", false)]
fn direct_lookup_preserves_ast_when_checking_exact_index_agreement(
    #[case] source: &str,
    #[case] expected: bool,
) {
    // Given: real parsed object/index syntax, including a TS-as index wrapper.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("test fixture must be an expression");
    };
    let Expression::ComputedMemberExpression(member) = &statement.expression else {
        panic!("test fixture must be a computed member");
    };
    let original = format!("{member:?}");

    // When: the caller supplies the syntax-unwrapped object as Preparation::value does.
    let accepted = direct_lookup(unwrap_syntax_only(&member.object), &member.expression);

    // Then: exact object keys agree, but the unsupported TS-as index stays native.
    assert_eq!(accepted, expected, "{source}");
    assert_eq!(format!("{member:?}"), original);
}

#[test]
fn direct_lookup_rejects_when_object_is_not_literal() {
    // Given: a literal key that passes the key checks, but an unknown object.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, "unknown['color'];", SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("test fixture must be an expression");
    };
    let Expression::ComputedMemberExpression(member) = &statement.expression else {
        panic!("test fixture must be a computed member");
    };
    let original = format!("{member:?}");

    // When: the direct helper receives the unknown object without caller filtering.
    let accepted = direct_lookup(&member.object, &member.expression);

    // Then: rejection preserves the original AST; this borrowed API cannot capture reads.
    assert!(!accepted);
    assert_eq!(format!("{member:?}"), original);
}

#[rstest]
#[case::spread_object("({ color: 'red', ...unknown }[key()]);")]
#[case::unknown_object("unknown[key()];")]
fn selection_rejects_without_mutation_or_capture_when_object_is_unsupported(#[case] source: &str) {
    // Given: an unsupported object and an index whose evaluation would require capture.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("test fixture must be an expression");
    };
    let Expression::ComputedMemberExpression(member) = unwrap_syntax_only(&statement.expression)
    else {
        panic!("test fixture must be a computed member");
    };
    let mut member = member.clone_in(&allocator);
    let original = format!("{member:?}");
    let mut preparation = Preparation {
        reads: Evaluations::new(&builder, source.to_string()),
        affected: false,
    };
    let guard = preparation.reads.boolean(true);

    // When: selection is attempted directly, including a spread after a readable entry.
    let result = preparation.select_lookup(&mut member, &guard);

    // Then: rejection is atomic, even after collecting a valid prefix of object entries.
    assert_eq!(result, None);
    assert_eq!(format!("{member:?}"), original);
    assert_eq!(preparation.reads.bindings.len(), 0);
}
