use super::*;
use oxc_ast::ast::Statement;
use rstest::rstest;

#[rstest]
#[case("const first=second;const second=first;first();", false)]
#[case("import api from 'styled-components';api();", true)]
#[case("import api from '@emotion/styled';api.div();", true)]
#[case("const api=require('@devup-ui/react');api.css();", true)]
#[case("const api=require('@stylexjs/stylex');api.create();", true)]
#[case("const { css: api }=require('@devup-ui/react');api();", true)]
#[case("const { create: api }=require('@stylexjs/stylex');api();", true)]
#[case("const {[key]:api}=require('@devup-ui/react');api();", false)]
#[case("const {...api}=require('@devup-ui/react');api.css();", false)]
#[case("const [api]=require('@devup-ui/react');api();", false)]
#[case("const {css:{api}}=require('@devup-ui/react');api();", false)]
#[case(
    "const require=loader;const api=require('@devup-ui/react');api.css();",
    false
)]
#[case("const api=loader('@devup-ui/react');api.css();", false)]
#[case("let api;api();", false)]
#[case("function api(){}api();", false)]
#[case("import * as api from '@devup-ui/react';api[key]();", false)]
#[case("import * as api from '@devup-ui/react';api['setTheme']();", false)]
#[case("import * as api from '@devup-ui/react';api['css']();", true)]
#[case(
    "import {css as api} from '@devup-ui/react';const alias=api;alias();",
    true
)]
#[case(
    "import * as api from '@devup-ui/react';const alias=api;alias.css();",
    true
)]
#[case("(0,unknown)();", false)]
fn compiled_identity_when_given_semantic_declarations_honors_the_api_boundary(
    #[case] source: &str,
    #[case] expected: bool,
) {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = oxc_semantic::SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let Statement::ExpressionStatement(statement) = parsed
        .program
        .body
        .last()
        .unwrap_or_else(|| panic!("fixture statement"))
    else {
        panic!("call fixture")
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("call fixture")
    };
    let context = Context {
        nodes: semantic.nodes(),
        scoping: semantic.scoping(),
        style: &|_| true,
        css: None,
        init: None,
    };
    // When
    let compiled = context.compiled_call(&call.callee);
    // Then
    assert_eq!(compiled, expected, "{source}");
}
