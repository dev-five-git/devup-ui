use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn callbacks_share_authored_argument_order_when_controllers_and_covered_reads_interleave(
    #[values(false, true)] single_css: bool,
) {
    // Given: earlier/later getters, two callbacks, an opaque class, and an inactive branch.
    let source = "const trace=[];let selected=true;const state={get before(){trace.push('before');return 'orange';},get opaque(){trace.push('class');return 'opaque';},get on(){trace.push('on');return selected;},get margin(){trace.push('margin');return '4px';},get after(){trace.push('after');return 'black';}};const make=input=>styled.div({color:input.before},(p,unused=(trace.push('first'),0))=>({color:(trace.push('covered'),p.color)}),input.opaque,input.on?[{margin:input.margin},{background:'green'}]:[],(p,unused=(trace.push('second'),0))=>({opacity:p.opacity,'&:hover':{color:(trace.push('hover'),p.color)}}),{color:input.after});const Card=make(state);const defined=trace.slice();";
    let observation = r"const first=render({target:Card,props:{color:'red',opacity:0.5}});const firstTrace=trace.slice();trace.length=0;selected=false;const second=render({target:Card,props:{color:'blue',opacity:0.25}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(first,'color',0,'&:hover'),values(first,'margin'),values(second,'color'),values(second,'color',0,'&:hover'),values(second,'margin'),values(second,'opacity'),[first,second].map(element=>element.props.className.split(/\s+/).includes('opaque'))]);";
    // When: both branch states execute through the same generated component.
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then: covered authored reads still run, without callback-prepending or inactive leaf reads.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            [
                "before", "first", "covered", "class", "on", "margin", "second", "hover", "after"
            ],
            [
                "before", "first", "covered", "class", "on", "second", "hover", "after"
            ],
            ["black"],
            ["red"],
            ["4px"],
            ["black"],
            ["blue"],
            [],
            [0.25],
            [true, true]
        ])
    );
}

#[rstest]
#[serial]
fn repeated_leaf_occurrences_stay_distinct_when_later_rules_cover_the_callback(
    #[values(false, true)] single_css: bool,
) {
    // Given: the same getter occurs twice, and only its hover declaration survives composition.
    let source = "const trace=[];let n=0;const data={get color(){trace.push('color');return ++n===1?'red':'blue';}};const Card=styled.div(({data})=>({color:data.color,'&:hover':{color:data.color}}),{color:'green'});const defined=trace.slice();";
    // When: one actual render evaluates the authored occurrences in object-initializer order.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{data}});JSON.stringify([defined,trace,values(element,'color'),values(element,'color',0,'&:hover')]);",
        single_css,
    );
    // Then: the covered first read is neither removed nor deduplicated into the surviving read.
    assert_eq!(
        observed,
        serde_json::json!([[], ["color", "color"], ["green"], ["blue"]])
    );
}

#[rstest]
#[serial]
fn inherited_vectors_rebind_when_retargeted_extensions_use_authored_capture_names(
    #[values(false, true)] single_css: bool,
) {
    // Given: inherited attrs and bindings, colliding authored names, and distinct caller props.
    let source = "const trace=[];function make(__devupRecord0){const Base=styled.div.attrs({role:'button'})(p=>({color:(trace.push(__devupRecord0),p.tone),margin:p.margin}));const With=Base.withComponent('section');const Child=styled(Base)((p,__devupRecord1=(trace.push('child'),0))=>({opacity:p.opacity}));const Again=Child.withComponent('aside');return {With,Again};}const {With,Again}=make('base');const defined=trace.slice();const App=ref=>jsx(Again,{tone:'blue',margin:3,opacity:0.5,ref,children:'child',className:'caller',style:{color:'olive'},forwardedAs:'header'});";
    let observation = r"const first=render({target:With,props:{tone:'red',margin:2}});const second=render(App(7));JSON.stringify([defined,trace,first.target,second.target,values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin'),values(second,'opacity'),second.props.role,second.props.ref,second.props.children,second.props.as,second.props.style.color,second.props.className.split(/\s+/).includes('caller'),'tone' in second.props,slots(second)]);";
    // When: a retarget and a retargeted extension each render inherited callback captures.
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then: initializers and index reads are renamed together; no private CSS transport leaks.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["base", "base", "child"],
            "section",
            "aside",
            ["red"],
            ["2px"],
            ["blue"],
            ["3px"],
            [0.5],
            "button",
            7,
            "child",
            "header",
            "olive",
            true,
            false,
            []
        ])
    );
}

#[rstest]
#[serial]
fn empty_conditional_arrays_still_run_between_callbacks_when_neither_branch_has_rules(
    #[values(false, true)] single_css: bool,
) {
    // Given: an empty choice still has a getter between two otherwise static callbacks.
    let source = "const trace=[];const state={get on(){trace.push('on');return false;}};const make=input=>styled.div((p,unused=(trace.push('first'),0))=>({}),input.on?[]:[],(p,unused=(trace.push('last'),0))=>({color:'red'}));const Card=make(state);const defined=trace.slice();";
    // When: the original callback and controller schedules execute in one render.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: no empty-vector or empty-array optimization erases observable work or reorders it.
    assert_eq!(
        observed,
        serde_json::json!([[], ["first", "on", "last"], ["red"]])
    );
}

#[rstest]
#[serial]
fn dependent_record_fallbacks_follow_inherited_and_own_vectors_when_presence_changes(
    #[values(false, true)] single_css: bool,
) {
    // Given: a conditional record override depends on earlier callback-vector leaves.
    let source = "const trace=[];let selected=true;const state={get on(){trace.push('on');return selected;},get color(){trace.push('after');return 'blue';}};const make=input=>{const Base=styled.div((p,unused=(trace.push('base'),0))=>({color:p.base}));const Child=styled(Base)((p,unused=(trace.push('own'),0))=>({color:p.tone,margin:p.margin}),{...(input.on?{color:input.color}:{})});return Child.withComponent('section');};const Card=make(state);const defined=trace.slice();";
    let observation = "const first=render({target:Card,props:{base:'orange',tone:'red',margin:2}});const firstTrace=trace.slice();trace.length=0;selected=false;const second=render({target:Card,props:{base:'orange',tone:'green',margin:3}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(second,'color'),values(first,'margin'),values(second,'margin'),second.target]);";
    // When: both record-presence states render through the same retargeted extension.
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then: dependent record bindings run last and fall back to this render's own vector.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["base", "own", "on", "after"],
            ["base", "own", "on"],
            ["blue"],
            ["green"],
            ["2px"],
            ["3px"],
            "section"
        ])
    );
}

#[rstest]
#[serial]
fn static_callback_vectors_keep_existing_css_slots_when_extensions_are_retargeted(
    #[values(false, true)] single_css: bool,
) {
    // Given: static rules still carry observable per-render work through an existing CSS slot.
    let source = "const trace=[];const Base=styled.div((p,unused=(trace.push('base'),0))=>({color:'red'}));const Child=styled(Base)({padding:1});const Card=Child.withComponent('section');const defined=trace.slice();const App=ref=>jsx(Card,{css:{color:'blue'},ref,children:'child',className:'caller'});";
    let observation = r"const plain=render({target:Card,props:{}});const raw=App(7);const composed=render(raw);JSON.stringify([defined,trace,raw.target===Card,slots(raw).length,values(plain,'color'),values(composed,'color'),values(composed,'padding'),composed.target,composed.props.ref,composed.props.children,slots(composed),composed.props.className.split(/\s+/).includes('caller')]);";
    // When: a plain render and an authored CSS-prop render share inherited bindings.
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then: the existing private slot is consumed without bypassing the empty-vector invocation.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["base", "base"],
            true,
            1,
            ["red"],
            ["blue"],
            ["4px"],
            "section",
            7,
            "child",
            [],
            true
        ])
    );
}
