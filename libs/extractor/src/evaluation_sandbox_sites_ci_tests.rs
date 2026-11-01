use oxc_ast_visit::Visit;

use super::Reads;

#[test]
fn unannotated_identifier_is_preserved_without_an_invented_read_site() {
    // Given
    let source = "unknown";
    let allocator = oxc_allocator::Allocator::default();
    let parsed =
        oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::default()).parse();
    let scoping = oxc_semantic::Scoping::default();
    let mut reads = Reads {
        scoping: &scoping,
        source,
        helper: "read",
        changes: Vec::new(),
        sites: Vec::new(),
        path: "unannotated.js",
    };
    // When
    reads.visit_program(&parsed.program);
    // Then
    assert_eq!(reads.changes, vec![]);
    assert_eq!(reads.sites, vec![]);
}
