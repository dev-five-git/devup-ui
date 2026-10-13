use super::*;

#[test]
#[serial]
fn retarget_when_order_is_captured_reuses_values_across_chains() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; let active=true; const config={get active(){trace.push('order');return active}}; const Base=styled('div',{styleOrder:config.active?2:3,color:'red'}); active=false; const Section=Base.withComponent('section'); const Span=Section.withComponent('span'); const a=Span({},null);";
    let actual = whole::evaluate(source, "[a.type,a.props.className]");
    assert_eq!(actual.trace, serde_json::json!(["order"]));
    assert_eq!(actual.element[0], "span");
    assert!(
        actual.element[1]
            .as_str()
            .required("retargeted component must emit classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn captured_inheritance_when_layers_differ_keeps_each_cascade_key() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const flag=()=>(trace.push('order'),true); const Base=styled('div',{styleOrder:flag()?2:3,color:'red'}); const Child=styled(Base,{styleOrder:7,color:'blue'}); const a=Child({},null);";
    let actual = whole::evaluate(source, "a.props.className");
    assert_eq!(actual.trace, serde_json::json!(["order"]));
    let classes = actual
        .element
        .as_str()
        .required("inherited different layers must emit classes");
    assert!(
        classes.contains("--2-") && classes.contains("--7-"),
        "{classes}"
    );
}

#[test]
#[serial]
fn captured_inheritance_when_keys_match_composes_instead_of_nesting() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const flag=()=>(trace.push('order'),true); const Base=styled('div',{styleOrder:flag()?2:3,color:'red'}); const Child=styled(Base,{styleOrder:2,color:'blue'}); const a=Child({as:'section'},null);";
    let actual = whole::evaluate(source, "[a.type,a.props.className]");
    assert_eq!(actual.trace, serde_json::json!(["order"]));
    assert_eq!(actual.element[0], "section");
    let classes = actual.element[1]
        .as_str()
        .required("matching inherited keys must emit classes");
    assert!(!classes.contains("-red--"), "{classes}");
    assert!(classes.contains("-blue--"), "{classes}");
}

#[test]
#[serial]
fn styled_callback_when_later_part_replaces_its_key_still_runs_once() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const flag=(on)=>(trace.push(on),on); const Card=styled.div(props=>({styleOrder:flag(props.active)?2:2,color:'red'}),{styleOrder:2,color:'blue'}); const a=Card({active:true},null);";
    let actual = whole::evaluate(source, "a.props.className");
    assert_eq!(actual.trace, serde_json::json!([true]));
    let classes = actual
        .element
        .as_str()
        .required("overridden callback must emit classes");
    assert!(
        !classes.contains("-red--") && classes.contains("-blue--"),
        "{classes}"
    );
}

#[test]
#[serial]
fn mixed_styled_parts_when_a_condition_is_false_preserve_previous_styles() {
    let source = "import {css,styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const base=css({styleOrder:2,color:'red'}); const flag=()=>(trace.push('branch'),false); const external='external'; const Card=styled.div(base,flag()&&[{styleOrder:2,color:'blue'},external],{styleOrder:7,color:'green'}); const a=Card({},null);";
    let actual = whole::evaluate(source, "a.props.className");
    assert_eq!(actual.trace, serde_json::json!(["branch"]));
    let classes = actual
        .element
        .as_str()
        .required("inactive mixed parts must keep classes");
    assert!(
        classes.contains("-red--") && classes.contains("--7-"),
        "{classes}"
    );
    assert!(
        !classes.contains("-blue--") && !classes.contains("external"),
        "{classes}"
    );
}

#[test]
#[serial]
fn wrapped_css_when_its_style_is_unconditional_retains_provenance() {
    let source = "import {css} from '@devup-ui/react'; const flag=()=>(trace.push('order'),true); const base=css({styleOrder:flag()?2:2,color:'red'}); const a=css(base,{styleOrder:2,color:'blue'});";
    let actual = whole::evaluate(source, "a");
    assert_eq!(actual.trace, serde_json::json!(["order"]));
    let classes = actual
        .element
        .as_str()
        .required("wrapped CSS must retain its class string");
    assert!(
        !classes.contains("-red--") && classes.contains("-blue--"),
        "{classes}"
    );
}

#[test]
#[serial]
fn retarget_when_attrs_close_over_a_factory_value_preserves_the_original_closure() {
    let source = concat!(
        "import {styled} from '@devup-ui/react'; const __devupForwardRef=render=>render; function make(label,active){const flag=()=>(trace.push(label),active); const Base=styled('div').attrs(()=>({",
        "id:label,className:'attrs',style:{zIndex:7}}))({styleOrder:flag()?2:3,color:'red'}); function change(){const label='shadow'; const Target=Base.withComponent('span');return Target}return change()} const A=make('A',true);const B=make('B',false);const a=A({className:'caller',style:{zIndex:9}},'refA');const b=B({},'refB');"
    );
    let actual = whole::evaluate(
        source,
        "[a.type,a.props.id,a.props.className,a.props.style.zIndex,a.props.ref,b.props.id,b.props.className,b.props.ref]",
    );
    assert_eq!(actual.trace, serde_json::json!(["A", "B"]));
    assert_eq!(actual.element[0], "span");
    assert_eq!(actual.element[1], "A");
    assert!(
        actual.element[2]
            .as_str()
            .required("attrs must emit a class string")
            .contains("attrs caller")
    );
    assert_eq!(actual.element[3], 7);
    assert_eq!(actual.element[4], "refA");
    assert_eq!(actual.element[5], "B");
    assert!(
        actual.element[6]
            .as_str()
            .required("second captured component must emit classes")
            .contains("--3-")
    );
    assert_eq!(actual.element[7], "refB");
}

#[test]
#[serial]
fn retarget_when_targets_are_effectful_freezes_each_target_before_render() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=render=>render; const flag=()=>(trace.push('order'),true);const target={get tag(){trace.push('target');return 'aside'}}; const Base=styled('div',{styleOrder:flag()?2:3,color:'red'});const Target=Base.withComponent(target.tag);const Child=styled(Target,{backgroundColor:'blue'});const a=Child({},null);const b=Child({},null);";
    let actual = whole::evaluate(source, "[a.type,b.type]");
    assert_eq!(actual.trace, serde_json::json!(["order", "target"]));
    assert_eq!(actual.element, serde_json::json!(["aside", "aside"]));
}

#[test]
#[serial]
fn retarget_when_native_and_custom_targets_differ_recomputes_read_prop_forwarding() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=render=>render; const Custom=()=>null;const Base=styled('div',props=>({styleOrder:props.secret?2:3,color:'red'}));const Native=Base.withComponent('button');const Component=Base.withComponent(Custom);const a=Native({secret:true,theme:'theme',id:'native'},null);const b=Component({secret:true,theme:'theme',id:'custom',forwardedAs:'a'},null);";
    let actual = whole::evaluate(
        source,
        "[a.props.secret===undefined,a.props.theme===undefined,b.type===Custom,b.props.secret,b.props.theme,b.props.as]",
    );
    assert_eq!(
        actual.element,
        serde_json::json!([true, true, true, true, "theme", "a"])
    );
}

#[test]
#[serial]
fn styled_callback_when_it_reads_special_props_receives_the_full_attrs_context() {
    let source = "import {styled} from '@devup-ui/react';const __devupForwardRef=render=>render;const Card=styled('div',props=>({styleOrder:props.className==='caller'&&props.as==='section'&&props.style.zIndex===3?2:3,color:'red'}));const a=Card({className:'caller',as:'section',style:{zIndex:3}},null);";
    let actual = whole::evaluate(source, "[a.type,a.props.className]");
    assert_eq!(actual.element[0], "section");
    assert!(
        actual.element[1]
            .as_str()
            .required("special-prop callback must emit classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn block_callback_when_it_has_a_prelude_keeps_effects_at_render_time() {
    let source = concat!(
        "import {styled} from '@devup-ui/react';const __devupForwardRef=render=>render;const Card=styled('div',function(props){trace.push('render');return {",
        "styleOrder:props.active?2:3,color:'red'}});const a=Card({active:true},null);const b=Card({active:false},null);"
    );
    let actual = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(actual.trace, serde_json::json!(["render", "render"]));
    assert!(
        actual.element[0]
            .as_str()
            .required("active block callback must emit classes")
            .contains("--2-")
    );
    assert!(
        actual.element[1]
            .as_str()
            .required("inactive block callback must emit classes")
            .contains("--3-")
    );
}

#[test]
#[serial]
fn recursive_order_when_used_in_jsx_and_runtime_has_matching_selection() {
    for view in [
        "<Box styleOrder={flag('outer',true)?flag('inner',false)?2:3:4} color='red'/>",
        "jsx(Box,{styleOrder:flag('outer',true)?flag('inner',false)?2:3:4,color:'red'})",
    ] {
        let source = format!(
            "{BOX}{JSX_RUNTIME}const flag=(name,on)=>(trace.push(name),on);const a={view};"
        );
        let actual = whole::evaluate(&source, "a.props.className");
        assert_eq!(actual.trace, serde_json::json!(["outer", "inner"]));
        assert!(
            actual
                .element
                .as_str()
                .required("recursive order must emit classes")
                .contains("--3-")
        );
        assert!(
            !actual
                .element
                .as_str()
                .required("recursive order must remain a class string")
                .contains("--2-")
        );
    }
}
