use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast_visit::VisitMut;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serial_test::serial;

use super::visit;
use crate::visit::DevupVisitor;

#[test]
#[serial]
fn files_that_bind_nothing_the_build_compiles_build_no_scoping() {
    let comment =
        visit("// @devup-ui/react\nconst css = (x) => x;\nexport const a = css({ color: 'red' });");
    assert_eq!(comment.errors, Vec::<String>::new());
    assert_eq!(comment.styles, 0);
    assert!(
        comment.code.contains("css({ color: \"red\" })"),
        "{}",
        comment.code
    );

    let other = visit("const css = require('other');\nexport const a = css({ color: 'red' });");
    assert_eq!(other.styles, 0);
    assert!(
        other.code.contains("css({ color: \"red\" })"),
        "{}",
        other.code
    );
}

#[test]
#[serial]
fn analysis_given_to_the_visitor_is_reused() {
    reset_class_map();
    reset_file_map();
    let allocator = Allocator::default();
    let code = "import { css } from '@devup-ui/react';\nexport function f(css) { return css({ color: 'red' }); }\nexport const a = css({ color: 'blue' });";
    let mut program = Parser::new(&allocator, code, SourceType::tsx())
        .parse()
        .program;
    let scoping = std::rc::Rc::new(
        oxc_semantic::SemanticBuilder::new()
            .build(&program)
            .semantic
            .into_scoping(),
    );
    let mut visitor =
        DevupVisitor::new(&allocator, "test.tsx", "@devup-ui/react", Vec::new(), None);
    visitor.reuse_scoping(Some(scoping));
    visitor.visit_program(&mut program);
    let printed = Codegen::new().build(&program).code;
    assert!(printed.contains("css({ color: \"red\" })"), "{printed}");
    assert!(printed.contains("export const a = \"a\";"), "{printed}");
    assert_eq!(visitor.styles.len(), 1);
}

#[test]
#[serial]
fn runtime_source_survives_when_syntax_validation_is_the_only_semantic_analysis()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "// @devup-ui/react\nconst handler=()=>document.body.append(window.name);export const view=<button onClick={handler}>{window.name}</button>;throw new Error('runtime only');";
    let allocator = Allocator::default();
    let authored = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(authored.diagnostics.len(), 0);
    let expected = Codegen::new().build(&authored.program).code;
    // When
    let output = crate::extract("scoping.tsx", source, crate::ExtractOption::default())?;
    // Then
    assert_eq!(output.styles.len(), 0);
    assert_eq!(output.code, expected);
    Ok(())
}
