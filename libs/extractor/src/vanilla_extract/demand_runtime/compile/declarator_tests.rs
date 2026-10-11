use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::declarators;

#[test]
fn declarators_of_each_top_level_statement_kind() {
    let allocator = Allocator::default();
    let source = "import a from './a';\nconst plain = 1;\nexport const named = 2, other = 3;\nexport function helper() {}\nexport class Box {}\nfoo();";
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();

    let counts: Vec<Option<usize>> = parsed
        .program
        .body
        .iter()
        .map(|statement| declarators(statement).map(|declarations| declarations.len()))
        .collect();

    assert_eq!(
        counts,
        vec![None, Some(1), Some(2), None, None, None],
        "only const and let declarations, plain or exported, declare styling bindings"
    );
}
