use super::*;

#[path = "coverage_cases/choices.rs"]
mod choices;
#[path = "coverage_cases/composition.rs"]
mod composition;
#[path = "coverage_cases/origins.rs"]
mod origins;
#[path = "coverage_cases/preflight.rs"]
mod preflight;
#[path = "coverage_cases/producers.rs"]
mod producers;
#[path = "coverage_cases/siblings.rs"]
mod siblings;
#[path = "coverage_cases/w38m_arrow.rs"]
mod w38m_arrow;

#[test]
#[serial]
fn computed_order_keys_when_static_follow_the_strict_metadata_contract() {
    for key in [
        "styleOrder",
        "['styleOrder']",
        "[`style${'Order'}`]",
        "[('styleOrder')]",
    ] {
        let source =
            format!("import {{css}} from '@devup-ui/react'; const a=css({{{key}:2,color:'red'}});");
        assert_eq!(
            orders(&source),
            vec![("color".to_string(), 0, Some(2))],
            "{key}"
        );
        let source = format!(
            "import {{css}} from '@devup-ui/react'; const a=css({{{key}:'01',color:'red'}});"
        );
        assert!(error(&source).contains("canonical decimal string"), "{key}");
    }
}

#[test]
#[serial]
fn computed_order_keys_when_duplicate_or_spread_keep_the_last_order() {
    let source = concat!(
        "import {css} from '@devup-ui/react'; const a=css({styleOrder:2,...{styleOrder:",
        "3},color:'red'});"
    );
    assert_eq!(orders(source), vec![("color".to_string(), 0, Some(3))]);
}

#[test]
#[serial]
fn computed_order_keys_when_number_coerced_accept_exact_numeric_values() {
    let source = "import {css} from '@devup-ui/react'; const a=css({[`style${'Order'}`]:+'01',color:'red'});";
    assert_eq!(orders(source), vec![("color".to_string(), 0, Some(1))]);
}

#[test]
#[serial]
fn css_when_literal_spread_composes_keeps_the_later_winner() {
    let source =
        "import {css} from '@devup-ui/react'; const a=css(...[{color:'red'},{color:'blue'}]);";
    let actual = whole::evaluate(source, "a");
    assert_eq!(actual.element, "color-0-blue--255-a");
}

#[test]
#[serial]
fn css_when_empty_returns_no_class_and_when_single_extracts_the_property() {
    let source = "import {css} from '@devup-ui/react'; const a=css();const b=css({color:'red'});";
    let actual = whole::evaluate(source, "[a,b]");
    assert_eq!(
        actual.element,
        serde_json::json!(["", "color-0-red--255-a"])
    );
}

#[test]
#[serial]
fn css_when_unsupported_argument_reports_the_original_call() {
    let source = "import {css} from '@devup-ui/react';\nconst a=css(getRules());";
    let actual = error(source);
    assert!(actual.starts_with("a.tsx:2:9:"), "{actual}");
    assert!(actual.contains("getRules()"), "{actual}");
}

#[test]
#[serial]
fn typography_when_call_or_getter_selects_a_class_reads_once() {
    let source = "import {Box} from '@devup-ui/react'; const getTypography=()=>(trace.push('call'),'heading'); const config={get typography(){trace.push('get');return 'body'}}; const a=<Box typography={getTypography()}/>; const b=<Box typography={config.typography}/>;";
    let actual = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(actual.trace, serde_json::json!(["call", "get"]));
    assert_eq!(
        actual.element,
        serde_json::json!(["typo-heading", "typo-body"])
    );
}

#[test]
#[serial]
fn literal_when_hole_interrupts_a_declaration_key_reports_the_hole() {
    let source = "import {css} from '@devup-ui/react';\nconst a=css`co${getKey()}lor:red;`;";
    let actual = error(source);
    assert!(actual.starts_with("a.tsx:2:17:"), "{actual}");
    assert!(actual.contains("getKey()"), "{actual}");
}

#[test]
#[serial]
fn metadata_when_nested_in_a_value_is_rejected_at_the_value() {
    let source = "import {css} from '@devup-ui/react';\nconst a=css({color:{styleOrder:'01'}});";
    let actual = error(source);
    assert!(actual.starts_with("a.tsx:2:32:"), "{actual}");
    assert!(actual.contains("styleOrder"), "{actual}");
}

#[test]
#[serial]
fn keyframes_when_logical_frame_contains_order_reports_the_key() {
    let source = "import {keyframes} from '@devup-ui/react';\nconst a=keyframes({from:flag&&{styleOrder:2,opacity:0}});";
    let actual = error(source);
    assert!(actual.contains("a.tsx:2:32:"), "{actual}");
    assert!(actual.contains("has no effect"), "{actual}");
}

#[test]
#[serial]
fn stylex_when_dynamic_arrow_contains_order_reports_the_key() {
    let source = "import * as stylex from '@stylexjs/stylex';\nconst a=stylex.create({root:value=>({styleOrder:2,color:value})});";
    let actual = error(source);
    assert!(actual.starts_with("a.tsx:2:38:"), "{actual}");
    assert!(actual.contains("has no effect"), "{actual}");
}

#[test]
#[serial]
fn stylex_when_dynamic_arrow_has_a_block_reports_the_unsupported_source() {
    let source = "import * as stylex from '@stylexjs/stylex';\nconst a=stylex.create({root:value=>{return {color:value}}});";
    let actual = error(source);
    assert!(actual.starts_with("a.tsx:2:"), "{actual}");
    assert!(actual.contains("value"), "{actual}");
}

#[test]
#[serial]
fn order_when_literal_truthiness_selects_an_arm_keeps_its_layer() {
    for (condition, selected) in [
        ("true", 2),
        ("false", 3),
        ("null", 3),
        ("''", 3),
        ("'x'", 2),
        ("!true", 3),
        ("``", 3),
        ("`x`", 2),
    ] {
        let source = format!(
            "import {{css}} from '@devup-ui/react'; const a=css({{styleOrder:{condition}?2:3,color:'red'}});"
        );
        assert_eq!(
            orders(&source),
            vec![("color".to_string(), 0, Some(selected))],
            "{condition}"
        );
    }
}

#[test]
#[serial]
fn order_when_true_logical_guard_selects_order_uses_that_layer() {
    let source = "import {css} from '@devup-ui/react'; const a=css({styleOrder:true&&3,backgroundColor:'blue'});";
    assert_eq!(
        orders(source),
        vec![("background-color".to_string(), 0, Some(3))]
    );
}

#[test]
#[serial]
fn jsx_factory_when_spread_order_is_overridden_preserves_props_and_effects() {
    let source = "import {Box} from '@devup-ui/react';import {jsx} from 'react/jsx-runtime'; const config={get styleOrder(){trace.push('order');return 2},get title(){trace.push('title');return 'kept'}};const a=jsx(Box,{...config,styleOrder:3,color:'red'});";
    let actual = whole::evaluate(source, "[a.props.title,a.props.className.trim()]");
    assert_eq!(actual.trace, serde_json::json!(["order", "title"]));
    assert_eq!(
        actual.element,
        serde_json::json!(["kept", "color-0-red--3-a"])
    );
}

#[test]
#[serial]
fn equal_identifier_branches_when_effectful_test_is_read_keep_one_evaluation() {
    let source = "import {Box} from '@devup-ui/react';let color='red';const flag=()=>(trace.push('flag'),true);const a=<Box color={flag()?color:color}/>;";
    let actual = whole::evaluate(source, "a.props.style");
    assert_eq!(actual.trace, serde_json::json!(["flag"]));
    assert!(
        actual
            .element
            .as_object()
            .required("dynamic color must emit a variable")
            .values()
            .any(|value| value == "red")
    );
}

#[test]
#[serial]
fn jsx_factory_when_finite_literal_spread_precedes_order_keeps_source_reads() {
    let source = "import {Box} from '@devup-ui/react';import {jsx} from 'react/jsx-runtime';const mark=name=>(trace.push(name),name);const a=jsx(Box,{...{styleOrder:2,title:mark('spread')},styleOrder:3,color:'red',id:mark('id')});";
    let actual = whole::evaluate(
        source,
        "[a.props.title,a.props.id,a.props.className.trim()]",
    );
    assert_eq!(actual.trace, serde_json::json!(["spread", "id"]));
    assert_eq!(
        actual.element,
        serde_json::json!(["spread", "id", "color-0-red--3-a"])
    );
}

#[cfg(test)]
#[path = "coverage_cases/w38n_mixin_controls.rs"]
mod w38n_mixin_controls;
#[cfg(test)]
#[path = "coverage_cases/w38n_mixin_diagnostics.rs"]
mod w38n_mixin_diagnostics;
#[cfg(test)]
#[path = "coverage_cases/w38n_mixin_source.rs"]
mod w38n_mixin_source;

#[cfg(test)]
#[path = "coverage_cases/w38o_logical_controls.rs"]
mod w38o_logical_controls;
#[cfg(test)]
#[path = "coverage_cases/w38o_logical_finite.rs"]
mod w38o_logical_finite;
#[cfg(test)]
#[path = "coverage_cases/w38o_logical_guards.rs"]
mod w38o_logical_guards;
#[cfg(test)]
#[path = "coverage_cases/w38o_logical_oracle.rs"]
mod w38o_logical_oracle;
#[cfg(test)]
#[path = "coverage_cases/w38o_logical_source.rs"]
mod w38o_logical_source;
#[cfg(test)]
#[path = "coverage_cases/w38p_saved_choices.rs"]
mod w38p_saved_choices;
#[cfg(test)]
#[path = "coverage_cases/w38p_saved_oracle.rs"]
mod w38p_saved_oracle;
#[cfg(test)]
#[path = "coverage_cases/w38p_saved_source.rs"]
mod w38p_saved_source;

#[cfg(test)]
#[path = "coverage_cases/w38q_leaf_cases.rs"]
mod w38q_leaf_cases;
#[cfg(test)]
#[path = "coverage_cases/w38q_leaf_oracle.rs"]
mod w38q_leaf_oracle;
#[cfg(test)]
#[path = "coverage_cases/w38q_leaf_source.rs"]
mod w38q_leaf_source;

#[cfg(test)]
#[path = "coverage_cases/w38r_u2_attrs.rs"]
mod w38r_u2_attrs;
#[cfg(test)]
#[path = "coverage_cases/w38r_u2_fallback.rs"]
mod w38r_u2_fallback;
