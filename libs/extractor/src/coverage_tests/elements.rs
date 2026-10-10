use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("styled.div")]
#[case("css")]
#[serial]
fn hard_string_spread_reports_the_authored_call(#[case] factory: &str) {
    // Given
    let source = format!(
        "import {{styled,css}} from '@devup-ui/react';\nexport const A = {factory}(...'color:red;');"
    );
    let offset = source
        .find(&format!("{factory}(..."))
        .unwrap_or_else(|| panic!("authored call"));
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .unwrap_or_else(|| panic!("authored line"))
        .len()
        + 1;
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    // When
    let Err(actual) = crate::extract("spread.tsx", &source, crate::ExtractOption::default()) else {
        panic!("string iteration is not static style composition");
    };
    // Then
    assert_eq!(
        actual.to_string(),
        format!(
            "spread.tsx:{line}:{column}: Cannot compose `...\"color:red;\"` at build time: each style must be a rule object, a class, or a condition choosing between them; pass the value itself instead of spreading it"
        )
    );
}

#[rstest]
#[case("styled.div", "{[Symbol.iterator]: function*(){yield {color:'red'}}}")]
#[case("css", "{[Symbol.iterator]: function*(){yield {color:'red'}}}")]
#[case("styled.div", "globalThis.styles")]
#[case("css", "globalThis.styles")]
#[serial]
fn hard_unsupported_spread_reports_the_authored_call(#[case] factory: &str, #[case] operand: &str) {
    // Given: a valid iterator or a value whose iterable contents are unknown.
    let source = format!(
        "import {{styled,css}} from '@devup-ui/react';\nexport const A = {factory}(...{operand});"
    );
    let offset = source
        .find(&format!("{factory}(..."))
        .unwrap_or_else(|| panic!("authored call"));
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .unwrap_or_else(|| panic!("authored line"))
        .chars()
        .count()
        + 1;
    let allocator = oxc_allocator::Allocator::default();
    let expression = oxc_parser::Parser::new(&allocator, operand, oxc_span::SourceType::ts())
        .parse_expression()
        .unwrap_or_else(|error| panic!("valid spread operand: {error:?}"));
    let code = crate::utils::readable_code(&expression);
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    // When
    let Err(actual) = crate::extract("spread.tsx", &source, crate::ExtractOption::default()) else {
        panic!("unsupported argument spread must not execute its iterator");
    };
    // Then: the call, rather than its computed iterator key, owns the diagnostic.
    assert_eq!(
        actual.to_string(),
        format!(
            "spread.tsx:{line}:{column}: Cannot compose `...{code}` at build time: each style must be a rule object, a class, or a condition choosing between them; pass the value itself instead of spreading it"
        )
    );
}

#[test]
#[serial]
fn hard_array_css_spread_selects_the_later_literal_rule() {
    // Given
    let source = "import {css}from '@devup-ui/react';const node={className:css(...[{color:'blue'},{color:'red'}])};";
    // When
    let actual = selected(source, "JSON.stringify(assigned(node));");
    // Then
    assert_eq!(actual, "[\"color:red:0\"]");
}

#[rstest]
#[case("styled.div")]
#[case("styled('div')")]
#[serial]
fn hard_array_styled_spread_reads_its_getter_at_creation_only(#[case] factory: &str) {
    // Given
    let source = format!(
        "import {{styled}}from '@devup-ui/react';let reads=0;Object.defineProperty(globalThis,'color',{{get(){{reads++;return 'red'}}}});const A={factory}(...[{{color:globalThis.color}}]);const created=reads;const first=A({{}}),second=A({{}});"
    );
    // When
    let actual = selected(
        &source,
        "JSON.stringify([created,reads,first.className===second.className,Object.values(first.style),Object.values(second.style),assigned(first),assigned(second)]);",
    );
    // Then
    assert_eq!(
        actual,
        "[1,1,true,[\"red\"],[\"red\"],[\"color:red:0\"],[\"color:red:0\"]]"
    );
}

#[rstest]
#[case(
    "<Box className={state.enabled&&'p-4'}/>",
    true,
    "[[\"enabled\"],[\"padding:1rem:0\"]]"
)]
#[case("<Box className={state.enabled&&'p-4'}/>", false, "[[\"enabled\"],[]]")]
#[case(
    "jsx(Box,{className:state.enabled&&'p-4'})",
    true,
    "[[\"enabled\"],[\"padding:1rem:0\"]]"
)]
#[case(
    "jsx(Box,{className:state.enabled&&'p-4'})",
    false,
    "[[\"enabled\"],[]]"
)]
#[serial]
fn rebuilt_tailwind_logical_class_omits_falsy_values(
    #[case] expression: &str,
    #[case] enabled: bool,
    #[case] expected: &str,
) {
    // Given
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';let trace=[];const state={{get enabled(){{trace.push('enabled');return {enabled}}}}};const node={expression};"
    );
    // When
    let actual = selected(
        &source,
        "function jsx(tag,props){return props}JSON.stringify([trace,assigned(node)]);",
    );
    // Then: the true branch selects its emitted padding; false never becomes a class word.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn conditional_styled_argument_keeps_its_creation_evaluation_when_rendered_twice() {
    // Given: the whole authored rule argument selects a static rule at creation.
    let source = "import {styled}from '@devup-ui/react';let reads=0;Object.defineProperty(globalThis,'flag',{get(){reads++;return true}});const A=styled.div(flag?{color:'red'}:{color:'blue'});const created=reads;const first=A({}),second=A({});JSON.stringify([created,reads,first.className===second.className]);";
    // When
    let actual = evaluate(&compiled_jsx(source));
    // Then: rendering never reevaluates the already selected argument.
    assert_eq!(actual, "[1,1,true]");
}
