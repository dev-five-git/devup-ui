use super::*;
use oxc_ast::ast::IdentifierReference;
use oxc_ast_visit::Visit;
use oxc_span::SourceType;

pub(super) fn reads(code: &str, name: &str) -> usize {
    struct Reads<'s> {
        name: &'s str,
        count: usize,
    }
    impl<'a> Visit<'a> for Reads<'_> {
        fn visit_identifier_reference(&mut self, value: &IdentifierReference<'a>) {
            self.count += usize::from(value.name == self.name);
        }
    }
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut counted = Reads { name, count: 0 };
    counted.visit_program(&parsed.program);
    counted.count
}

#[test]
#[serial]
fn core_regression_when_runtime_order_has_one_source_test_reads_it_once() {
    let source = "import {jsx} from 'react/jsx-runtime'; import {Box} from '@devup-ui/react'; function render(isActive){return jsx(Box,{styleOrder:isActive?5:10,bg:'red',p:'4'})}";
    let result = output(source);
    assert_eq!(reads(&result.code, "isActive"), 1, "{}", result.code);
}

#[test]
#[serial]
fn core_regression_when_same_spelling_occurs_in_order_and_bg_preserves_two_reads() {
    let source = "import {jsx} from 'react/jsx-runtime'; import {Box} from '@devup-ui/react'; function render(isActive){return jsx(Box,{styleOrder:isActive?5:10,bg:isActive?'red':'blue'})}";
    let result = output(source);
    assert_eq!(reads(&result.code, "isActive"), 2, "{}", result.code);
}

#[test]
#[serial]
fn core_regression_when_runtime_order_is_logical_reads_absence_test_once() {
    let source = "import {jsx} from 'react/jsx-runtime'; import {Box} from '@devup-ui/react'; function render(a){return jsx(Box,{styleOrder:a===1&&5,bg:'red',p:'4'})}";
    let result = output(source);
    assert_eq!(reads(&result.code, "a"), 1, "{}", result.code);
}

#[test]
#[serial]
fn core_regression_when_emotion_css_condition_changes_does_not_replay_it() {
    for active in [true, false] {
        let source = format!(
            "/** @jsxImportSource @emotion/react */\nconst config={{get active(){{trace.push('css');return {active}=== (trace.length===2)}}}}; const a=<div id={{(trace.push('id'),'a')}} css={{config.active?{{color:'red'}}:{{color:'blue'}}}} title={{(trace.push('title'),'b')}}/>;"
        );
        let result = compile_emotion(&source).required("finite Emotion css condition must compile");
        let evaluated = whole::evaluate_code(&result.code, "a.props.className");
        assert_eq!(
            evaluated.trace,
            serde_json::json!(["id", "css", "title"]),
            "{}",
            result.code
        );
        assert!(
            evaluated
                .element
                .as_str()
                .required("Emotion css selection must emit a class string")
                .contains(if active { "red" } else { "blue" })
        );
    }
}

#[test]
#[serial]
fn core_regression_when_runtime_getter_is_used_twice_keeps_each_source_selection() {
    for active in [true, false] {
        let source = format!(
            "import {{jsx}} from 'react/jsx-runtime'; import {{Box}} from '@devup-ui/react'; let on={active}; const config={{get active(){{trace.push(on);const result=on;on=!on;return result}}}}; const a=jsx(Box,{{styleOrder:config.active?5:10,bg:config.active?'red':'blue'}});"
        );
        let result = whole::evaluate(&source, "a.props.className");
        assert_eq!(result.trace, serde_json::json!([active, !active]));
        let class = result
            .element
            .as_str()
            .required("runtime order selection must emit a class string");
        assert!(
            class.contains(if active { "blue--5-" } else { "red--10-" }),
            "{class}"
        );
    }
}

#[test]
#[serial]
fn core_regression_when_nested_runtime_order_is_lazy_preserves_test_sequence() {
    for active in [true, false] {
        let source = format!(
            "import {{jsx}} from 'react/jsx-runtime'; import {{Box}} from '@devup-ui/react'; const choose=(name,value)=>(trace.push(name),value); const a=jsx(Box,{{id:(trace.push('id'),'a'),styleOrder:choose('outer',{active})?(choose('inner',false)?2:3):4,bg:choose('bg',true)?'red':'blue',title:(trace.push('title'),'b')}});"
        );
        let result = whole::evaluate(&source, "a.props.className");
        assert_eq!(
            result.trace,
            if active {
                serde_json::json!(["id", "outer", "inner", "bg", "title"])
            } else {
                serde_json::json!(["id", "outer", "bg", "title"])
            }
        );
        assert!(
            result
                .element
                .as_str()
                .required("nested order selection must emit a class string")
                .contains(if active { "red--3-" } else { "red--4-" })
        );
    }
}

#[test]
#[serial]
fn core_regression_when_runtime_logical_order_is_absent_keeps_one_getter_read() {
    for active in [true, false] {
        let source = format!(
            "import {{jsx}} from 'react/jsx-runtime'; import {{Box}} from '@devup-ui/react'; const config={{get active(){{trace.push('order');return {active}}}}}; const a=jsx(Box,{{styleOrder:config.active&&5,bg:'red'}});"
        );
        let result = whole::evaluate(&source, "a.props.className");
        assert_eq!(result.trace, serde_json::json!(["order"]));
        assert!(
            result
                .element
                .as_str()
                .required("logical order selection must emit a class string")
                .contains(if active { "red--5-" } else { "red--255-" })
        );
    }
}

#[test]
#[serial]
fn core_regression_when_runtime_order_is_invalid_reports_one_located_error() {
    let source = "import {jsx} from 'react/jsx-runtime'; import {Box} from '@devup-ui/react'; function render(order){return jsx(Box,{styleOrder:order,bg:'red'})}";
    let message = error(source);
    assert_eq!(message.lines().count(), 1, "{message}");
    assert!(message.starts_with("a.tsx:1:"), "{message}");
}
