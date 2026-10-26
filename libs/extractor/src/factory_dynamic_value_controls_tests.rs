use super::*;

#[rstest]
#[case::direct("other:OWN")]
#[case::template("other:`${OWN}`")]
#[case::selector("selectors:{ '&:hover':{ other:OWN } }")]
#[case::member("other:(({ ...{},a:OWN }) as const)[key]")]
#[serial]
fn lexical_helpers_stay_local_when_prepared_values_copy_semantic_references(
    #[case] props: &str,
    #[values(false, true)] namespace: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let (parameter, call, argument) = if namespace {
        (
            "ui",
            "ui.css({overflow:'hidden'})",
            "{css:rules=>(trace.push(rules.overflow),'local')}",
        )
    } else {
        (
            "i",
            "i({overflow:'hidden'})",
            "rules=>(trace.push(rules.overflow),'local')",
        )
    };
    let props = props.replace("OWN", call);
    let source = format!(
        "{} import {{jsx as o}} from 'react/jsx-runtime'; function render({parameter},trace,key,index){{ return o(t,{{{props}}}); }}",
        imports(namespace)
    );
    // When
    let output = extract_factory(&source, single_css);
    // Then: the distinct local value is returned, without own overflow styles.
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Static(_)))
            .count(),
        0
    );
    let invocation = format!("render({argument},trace,'a',0)");
    let observed = render_observation(&output, &invocation, (!single_css).then_some(FILE));
    let (trace, values): (Vec<String>, Vec<String>) = serde_json::from_str(&observed).unwrap();
    assert_eq!(values.len(), trace.len());
    assert_eq!(values, vec!["local"; trace.len()]);
    assert_eq!(trace, ["hidden"]);
}

#[rstest]
#[case::direct("other:OWN")]
#[case::member("other:(({ ...{},a:OWN }) as const)[key]")]
#[serial]
fn block_local_helpers_run_once_when_factory_preparation_respects_lexical_bindings(
    #[case] props: &str,
    #[values(false, true)] namespace: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let helper = "rules=>(trace.push('block:'+rules.overflow),'block-local')";
    let (declaration, call) = if namespace {
        (
            format!("const ui={{css:{helper}}};"),
            "ui.css({overflow:'hidden'})",
        )
    } else {
        (format!("const i={helper};"), "i({overflow:'hidden'})")
    };
    let props = props.replace("OWN", call);
    let source = format!(
        "{} import {{jsx as o}} from 'react/jsx-runtime'; function render(trace,key){{ {{ {declaration} return o(t,{{{props}}}); }} }}",
        imports(namespace)
    );
    // When
    let output = extract_factory(&source, single_css);
    // Then
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Static(_)))
            .count(),
        0
    );
    assert_eq!(
        render_observation(&output, "render(trace,'a')", (!single_css).then_some(FILE)),
        "[[\"block:hidden\"],[\"block-local\"]]"
    );
}

#[rstest]
#[case::false_value(("false", "false"))]
#[case::null_value(("null", "null"))]
#[case::undefined_value(("undefined", "undefined"))]
#[serial]
fn ordinary_helper_preserves_absence_when_dynamic_value_is_serialized(
    #[case] sample: (&str, &str),
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let (value, expected) = sample;
    let source = "import {Box as t} from '@devup-ui/react'; import {jsx as o} from 'react/jsx-runtime'; function render(value,trace){const held=x=>(trace.push('held'),x); return o(t,{other:held(value)});}";
    // When
    let output = extract_factory(source, single_css);
    // Then: compare the actual held value, distinguishing undefined from null.
    let invocation = format!(
        "(() => {{const result=render({value},trace); trace.push(String(Object.values(result.props.style)[0])); return {{props:{{}}}};}})()"
    );
    assert_eq!(
        render_observation(&output, &invocation, (!single_css).then_some(FILE)),
        format!("[[\"held\",{expected:?}],[]]")
    );
}

#[rstest]
#[serial]
fn nested_helper_keeps_lazy_branch_and_order_when_preparing_template(
    #[values(false, true)] single_css: bool,
    #[values(false, true)] h: bool,
) {
    // Given: laziness lives inside the retained template, not a queued eager prop branch.
    let _state = crate::test_state::TestStateGuard::new();
    let source = "import {Box as t,css as i} from '@devup-ui/react'; import {jsx as o} from 'react/jsx-runtime'; function render(h,trace){const held=x=>(trace.push('held'),x); return o(t,{other:`${h?held(i({overflow:'hidden'})):''}${(trace.push('tail'),'')}`});}";
    // When
    let output = extract_factory(source, single_css);
    // Then
    assert_eq!(
        held_observation(&output, &format!("render({h},trace)"), single_css),
        if h {
            "[[\"held\",\"tail\"],[\"hidden\"]]"
        } else {
            "[[\"tail\"],[]]"
        }
    );
}

#[rstest]
#[serial]
fn conditional_arms_select_real_css_when_each_holds_a_compiled_call(
    #[values(false, true)] single_css: bool,
    #[values(false, true)] h: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let source = "import {Box as t,css as i} from '@devup-ui/react'; import {jsx as o} from 'react/jsx-runtime'; function render(h){return o(t,{other:h?i({overflow:'hidden'}):i({overflow:'scroll'})});}";
    // When
    let output = extract_factory(source, single_css);
    // Then
    assert_eq!(
        held_observation(&output, &format!("render({h})"), single_css),
        if h {
            "[[],[\"hidden\"]]"
        } else {
            "[[],[\"scroll\"]]"
        }
    );
}

#[rstest]
#[serial]
fn names_repeat_after_reset_when_multiple_imports_and_local_collisions_coexist(
    #[values(false, true)] namespace: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let call = if namespace { "ui.css" } else { "i" };
    let source = format!(
        "{} import {{css as j}} from '@devup-ui/react'; import {{jsx as o}} from 'react/jsx-runtime'; function render(trace){{const a='local-a',ui$css='local-ui'; return o(t,{{other:`${{{call}({{overflow:'hidden'}})}} ${{j({{overflow:'scroll'}})}} ${{{call}({{overflow:'hidden'}})}}`}});}}",
        imports(namespace)
    );
    // When
    let outputs: Vec<_> = (0..2)
        .map(|_| {
            let _state = crate::test_state::TestStateGuard::new();
            css::set_prefix(Some("held-".into()));
            let output = extract_factory(&source, single_css);
            let held = held_observation(&output, "render(trace)", single_css);
            (output, held)
        })
        .collect();
    // Then
    assert_eq!(outputs[0].0.code, outputs[1].0.code);
    assert_eq!(outputs[0].0.styles, outputs[1].0.styles);
    assert_eq!(outputs[0].1, outputs[1].1);
    let (trace, mut values): (Vec<String>, Vec<String>) =
        serde_json::from_str(&outputs[0].1).unwrap();
    values.sort();
    assert_eq!(
        (trace, values),
        (vec![], vec!["hidden".into(), "scroll".into()])
    );
    assert!(outputs[0].0.code.contains("held-"));
}
