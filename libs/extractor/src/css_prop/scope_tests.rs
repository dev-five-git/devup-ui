use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, IdentifierReference};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;

use super::{binding_of, reads_top_level, root_reference};

struct Reads<'s> {
    scoping: &'s Scoping,
    found: Vec<(String, bool, bool)>,
}

impl<'a> Visit<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        self.found.push((
            identifier.name.to_string(),
            binding_of(self.scoping, identifier).is_some(),
            reads_top_level(self.scoping, identifier),
        ));
    }
}

#[test]
fn references_tell_locals_from_top_level_bindings_and_globals() {
    let allocator = Allocator::default();
    let program = Parser::new(
        &allocator,
        "const top = 1; function f(local) { return [top, local, global]; }",
        SourceType::tsx(),
    )
    .parse()
    .program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let mut reads = Reads {
        scoping: &scoping,
        found: Vec::new(),
    };
    reads.visit_program(&program);
    assert_eq!(
        reads.found,
        vec![
            ("top".to_string(), true, true),
            ("local".to_string(), true, false),
            ("global".to_string(), false, true),
        ]
    );
}

#[test]
fn a_reference_without_semantic_data_reads_a_global() {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, "", SourceType::tsx())
        .parse()
        .program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let Ok(Expression::Identifier(identifier)) =
        Parser::new(&allocator, "a", SourceType::tsx()).parse_expression()
    else {
        panic!("a");
    };
    assert_eq!(binding_of(&scoping, &identifier), None);
    assert!(reads_top_level(&scoping, &identifier));
}

#[test]
fn the_root_of_a_member_chain_is_the_identifier_it_starts_at() {
    let allocator = Allocator::default();
    for (code, root) in [
        ("a", Some("a")),
        ("a.b", Some("a")),
        ("a[b]", Some("a")),
        ("a.b[c].d", Some("a")),
        ("a()", None),
        ("'a'", None),
    ] {
        let Ok(expression) = Parser::new(&allocator, code, SourceType::tsx()).parse_expression()
        else {
            panic!("{code}");
        };
        assert_eq!(
            root_reference(&expression).map(|identifier| identifier.name.as_str()),
            root,
            "{code}"
        );
    }
}
