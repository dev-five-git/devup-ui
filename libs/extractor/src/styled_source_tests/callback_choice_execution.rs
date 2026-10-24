use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::call(false)]
#[case::template(true)]
#[serial]
fn choice_defaults_share_one_binding_pass_when_branches_have_distinct_keys(
    #[case] template: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: two binding defaults and a rest formal must see one actual props argument.
    let callback = "({on,data:{c,m}=fallback()},unused=(trace.push('extra'),0),...tail)=>on?{color:c,margin:m,opacity:tail.length}:{backgroundColor:c}";
    let factory = if template {
        format!("styled(Base)`${{{callback}}};`")
    } else {
        format!("styled(Base)({callback})")
    };
    let source = format!(
        "const trace=[];function fallback(){{trace.push('fallback');return {{c:'red',m:2}};}}const Base=styled.div({{color:'blue',backgroundColor:'black'}});const Card={factory};const defined=trace.slice();"
    );
    // When: actual generated renders select opposite branches and default paths.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const first=render({target:Card,props:{ on: true }});const firstTrace=trace.slice();const second=render({target:Card,props:{on:false,data:{ c: 'orange',m:3 }}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(first,'margin'),values(first,'opacity'),values(first,'background-color'),values(second,'color'),values(second,'background-color')]);",
        single_css,
    );
    // Then: omitted branch keys Keep the base and all selected leaves share one binding pass.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["fallback", "extra"],
            ["fallback", "extra", "extra"],
            ["red"],
            ["2px"],
            [0],
            ["black"],
            ["blue"],
            ["orange"]
        ])
    );
}

#[rstest]
#[case::empty("p.on ? {} : {color:'red'}", "", serde_json::json!([]))]
#[case::static_rules("p.on ? {color:'red'} : {}", "", serde_json::json!(["red"]))]
#[case::covered("p.on ? {color:data.value} : {}", "color:green;", serde_json::json!(["green"]))]
#[serial]
fn choice_evaluations_survive_when_no_dynamic_atom_remains(
    #[case] rules: &str,
    #[case] later: &str,
    #[case] color: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: the callback is still invoked when an empty/static/covered result needs no slots.
    let source = format!(
        "const trace=[];const data={{get value(){{trace.push('leaf');return 'red';}}}};const Card=styled.div`${{(p,unused=(trace.push('default'),0))=>{rules}}};{later}`;const defined=trace.slice();"
    );
    // When: the selected branch renders once.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const element=render({target:Card,props:{ on: true }});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: no slot survival test can erase authored binding or getter work.
    let trace = if later.is_empty() {
        serde_json::json!(["default"])
    } else {
        serde_json::json!(["default", "leaf"])
    };
    assert_eq!(observed, serde_json::json!([[], trace, color]));
}

#[rstest]
#[serial]
fn nullish_choice_reads_changing_getter_once_when_missing_branch_keeps_base(
    #[values(false, true)] single_css: bool,
) {
    // Given: the getter's next read differs, so testing the same operand twice changes selection.
    let source = "const trace=[];let reads=0;const input={get maybe(){trace.push('maybe');return ++reads===1?undefined:true;}};const Card=styled.div`color:blue;${p=>(p.input.maybe??false)&&{color:'red'}};`;const defined=trace.slice();";
    // When: the real renderer evaluates the original short circuit on two renders.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const first=render({target:Card,props:{input}});const firstTrace=trace.slice();const second=render({target:Card,props:{input}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(second,'color')]);",
        single_css,
    );
    // Then: the first undefined read keeps blue; only the second render selects red.
    assert_eq!(
        observed,
        serde_json::json!([[], ["maybe"], ["maybe", "maybe"], ["blue"], ["red"]])
    );
}

#[rstest]
#[serial]
fn discarded_object_evaluates_leaves_when_and_selects_the_right_rules(
    #[values(false, true)] single_css: bool,
) {
    // Given: both authored object occurrences have observable dynamic leaves.
    let source = "const trace=[];const data={get before(){trace.push('before');return 'red';},get after(){trace.push('after');return 'blue';}};const Card=styled.div`${p=>({color:data.before})&&{color:data.after}};`;const defined=trace.slice();";
    // When: the left truthy object's rules are discarded, but its evaluation is not.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: both getters execute once in authored order and only the right rules apply.
    assert_eq!(
        observed,
        serde_json::json!([[], ["before", "after"], ["blue"]])
    );
}

#[rstest]
#[case::normal(
    "function rules(p){'use strict';return p.on?{color:this===undefined&&arguments.length===1&&arguments[0]===p?p.color:'wrong'}:{color:p.other};}",
    "red",
    "blue"
)]
#[case::generic(
    "function rules<T>(p:T){'use strict';return p.on?{color:this===undefined&&arguments.length===1&&arguments[0]===p?p.color:'wrong'}:{color:p.other};}",
    "red",
    "blue"
)]
#[case::lexical(
    "p=>p.on?{color:arguments[0].color,opacity:this===factoryReceiver?0.5:0}:{color:this.color}",
    "orange",
    "orange"
)]
#[serial]
fn selected_choices_keep_scope_when_callback_form_is_preserved(
    #[case] callback: &str,
    #[case] first: &str,
    #[case] second: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: the lexical receiver/arguments differ from render props and normal callback scope.
    let source = format!(
        "const receiver={{color:'orange'}};function make(factoryReceiver){{return styled.div`${{{callback}}};`;}}const Card=make.call(receiver,receiver);"
    );
    // When: opposite branches execute inside the generated full render scope.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const first=render({target:Card,props:{on:true,color:'red'}});const second=render({target:Card,props:{on:false,other:'blue'}});JSON.stringify([values(first,'color'),values(second,'color')]);",
        single_css,
    );
    // Then: normal this/arguments stay own; arrow this/arguments stay factory-lexical.
    assert_eq!(observed, serde_json::json!([[first], [second]]));
}

#[rstest]
#[serial]
fn selected_function_leaves_close_over_the_original_binding_and_have_no_receiver(
    #[values(false, true)] single_css: bool,
) {
    // Given: a function leaf depends on an outer default binding and strict receiver semantics.
    let source = "const trace=[];const Card=styled.div`${({on,token=(trace.push('default'),'red')})=>on?{color:function(q){'use strict';return this===undefined&&arguments.length===1&&q.on?token:'wrong';},margin:q=>q.margin}:{color:'blue'}};`";
    // When: the selected closure is created inside the one original callback invocation.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{on:true,margin:2}});JSON.stringify([trace,values(element,'color'),values(element,'margin')]);",
        single_css,
    );
    // Then: property-function numeric results remain ordinary values, not whole-rule px leaves.
    assert_eq!(observed, serde_json::json!([["default"], ["red"], [2]]));
}
