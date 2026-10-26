use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[path = "props_function_self_reference.rs"]
mod self_reference;

#[path = "props_function_generic_self_reference.rs"]
mod generic_self_reference;

#[rstest]
#[serial]
fn parameter_defaults_and_rest_run_once_when_rules_have_multiple_leaves(
    #[values(false, true)] single_css: bool,
) {
    // Given: defaults do observable work and the callback receives only one actual argument.
    let source = "const trace=[];function fallback(){trace.push('fallback');return {c:'red',m:2};}const Card=styled.div(({data:{c,m}=fallback()},unused=(trace.push('extra'),'unused'),...tail)=>({color:c,margin:m,opacity:tail.length}));const defined=trace.slice();";
    let observation = "const first=render({target:Card,props:{}});const firstTrace=trace.slice();const second=render({target:Card,props:{data:{c:'blue',m:3}}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(first,'margin'),values(first,'opacity'),values(second,'color'),values(second,'margin')]);";
    // When: two renders take different branches of the destructuring default.
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then: binding work happens once per render, not once per surviving declaration.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["fallback", "extra"],
            ["fallback", "extra", "extra"],
            ["red"],
            ["2px"],
            [0],
            ["blue"],
            ["3px"]
        ])
    );
}

#[rstest]
#[case::empty("{}", serde_json::json!([]))]
#[case::static_rules("{color:'red'}", serde_json::json!(["red"]))]
#[serial]
fn defaults_keep_their_props_when_the_value_vector_is_empty(
    #[case] rules: &str,
    #[case] colors: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: the only props read is in a default initializer, including through authored JSX.
    let source = format!(
        "const trace=[];const Card=styled.div((p,unused=(trace.push(p.token),0))=>({rules}));const defined=trace.slice();const App=token=>jsx(Card,{{token}});"
    );
    let observation = "const first=render(App('one'));const firstTrace=trace.slice();const second=render(App('two'));JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(second,'color'),'token' in first.props]);";
    // When: static-only and empty callbacks render twice.
    let observed = execute_in_mode("test.ts", &source, observation, single_css);
    // Then: no definition-time work or erased initializer; the style-only prop stays withheld.
    assert_eq!(
        observed,
        serde_json::json!([[], ["one"], ["one", "two"], colors, colors, false])
    );
}

#[rstest]
#[serial]
fn unused_nested_binding_still_reads_when_the_rule_object_is_empty(
    #[values(false, true)] single_css: bool,
) {
    // Given: no dynamic leaf survives, but destructuring itself has an observable getter.
    let source = "const trace=[];const data={get c(){trace.push('c');return 'unused';}};const Card=styled.div(({data:{c}})=>({}));const defined=trace.slice();";
    // When: the generated component renders twice.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "render({target:Card,props:{data}});render({target:Card,props:{data}});JSON.stringify([defined,trace]);",
        single_css,
    );
    // Then: empty vectors still invoke the original parameter binding.
    assert_eq!(observed, serde_json::json!([[], ["c", "c"]]));
}

#[rstest]
#[serial]
fn rest_only_props_survive_when_authored_elements_call_the_component(
    #[values(false, true)] single_css: bool,
) {
    // Given: the first actual props object is reached through a rest formal.
    let source = "const Card=styled.div((...all)=>({color:all[0].tone,opacity:all.length}));const App=tone=>jsx(Card,{tone});";
    // When: the real JSX-call extraction and generated renderer run together.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const first=render(App('red'));const second=render(App('blue'));JSON.stringify([values(first,'color'),values(second,'color'),values(first,'opacity')]);",
        single_css,
    );
    // Then: there is exactly one actual argument and no pruned input to the rest binding.
    assert_eq!(observed, serde_json::json!([["red"], ["blue"], [1]]));
}

#[rstest]
#[serial]
fn arrows_keep_lexical_arguments_and_this_when_the_factory_has_both(
    #[values(false, true)] single_css: bool,
) {
    // Given: lexical factory state deliberately differs from render props.
    let source = "const receiver={color:'orange'};function make(factoryReceiver){return styled.div(p=>({color:arguments[0].color,opacity:this===factoryReceiver?0.5:0,margin:p.margin}));}const Card=make.call(receiver,receiver);";
    // When: two renders invoke the original arrow inside compiler-generated arrow wrappers.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const first=render({target:Card,props:{color:'red',margin:2}});const second=render({target:Card,props:{color:'blue',margin:3}});JSON.stringify([values(first,'color'),values(second,'color'),values(first,'opacity'),values(first,'margin'),values(second,'margin')]);",
        single_css,
    );
    // Then: lexical scope is retained while the actual props binding changes per render.
    assert_eq!(
        observed,
        serde_json::json!([["orange"], ["orange"], [0.5], ["2px"], ["3px"]])
    );
}

#[rstest]
#[serial]
fn normal_callback_direct_scope_survives_when_named_without_self_references(
    #[values(false, true)] single_css: bool,
) {
    // Given: a named callback observes strict this and its own actual arguments, not its identity.
    let source = "const Card=styled.div(function rules(p){'use strict';return {color:this===undefined&&arguments.length===1&&arguments[0]===p?arguments[0].color:'orange'};});";
    // When: the generated renderer invokes the original function.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{color:'red'}});JSON.stringify(values(element,'color'));",
        single_css,
    );
    // Then: no added actual argument or receiver changes the direct-call contract.
    assert_eq!(observed, serde_json::json!(["red"]));
}

#[rstest]
#[case::normal_leaf(
    "p=>({color:function(q){'use strict';return this===undefined&&arguments.length===1&&q===p?p.color:'orange';},margin:q=>p.margin})"
)]
#[case::arrow_leaf(
    "function(p){'use strict';return {color:()=>arguments[0].color,margin:()=>p.margin};}"
)]
#[serial]
fn function_leaves_retain_outer_bindings_when_invoked_without_an_array_receiver(
    #[case] callback: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: leaves close over outer callback bindings, rather than a recreated parameter list.
    let source = format!("const Card=styled.div({callback});");
    // When: both leaf kinds execute through real generated style expressions in two renders.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const first=render({target:Card,props:{color:'red',margin:2}});const second=render({target:Card,props:{color:'blue',margin:3}});JSON.stringify([values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin')]);",
        single_css,
    );
    // Then: closure state is current and ordinary property callbacks keep bare numeric values.
    assert_eq!(observed, serde_json::json!([["red"], [2], ["blue"], [3]]));
}

#[rstest]
#[case::no_formals("function(){return {color:arguments[0].tone};}")]
#[case::unused_formal("function(p){return {color:arguments[0].tone};}")]
#[serial]
fn own_arguments_keep_original_props_reads_when_authored_jsx_would_otherwise_prune_them(
    #[case] callback: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: a non-DOM input is read only through the normal callback's own arguments object.
    let source = format!("const Card=styled.div({callback});const App=tone=>jsx(Card,{{tone}});");
    // When: authored JSX and the generated renderer execute together for two props values.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const first=render(App('red'));const second=render(App('blue'));JSON.stringify([values(first,'color'),values(second,'color')]);",
        single_css,
    );
    // Then: original arguments reads retain the actual props rather than a pruned empty object.
    assert_eq!(observed, serde_json::json!([["red"], ["blue"]]));
}
