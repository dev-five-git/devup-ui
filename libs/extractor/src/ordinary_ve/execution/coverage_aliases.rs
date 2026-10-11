use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

use super::{
    Stylesheet,
    aliases::{Alias, Aliases},
};

#[rstest]
#[case("module.exports")]
#[case("module['exports']")]
#[case("exports")]
fn const_commonjs_alias_is_readonly_when_only_exported(#[case] input: &str) {
    let code = format!("const alias={input};module.exports.copy=alias;");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let stylesheet = Stylesheet {
        filename: "/aliases.ts",
        code: &code,
        source: &code,
        edits: &[],
    };
    let aliases = Aliases::new(stylesheet, &selection);

    let result = aliases.classify(
        if input == "exports" {
            "exports"
        } else {
            "module.exports"
        },
        "alias",
    );

    assert!(matches!(result, Alias::Readonly));
}

#[rstest]
#[case("module.exports")]
#[case("exports")]
fn const_commonjs_alias_is_mutable_when_a_transitive_alias_is_written(#[case] input: &str) {
    let code = format!("const alias={input};const next=alias;\nnext.space='12px';");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let stylesheet = Stylesheet {
        filename: "/aliases.ts",
        code: &code,
        source: &code,
        edits: &[],
    };
    let aliases = Aliases::new(stylesheet, &selection);

    let result = aliases.classify(input, "alias");

    match result {
        Alias::Mutable(at) => assert_eq!(super::policy::place(stylesheet, at), "/aliases.ts:2:1"),
        Alias::Readonly | Alias::Other => panic!("mutation through next was lost"),
    }
}

#[test]
fn alias_is_readonly_when_its_write_belongs_to_a_selected_initializer() {
    let code = "import {style} from '@vanilla-extract/css';const input={ padding:8 };const alias=input;const box=(()=>{alias.padding=12;return style(input)})();";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let aliases = Aliases::new(
        Stylesheet {
            filename: "/selected.ts",
            code,
            source: code,
            edits: &[],
        },
        &selection,
    );

    let result = aliases.classify("input", "alias");

    assert!(matches!(result, Alias::Readonly));
}
