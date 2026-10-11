use super::*;
use serial_test::serial;

#[test]
#[serial]
fn saved_css_when_source_flag_changes_composes_the_constructed_order() {
    for initial in [true, false] {
        let source = format!(
            "import {{css}} from '@devup-ui/react'; let active={initial}; const config={{get active(){{trace.push('construct');return active}}}}; const base:string=(css({{styleOrder:config.active?1:2,color:'red'}})); const alias=base; active=false; const a=css(alias,{{styleOrder:1,color:'blue'}});"
        );
        let actual = whole::evaluate(&source, "[typeof base,a]");
        assert_eq!(actual.trace, serde_json::json!(["construct"]));
        assert_eq!(actual.element[0], "string");
        let classes = actual.element[1]
            .as_str()
            .required("saved CSS composition must emit a class string");
        assert!(classes.contains("color-0-blue--1-a"), "{classes}");
        assert_eq!(classes.contains("red"), !initial, "{classes}");
        if !initial {
            assert!(classes.contains("color-0-red--2-a"), "{classes}");
        }
    }
}

#[test]
#[serial]
fn inline_css_when_a_later_part_covers_it_keeps_source_effects() {
    let actual = whole::evaluate(
        "import {css} from '@devup-ui/react'; const flag=()=> (trace.push('inline'),true); const a=css(css({styleOrder:flag()?1:2,color:'red'}),{styleOrder:1,color:'blue'});",
        "a",
    );
    assert_eq!(actual.trace, serde_json::json!(["inline"]));
    assert_eq!(
        actual
            .element
            .as_str()
            .required("covered inline CSS must emit a class string")
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["color-0-blue--1-a"]
    );
}

#[test]
#[serial]
fn css_callback_when_composed_in_styled_keeps_closure_and_render_timing() {
    let actual = whole::evaluate(
        "import {css,styled} from '@devup-ui/react'; const __devupForwardRef=render=>render; const make=()=> { const check=active=>(trace.push(active),active); return styled('div')(p=>css({styleOrder:check(p.active)?1:2,color:'red'}),{styleOrder:1,color:'blue'}); }; const Card=make(); const a=Card({active:true},null); const b=Card({active:false},null);",
        "[a.props.className,b.props.className]",
    );
    assert_eq!(actual.trace, serde_json::json!([true, false]));
    assert_eq!(
        actual.element[0]
            .as_str()
            .required("active CSS callback must emit a class string")
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["color-0-blue--1-a"]
    );
    assert!(
        actual.element[1]
            .as_str()
            .required("inactive CSS callback must emit a class string")
            .contains("color-0-red--2-a")
    );
}

#[test]
#[serial]
fn opaque_collision_when_equal_to_a_generated_result_remains_opaque() {
    let actual = whole::evaluate(
        "import {css} from '@devup-ui/react'; const external='color-0-red--1-a'; const mixed=css(external,{styleOrder:1,color:'blue'}); const a=css(mixed,{styleOrder:1,color:'green'});",
        "a",
    );
    assert!(
        actual
            .element
            .as_str()
            .required("opaque collision composition must remain a string")
            .contains("color-0-red--1-a")
    );
    assert!(
        actual
            .element
            .as_str()
            .required("opaque composition must retain its class string")
            .contains("color-0-blue--1-a")
    );
}

#[test]
#[serial]
fn conditional_alias_when_the_choice_changes_keeps_its_saved_primitive() {
    let source = "import {css} from '@devup-ui/react'; let selected=true; const choose=()=>(trace.push('alias'),selected); const red=css({styleOrder:2,color:'red'}); const blue=css({styleOrder:1,color:'blue'}); const alias:string=choose()?red:blue; selected=false; const a=css(alias,{styleOrder:1,color:'green'});";
    let actual = whole::evaluate(source, "[typeof alias,a]");
    assert_eq!(actual.trace, serde_json::json!(["alias"]));
    assert_eq!(actual.element[0], "string");
    let mut classes: Vec<_> = actual.element[1]
        .as_str()
        .required("saved conditional alias must emit a class string")
        .split_whitespace()
        .collect();
    classes.sort_unstable();
    assert_eq!(classes, vec!["color-0-green--1-a", "color-0-red--2-a"]);
}

#[test]
#[serial]
fn finite_class_array_when_stored_then_composed_keeps_its_source_values() {
    let source = "import {css} from '@devup-ui/react'; let active=true; const flag=()=>(trace.push('array'),active); const parts=[css({styleOrder:flag()?1:2,color:'red'}),css({styleOrder:2,background:'black'})]; active=false; const a=css(parts,{styleOrder:1,color:'blue'});";
    let actual = whole::evaluate(source, "a");
    assert_eq!(actual.trace, serde_json::json!(["array"]));
    let mut classes: Vec<_> = actual
        .element
        .as_str()
        .required("saved finite array must emit a class string")
        .split_whitespace()
        .collect();
    classes.sort_unstable();
    assert_eq!(
        classes,
        vec!["background-0-black--2-a", "color-0-blue--1-a"]
    );
}

#[test]
#[serial]
fn imported_finite_css_when_source_state_changes_uses_producer_names() {
    let producer = "import {css} from '@devup-ui/react'; let active=true; const config={get active(){trace.push('producer');return active}}; export const base:string=css({styleOrder:config.active?1:2,color:'red'}); active=false;";
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/producer.ts".to_string(),
            code: producer.to_string(),
        })
    };
    reset_class_map();
    reset_file_map();
    css::debug::set_debug(true);
    let source = "import {css} from '@devup-ui/react'; import {base} from './producer'; const a=css(base,{styleOrder:1,color:'blue'});";
    let consumer = extract_with_modules(
        "/consumer.ts",
        source,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .required("imported finite CSS consumer must compile");
    let producer = extract_without_source_map("/producer.ts", producer, ExtractOption::default())
        .required("finite CSS producer must compile");
    css::debug::set_debug(false);
    let actual = whole::evaluate_code(&format!("{}\n{}", producer.code, consumer.code), "a");
    assert_eq!(actual.trace, serde_json::json!(["producer"]));
    let classes = actual
        .element
        .as_str()
        .required("imported CSS composition must emit a class string");
    assert!(!classes.contains("red"), "{classes}");
    assert!(classes.contains("color-0-blue--1-"), "{classes}");
}

#[test]
#[serial]
fn tagged_mixins_when_ordered_objects_and_css_callbacks_compose_are_structured() {
    let source = "import {css,styled} from '@devup-ui/react'; const __devupForwardRef=render=>render; let active=true; const base=css({styleOrder:active?1:2,color:'red'}); active=false; const Card=styled.div`${base}${p=>css({styleOrder:p.active?1:2,color:'blue'})}`; const a=Card({active:true},null);";
    let actual = whole::evaluate(source, "a.props.className");
    assert_eq!(
        actual
            .element
            .as_str()
            .required("tagged mixin must emit a class string")
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["color-0-blue--1-a"]
    );
}

#[test]
#[serial]
fn saved_empty_css_when_used_as_a_lazy_fallback_keeps_source_guards_once() {
    let source = "import {css} from '@devup-ui/react'; const flag=()=>(trace.push('construct'),false); const base=css(flag()?{styleOrder:1,color:'red'}:null); const fallback=()=>(trace.push('fallback'),true); const a=css(base||css({styleOrder:fallback()?1:2,color:'blue'}));";
    let actual = whole::evaluate(source, "a");
    assert_eq!(
        actual.trace,
        serde_json::json!(["construct", "fallback"]),
        "{}",
        code(source)
    );
    assert_eq!(
        actual
            .element
            .as_str()
            .required("lazy CSS fallback must emit a class string")
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["color-0-blue--1-a"]
    );
}

#[test]
#[serial]
fn finite_keyed_rules_when_the_key_matches_replace_only_matching_order() {
    for key in ["a", "b", "missing"] {
        let source = format!(
            "import {{css}} from '@devup-ui/react'; const key={{toString(){{trace.push('key');return '{key}'}}}}; const a=css({{styleOrder:1,color:'red'}},{{styleOrder:1,color:{{a:'blue',b:'green'}}[key]}});"
        );
        let actual = whole::evaluate(&source, "a");
        assert_eq!(actual.trace, serde_json::json!(["key"]));
        let expected = match key {
            "a" => "color-0-blue--1-a",
            "b" => "color-0-green--1-a",
            "missing" => "color-0-red--1-a",
            _ => unreachable!(),
        };
        assert_eq!(
            actual
                .element
                .as_str()
                .required("finite keyed rule must emit a class string")
                .split_whitespace()
                .collect::<Vec<_>>(),
            vec![expected]
        );
    }
}

#[test]
#[serial]
fn callbacks_when_css_returns_early_or_falls_through_keep_inactive_styles() {
    let source = "import {css,styled} from '@devup-ui/react'; const __devupForwardRef=render=>render; const Card=styled.div({styleOrder:1,color:'red'},p=>{trace.push('render');if(p.active)return css({styleOrder:1,color:'blue'});}); const a=Card({active:true},null); const b=Card({active:false},null);";
    let actual = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(actual.trace, serde_json::json!(["render", "render"]));
    assert_eq!(
        actual.element[0]
            .as_str()
            .required("early CSS callback return must emit a class string")
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["color-0-blue--1-a"]
    );
    assert_eq!(
        actual.element[1]
            .as_str()
            .required("inactive CSS callback must keep a class string")
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["color-0-red--1-a"]
    );
}
