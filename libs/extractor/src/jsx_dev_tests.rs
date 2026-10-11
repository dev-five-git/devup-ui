use crate::{ExtractOption, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast::ast::CallExpression;
use oxc_ast_visit::{Visit, walk};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use rstest::rstest;
use serial_test::serial;
use std::error::Error;

fn normalized(source: &str) -> String {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.errors().count(), 0);
    Codegen::new().build(&parsed.program).code
}

#[derive(Default)]
struct Calls<'s> {
    source: &'s str,
    calls: Vec<(String, Vec<String>)>,
}

impl<'a> Visit<'a> for Calls<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let callee = call.callee.span().source_text(self.source);
        if callee == "jsxDEV" || callee == "dev" || callee.ends_with(".jsxDEV") {
            self.calls.push((
                normalized(&format!("({callee});")),
                call.arguments
                    .iter()
                    .map(|arg| normalized(&format!("({});", arg.span().source_text(self.source))))
                    .collect(),
            ));
        }
        walk::walk_call_expression(self, call);
    }
}

fn calls(source: &str) -> Vec<(String, Vec<String>)> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.errors().count(), 0);
    let mut visitor = Calls {
        source,
        ..Calls::default()
    };
    visitor.visit_program(&parsed.program);
    visitor.calls
}

#[rstest]
#[case(
    "import {jsxDEV} from 'react/jsx-dev-runtime'; import {Box} from '@devup-ui/react';",
    "jsxDEV",
    "Box"
)]
#[case(
    "import {jsxDEV as dev} from 'react/jsx-dev-runtime'; import {Box as Card} from '@devup-ui/react';",
    "dev",
    "Card"
)]
#[case(
    "import * as runtime from 'react/jsx-dev-runtime'; import * as UI from '@devup-ui/react';",
    "runtime.jsxDEV",
    "UI.Box"
)]
#[case(
    "const runtime = require('react/jsx-dev-runtime'); const UI = require('@devup-ui/react');",
    "runtime.jsxDEV",
    "UI.Box"
)]
#[case(
    "const {jsxDEV} = require('react/jsx-dev-runtime'); const {Box} = require('@devup-ui/react');",
    "jsxDEV",
    "Box"
)]
#[case(
    "const {jsxDEV: dev} = require('react/jsx-dev-runtime'); const {Box} = require('@devup-ui/react');",
    "dev",
    "Box"
)]
#[serial]
fn jsx_dev_extracts_when_runtime_origin_is_known(
    #[case] imports: &str,
    #[case] callee: &str,
    #[case] component: &str,
) -> Result<(), Box<dyn Error>> {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "{imports} export const view = {callee}({component}, {{as: 'section', bg: 'tomato', color: color, id: 'card', props: {{title: 'hello'}}}}, key(), true, {{fileName: 'actual.mdx', lineNumber: 7, columnNumber: 3}}, this);"
    );
    let before = calls(&source);

    let output = extract("/src/page.mdx", &source, ExtractOption::default())?;

    let after = calls(&output.code);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].0, before[0].0);
    assert_eq!(after[0].1.len(), 6);
    assert_eq!(after[0].1[0], normalized("('section');"));
    assert_eq!(after[0].1[2..], before[0].1[2..]);
    assert!(after[0].1[1].contains("className"));
    assert!(after[0].1[1].contains("title: \"hello\""));
    assert!(after[0].1[1].contains("id: \"card\""));
    assert!(after[0].1[1].contains("color"));
    assert!(after[0].1[1].contains("--"));
    assert!(!after[0].1[1].contains("bg:"));
    assert_ne!(output.styles.len(), 0);
    Ok(())
}

#[rstest]
#[case(
    "import * as prod from 'react/jsx-runtime'; import * as other from 'react/jsx-runtime'; import * as runtime from 'react/jsx-dev-runtime'; import {Box} from '@devup-ui/react';"
)]
#[case(
    "const prod = require('react/jsx-runtime'); const runtime = require('react/jsx-dev-runtime'); const other = require('react/jsx-runtime'); const {Box} = require('@devup-ui/react');"
)]
#[serial]
fn jsx_dev_extracts_when_production_and_development_objects_coexist(
    #[case] imports: &str,
) -> Result<(), Box<dyn Error>> {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "{imports} export const view = [prod.jsx(Box, {{bg: 'red'}}), runtime.jsxDEV(Box, {{bg: 'blue'}}, void 0, false, source, self), other.jsxs(Box, {{color: 'green'}})];"
    );

    let output = extract("/src/mixed.js", &source, ExtractOption::default())?;

    assert_eq!(output.code.matches("className:").count(), 3);
    assert!(!output.code.contains("(Box"));
    assert_eq!(calls(&output.code)[0].1[2..], calls(&source)[0].1[2..]);
    Ok(())
}

#[rstest]
#[case("...readProps(), bg: 'red'")]
#[case(
    "as: active ? 'article' : 'section', styleOrder: active ? 1 : 2, bg: color, style: {opacity: 0.5}, styleVars: {'--custom': color}, props: {title: 'nested'}"
)]
#[serial]
fn jsx_dev_preserves_metadata_when_props_wrap_or_children_nest(
    #[case] props: &str,
) -> Result<(), Box<dyn Error>> {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{jsxDEV as dev}} from 'react/jsx-dev-runtime'; import {{Box}} from '@devup-ui/react'; export const view = dev(Box, {{{props}, children: dev(Box, {{p: 2}}, childKey, false, childSource, childSelf)}}, parentKey, true, parentSource, this);"
    );
    let before = calls(&source);

    let output = extract("/src/nested.js", &source, ExtractOption::default())?;

    let after = calls(&output.code);
    assert_eq!(after.len(), 2);
    for (old, new) in before.iter().zip(&after) {
        assert_eq!(new.0, old.0);
        assert_eq!(new.1.len(), 6);
        assert_eq!(new.1[2..], old.1[2..]);
        assert!(new.1[1].contains("className"));
    }
    assert_eq!(
        output.code.matches("readProps()").count(),
        usize::from(props.contains("readProps"))
    );
    assert!(!output.code.contains("dev(Box"));
    Ok(())
}

#[rstest]
#[case("import {jsxDEV} from 'unrelated'; jsxDEV('div', {bg: 'red'}, key, false, source, self);")]
#[case("import * as unrelated from 'unrelated'; unrelated.jsxDEV('div', {bg: 'red'});")]
#[case(
    "import {jsxDEV} from 'react/jsx-dev-runtime'; jsxDEV('div', {bg: 'red'}, key, false, source, self);"
)]
#[case(
    "import {jsxDEV} from 'react/jsx-dev-runtime'; jsxDEV(); jsxDEV(...args); jsxDEV('div'); jsxDEV('div', ...args);"
)]
#[case("import runtime from 'react/jsx-dev-runtime'; runtime.jsxDEV('div', {bg: 'red'});")]
#[case(
    "import {jsxDEV} from 'react/jsx-dev-runtime'; import * as UI from '@devup-ui/react'; jsxDEV(UI.Unknown, {bg: 'red'});"
)]
#[case(
    "const runtime = require('react/jsx-dev-runtime'); const UI = require('@devup-ui/react'); runtime.jsxDEV(UI.Unknown, {bg: 'red'});"
)]
#[serial]
fn jsx_dev_keeps_calls_when_origin_component_or_arguments_are_ineligible(
    #[case] control: &str,
) -> Result<(), Box<dyn Error>> {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{css}} from '@devup-ui/react'; export const cls = css({{color: 'blue'}}); {control}"
    );

    let output = extract("/src/control.js", &source, ExtractOption::default())?;

    assert!(output.code.contains(&normalized(control)));
    assert!(!output.code.contains("className"));
    Ok(())
}

#[rstest]
#[case("jsxDEV(Box)", "react/jsx-dev-runtime")]
#[case("jsxDEV(Box, ...args)", "react/jsx-dev-runtime")]
#[case("other.jsxDEV(Box, {})", "react/jsx-dev-runtime")]
#[case("jsxDEV(Box, {})", "unrelated")]
#[serial]
fn jsx_dev_keeps_runtime_read_diagnostics_when_call_is_ineligible(
    #[case] call: &str,
    #[case] runtime: &str,
) {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{jsxDEV}} from '{runtime}'; import {{Box}} from '@devup-ui/react'; {call};"
    );

    let result = extract("/src/control.js", &source, ExtractOption::default());

    assert!(result.is_err_and(|error| {
        let message = error.to_string();
        message.contains("Box") && message.contains("runtime")
    }));
}

#[test]
#[serial]
fn jsx_dev_namespace_lookup_keeps_production_parity() -> Result<(), Box<dyn Error>> {
    reset_class_map();
    reset_file_map();
    let source = "import {jsx} from 'react/jsx-runtime'; import * as UI from '@devup-ui/react'; export const view = jsx(UI.Box, {bg: 'red'});";

    let output = extract("/src/production.js", source, ExtractOption::default())?;

    assert!(output.code.contains("jsx(\"div\""));
    assert!(output.code.contains("className"));
    assert_ne!(output.styles.len(), 0);
    Ok(())
}

#[rstest]
#[case("jsx", "react/jsx-runtime")]
#[case("jsxDEV", "react/jsx-dev-runtime")]
#[serial]
fn jsx_dev_keeps_existing_callee_limits(
    #[case] method: &str,
    #[case] runtime: &str,
    #[values("(0, helper)(Box, {})", "runtime['CALL'](Box, {})", "rebound(Box, {})")] call: &str,
) {
    reset_class_map();
    reset_file_map();
    let call = call.replace("CALL", method);
    let source = format!(
        "import {{{method} as helper}} from '{runtime}'; import * as runtime from '{runtime}'; import {{Box}} from '@devup-ui/react'; const rebound = helper; {call};"
    );

    let result = extract("/src/limits.js", &source, ExtractOption::default());

    assert!(result.is_err_and(|error| error.to_string().contains("Box")));
}
