use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn direct_nullish_choice_reads_once_when_factory_input_changes_between_renders(
    #[values(false, true)] single_css: bool,
) {
    // Given: only the first getter read is absent, and constructing the component must not read it.
    let source = "const trace=[];let count=0;const state={get maybe(){trace.push('maybe');return ++count===1?undefined:true;}};const make=input=>styled.div`color:blue;${(input.maybe??false)&&{color:'red'}};`;const Card=make(state);const defined=trace.slice();";
    // When: the complete generated renderer evaluates the factory-lexical choice twice.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const first=render({target:Card,props:{}});const firstTrace=trace.slice();const second=render({target:Card,props:{}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(second,'color')]);",
        single_css,
    );
    // Then: each render reads once, keeping blue before the next render selects red.
    assert_eq!(
        observed,
        serde_json::json!([[], ["maybe"], ["maybe", "maybe"], ["blue"], ["red"]])
    );
}

#[rstest]
#[case::selected("", "blue")]
#[case::covered("color:green;", "green")]
#[serial]
fn direct_discarded_objects_keep_reads_when_and_selects_later_rules(
    #[case] later: &str,
    #[case] color: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: both authored object operands have distinct getters, even if all their rules are covered.
    let source = format!(
        "const trace=[];const data={{get before(){{trace.push('before');return 'red';}},get after(){{trace.push('after');return 'blue';}}}};const make=input=>styled.div`${{({{color:input.before}})&&{{color:input.after}}}};{later}`;const Card=make(data);const defined=trace.slice();"
    );
    // When: the generated renderer evaluates the original direct logical expression.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const element=render({target:Card,props:{}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: discarded and covered leaves still run once, in source order.
    assert_eq!(
        observed,
        serde_json::json!([[], ["before", "after"], [color]])
    );
}

#[rstest]
#[case::or(("({color:input.before})||{color:input.after}", serde_json::json!(["before"]), "red"))]
#[case::coalesce(("({color:input.before})??{color:input.after}", serde_json::json!(["before"]), "red"))]
#[case::and(("input.off&&{color:input.after}", serde_json::json!(["off"]), "blue"))]
#[case::truthy_scalar(("true||{color:input.after}", serde_json::json!([]), "blue"))]
#[case::null_scalar(("null&&{color:input.after}", serde_json::json!([]), "blue"))]
#[case::unreachable_call(("({})??unknownRules()", serde_json::json!([]), "blue"))]
#[serial]
fn direct_choices_skip_unreachable_operands_when_factory_input_supplies_getters(
    #[case] scenario: (&str, serde_json::Value, &str),
    #[values(false, true)] single_css: bool,
) {
    let (fixture, trace, color) = scenario;
    // Given: an unreachable getter or call would change the trace or throw if executed.
    let source = format!(
        "const trace=[];const state={{get before(){{trace.push('before');return 'red';}},get after(){{trace.push('after');return 'wrong';}},get off(){{trace.push('off');return false;}}}};const make=input=>styled.div`color:blue;${{{fixture}}};`;const Card=make(state);const defined=trace.slice();"
    );
    // When: one real render follows the direct source expression's short circuit.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const element=render({target:Card,props:{}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: only reachable operands execute and omission keeps the earlier declaration.
    assert_eq!(observed, serde_json::json!([[], trace, [color]]));
}

#[rstest]
#[case::empty_or(("empty||{color:input.after}", serde_json::json!(["after"]), "orange"))]
#[case::empty_and(("empty&&unknownRules()", serde_json::json!([]), "blue"))]
#[case::nonempty_or(("mixin||unknownRules()", serde_json::json!([]), "red"))]
#[case::written_empty_or(("({})||{color:input.after}", serde_json::json!([]), "blue"))]
#[serial]
fn direct_known_mixins_keep_selection_facts_when_written_empty_objects_are_truthy(
    #[case] scenario: (&str, serde_json::Value, &str),
    #[values(false, true)] single_css: bool,
) {
    let (fixture, trace, color) = scenario;
    // Given: known empty CSS and a written empty object intentionally have different selection facts.
    let source = format!(
        "const trace=[];const empty=css({{}});const mixin=css({{color:'red'}});const state={{get after(){{trace.push('after');return 'orange';}}}};const make=input=>styled.div`color:blue;${{{fixture}}};`;const Card=make(state);const defined=trace.slice();"
    );
    // When: the original direct expression selects or skips the later rule operand.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const element=render({target:Card,props:{}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: semantic CSS identity, truthiness and D1 are retained without reading class strings.
    assert_eq!(observed, serde_json::json!([[], trace, [color]]));
}

#[rstest]
#[serial]
fn direct_conditional_operands_keep_source_selection_when_later_rules_cover_both_branches(
    #[values(false, true)] single_css: bool,
) {
    // Given: a conditional object operand must evaluate its selected leaves before the final AND operand.
    let source = "const trace=[];let selected=true;const state={get on(){trace.push('on');return selected;},get yes(){trace.push('yes');return 'red';},get no(){trace.push('no');return 'blue';},get after(){trace.push('after');return 'orange';}};const make=input=>styled.div`${(input.on?{color:input.yes}:{color:input.no})&&{color:input.after}};`;const Card=make(state);const defined=trace.slice();";
    // When: two real renders follow opposite source branches without re-binding the factory input.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const first=render({target:Card,props:{}});const firstTrace=trace.slice();selected=false;const second=render({target:Card,props:{}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(second,'color')]);",
        single_css,
    );
    // Then: shape selection and scalar selection stay distinct, preserving each evaluated leaf.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["on", "yes", "after"],
            ["on", "yes", "after", "on", "no", "after"],
            ["orange"],
            ["orange"]
        ])
    );
}

#[rstest]
#[serial]
fn direct_record_values_keep_units_when_known_spreads_inherit_selector_and_layer_context(
    #[values(false, true)] single_css: bool,
) {
    // Given: a direct rule record retains static shorthand units and unconverted dynamic values.
    let source = "const trace=[];const state={get on(){trace.push('on');return true;},get width(){trace.push('width');return 2;}};const make=input=>styled.div`@layer theme { &:hover { ${input.on?{...{width:input.width},margin:2,opacity:0.5}:null}; } }`;const Card=make(state);const defined=trace.slice();";
    // When: the generated renderer selects the written spread record in its enclosing context.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{}});JSON.stringify([defined,trace,values(element,'width',0,'&:hover'),values(element,'margin',0,'&:hover'),values(element,'opacity',0,'&:hover').map(Number),atoms(element).map(r=>r[4])]);",
        single_css,
    );
    // Then: direct record capture does not apply whole-props-callback px conversion or lose context.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["on", "width"],
            [2],
            ["8px"],
            [0.5],
            ["theme", "theme", "theme"]
        ])
    );
}

#[rstest]
#[serial]
fn direct_discarded_spreads_keep_evaluation_when_only_the_final_rule_shape_is_selected(
    #[values(false, true)] single_css: bool,
) {
    // Given: the discarded object's runtime spread must execute without being interpreted as CSS.
    let source = "const trace=[];const state={get discarded(){trace.push('spread');return {get color(){trace.push('leaf');return 'red';}};},get after(){trace.push('after');return 'blue';}};const make=input=>styled.div`${({...input.discarded})&&{color:input.after}};`;const Card=make(state);const defined=trace.slice();";
    // When: one actual render evaluates the discarded object before the selected known-shape object.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: source evaluation is retained, but discarded runtime keys do not become style rules.
    assert_eq!(
        observed,
        serde_json::json!([[], ["spread", "leaf", "after"], ["blue"]])
    );
}

#[rstest]
#[serial]
fn selected_runtime_spread_stays_a_located_error_when_its_rule_shape_is_unknown(
    #[values(false, true)] single_css: bool,
) {
    // Given: the same runtime spread is selected, so its unknown keys would have to become CSS rules.
    let source = "import {styled} from '@devup-ui/react';\nconst make=input=>styled.div`${({...input.rules})};`;";
    // When: native extraction attempts to admit the selected unknown-shape record.
    let result = crate::extract(
        "direct-selected-spread.ts",
        source,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    );
    // Then: it remains rejected at the authored location, without a runtime rule interpreter.
    let error = result
        .err()
        .unwrap_or_else(|| panic!("selected unknown rule shape must be refused"))
        .to_string();
    assert!(error.starts_with("direct-selected-spread.ts:2:"), "{error}");
}
