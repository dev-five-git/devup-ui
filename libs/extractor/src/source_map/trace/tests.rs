use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::*;

#[test]
fn marks_tie_the_generated_tokens_to_the_parsed_ones() {
    let source = "let   a  =  1;\nlet b = a;\n";
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, source, SourceType::mjs())
        .parse()
        .program;
    let output = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some(PathBuf::from("a.js")),
            ..CodegenOptions::default()
        })
        .build(&program);
    assert_eq!(output.code, "let a = 1;\nlet b = a;\n");
    let marks = output
        .map
        .map(|map| marks(&map, &output.code, source))
        .unwrap_or_default();
    assert!(marks.contains(&(15, 19)), "{marks:?}");
    assert!(marks.contains(&(19, 23)), "{marks:?}");
    assert!(marks.is_sorted(), "{marks:?}");
}

#[test]
fn a_position_follows_the_closest_mark_and_the_text_after_it() {
    let trace = Trace::new(&[(11, 13)], &[]);
    let generated = "let a = 1; throw 2;";
    let source = "let a = 1;\n  throw 2;";
    assert_eq!(trace.resolve(generated, source, 11), 13);
    assert_eq!(trace.resolve(generated, source, 14), 16);
    assert_eq!(trace.resolve(generated, source, 3), 3);
    assert_eq!(trace.resolve("x 'a'", "x \"a\"", 3), 0);
}

#[test]
fn the_layers_of_edits_are_undone_last_made_first() {
    let first = [(1, 3, 3)];
    let last = [(5, 7, 1)];
    let trace = Trace::new(&[(5, 5)], &[&last, &first]);
    assert_eq!(trace.resolve("aXYZdG", "abcdef", 5), 4);
    assert_eq!(trace.resolve("aXYZdG", "abcdef", 0), 0);
}

#[rstest::rstest]
#[case("const value = foo(bar());")]
#[case("const value = (")]
fn existing_marks_are_preserved_when_printed_syntax_cannot_be_paired(#[case] generated: &str) {
    // Given
    let source = "const value=foo();bar();";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    let mut marked = vec![(0, 6)];
    // When
    complete_marks(&parsed.program, generated, &mut marked);
    // Then
    assert_eq!(Trace::new(&marked, &[]).resolve(generated, source, 0), 6);
}

#[test]
fn existing_duplicate_marks_keep_the_last_mapping_when_syntax_marks_are_added() {
    // Given
    let source = "const value=foo();";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    let generated = Codegen::new().build(&parsed.program).code;
    let mut marked = vec![(0, 0), (0, 6)];
    // When
    complete_marks(&parsed.program, &generated, &mut marked);
    // Then
    assert_eq!(Trace::new(&marked, &[]).resolve(&generated, source, 0), 6);
}
