use super::{accepts_declaration_goal, has_diagnostics};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{LabeledSpan, SourceType, Span};
use rstest::rstest;

const IMPORT_ERROR: &str = "Cannot use import statement outside a module";

#[test]
fn stock_result_is_untouched_when_goal_policy_accepts_hybrid_declarations() {
    // Given
    let arena = Allocator::default();
    let physical = SourceType::from_path("input.cjs").unwrap_or_else(|error| panic!("{error}"));
    let parsed = Parser::new(&arena, "import 'dep'; export const value = 1;", physical).parse();
    let diagnostics = parsed.diagnostics.clone();
    // When
    let invalid = has_diagnostics(&parsed);
    // Then
    assert!(!invalid);
    assert_eq!(parsed.diagnostics, diagnostics);
    assert_eq!(parsed.diagnostics.len(), 2);
    assert_eq!(parsed.program.source_type, physical);
    assert_eq!(parsed.program.body.len(), 2);
}

#[rstest]
#[case("unknown", vec![LabeledSpan::new(None, 0, 6)])]
#[case("Cannot use export statement outside a module", vec![LabeledSpan::new(None, 0, 6)])]
#[case(IMPORT_ERROR, vec![])]
#[case(IMPORT_ERROR, vec![LabeledSpan::new(None, 0, 6), LabeledSpan::new(None, 0, 6)])]
#[case(IMPORT_ERROR, vec![LabeledSpan::new(Some("label".into()), 0, 6)])]
#[case(IMPORT_ERROR, vec![LabeledSpan::new_primary_with_span(None, Span::new(0, 6))])]
#[case(IMPORT_ERROR, vec![LabeledSpan::new(None, 0, 5)])]
#[case(IMPORT_ERROR, vec![LabeledSpan::new(None, 0, 7)])]
#[case(IMPORT_ERROR, vec![LabeledSpan::new(None, 1, 6)])]
fn unknown_diagnostic_shape_is_rejected_when_keyword_is_a_direct_import(
    #[case] message: &str,
    #[case] labels: Vec<LabeledSpan>,
) {
    // Given
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, "import 'dep';", SourceType::cjs()).parse();
    // When / Then
    assert!(!accepts_declaration_goal(&parsed, message, &labels));
}

#[rstest]
#[case(SourceType::mjs())]
#[case(SourceType::ts().with_commonjs(true))]
#[case(SourceType::script())]
fn other_source_goals_are_rejected_when_diagnostic_shape_matches(#[case] source_type: SourceType) {
    // Given
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, "import 'dep';", source_type).parse();
    // When / Then
    assert!(!accepts_declaration_goal(
        &parsed,
        IMPORT_ERROR,
        &[LabeledSpan::new(None, 0, 6)]
    ));
}

#[rstest]
#[case("function f() { import 'dep'; }", 15)]
#[case("const value = 1;", 0)]
fn unmatched_top_level_shape_is_rejected_when_message_matches(
    #[case] source: &str,
    #[case] offset: u32,
) {
    // Given
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, source, SourceType::cjs()).parse();
    // When / Then
    assert!(!accepts_declaration_goal(
        &parsed,
        IMPORT_ERROR,
        &[LabeledSpan::new(None, offset, 6)]
    ));
}

#[rstest]
#[case("export 'dep';")]
#[case("impor")]
fn mismatching_source_bytes_are_rejected_when_ast_shape_matches(#[case] source: &str) {
    // Given: synthetic boundary data keeps the import AST but changes its backing bytes.
    let arena = Allocator::default();
    let mut parsed = Parser::new(&arena, "import 'dep';", SourceType::cjs()).parse();
    parsed.program.source_text = source;
    // When / Then
    assert!(!accepts_declaration_goal(
        &parsed,
        IMPORT_ERROR,
        &[LabeledSpan::new(None, 0, 6)]
    ));
}

#[test]
fn fatal_parse_is_rejected_when_a_goal_mismatch_precedes_it() {
    // Given
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, "import 'dep'; const value = ;", SourceType::cjs()).parse();
    // When / Then
    assert!(parsed.fatal_error);
    assert!(!accepts_declaration_goal(
        &parsed,
        IMPORT_ERROR,
        &[LabeledSpan::new(None, 0, 6)]
    ));
    assert!(has_diagnostics(&parsed));
}

#[test]
fn out_of_source_keyword_is_rejected_when_synthetic_ast_start_matches_label() {
    // Given
    let arena = Allocator::default();
    let mut parsed = Parser::new(&arena, "import 'dep';", SourceType::cjs()).parse();
    let oxc_ast::ast::Statement::ImportDeclaration(import) = &mut parsed.program.body[0] else {
        panic!("expected import");
    };
    import.span.start = 100;
    // When / Then
    assert!(!accepts_declaration_goal(
        &parsed,
        IMPORT_ERROR,
        &[LabeledSpan::new(None, 100, 6)]
    ));
}
