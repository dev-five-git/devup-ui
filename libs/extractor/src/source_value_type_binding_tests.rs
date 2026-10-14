use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, IdentifierReference};
use oxc_ast_visit::{Visit, VisitMut, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::{Scope, is_bound};

struct BindingReads(Vec<bool>);

impl<'a> Visit<'a> for BindingReads {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if matches!(identifier.name.as_str(), "undefined" | "NaN" | "Infinity") {
            self.0.push(is_bound(identifier));
        }
        walk::walk_identifier_reference(self, identifier);
    }
}

#[rstest]
#[case("useValue(undefined,NaN,Infinity)", vec![false,false,false])]
#[case("function f(undefined,NaN,Infinity){useValue(undefined,NaN,Infinity)}", vec![true,true,true])]
#[case("function f(){const undefined='8px';useValue(undefined)}useValue(undefined)", vec![true,false])]
fn binding_facts_distinguish_globals_when_original_references_are_queried(
    #[case] source: &str,
    #[case] expected: Vec<bool>,
) {
    // Given: original Oxc references include both bound and global spellings.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let (_scope, _) = Scope::enter(&parsed.program, "/entry.tsx", None, "@devup-ui/react");
    // When: source spans have been marked by extraction provenance.
    let mut program = parsed.program;
    crate::provenance::MarkRanges(&[(0, source.len())]).visit_program(&mut program);
    let mut reads = BindingReads(Vec::new());
    reads.visit_program(&program);
    // Then: the binding query follows original symbols rather than names.
    assert_eq!(reads.0, expected);
}

#[test]
fn generated_identifier_is_global_when_replacement_reuses_bound_source_span() {
    // Given: a source span originally refers to a lexical binding.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, "const value='8px';value", SourceType::tsx()).parse();
    let (_scope, _) = Scope::enter(&parsed.program, "/entry.tsx", None, "@devup-ui/react");
    let oxc_ast::ast::Statement::ExpressionStatement(statement) = &parsed.program.body[1] else {
        panic!("fixture must contain an identifier read");
    };
    let Expression::Identifier(original) = &statement.expression else {
        panic!("fixture must contain an identifier read");
    };
    // When: the inliner's generated literal replaces that read at the same span.
    let builder = oxc_ast::builder::AstBuilder::new(&allocator);
    let replacement = Expression::new_identifier(original.span, "undefined", &builder);
    let Expression::Identifier(generated) = replacement else {
        panic!("builder must produce an identifier");
    };
    // Then: the generated global does not inherit the old lexical binding.
    assert!(!is_bound(&generated));
}
