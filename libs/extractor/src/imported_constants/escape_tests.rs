use super::exact_tests::{extracted, static_values};
use rstest::rstest;

#[rstest]
#[case("watch(made);")]
#[case("made.custom();")]
#[case("made.join();")]
#[case("made['has']();")]
#[case("made?.valueOf();")]
#[case("const alias=made; watch(alias);")]
#[case("const box={inner:[made]}; watch(box);")]
#[case("const box={inner:[made]};const other=box;watch(other);")]
#[case("watch({inner:[made]});")]
#[case("let alias=made; alias.p=2;")]
#[case("const { inner: alias }={ inner: made }; watch(alias);")]
#[case("made.p=2;")]
#[case("made.p+=2;")]
#[case("made.p ||= 2;")]
#[case("made.p++;")]
#[case("delete made.p;")]
#[case("Object.assign(made,{ p: 2 });")]
#[case("Object.defineProperty(made,'p',{ value: 2 });")]
#[case("Object.defineProperties(made,{p:{value:2}});")]
#[case("const writer=()=>{made.p=2}; writer();")]
#[case("watch(()=>{made.p=2});")]
#[case("const writer={go:()=>{made.p=2}}; writer.go();")]
#[case("[made.p]=[2];")]
#[case("for(made.p of [2]){}")]
#[case("Object.setPrototypeOf(made,{});")]
#[case("const expose=()=>made;watch(expose);")]
#[case("const box={expose:()=>made};watch(box.expose);")]
#[serial_test::serial]
fn factory_read_when_its_object_may_change_is_not_folded(#[case] hazard: &str) {
    // Given
    let declarations =
        format!("const make=()=>({{p:1}});const made=make();{hazard}const f=()=>made.p;");
    // When
    let result = extracted(
        &format!("import {{css}} from '@devup-ui/react';{declarations}css({{p:f()}});"),
        "",
    );
    // Then
    assert!(result.is_err(), "{hazard}: {result:?}");
}

#[test]
#[serial_test::serial]
fn factory_read_when_escaped_reports_the_origin_and_consumer() {
    let source = r"import {css} from '@devup-ui/react';
const make=()=>({ p: 1 });
const made=make();
watch(made);
const f=()=>made.p;
css({p:f()});";
    let error = extracted(source, "")
        .err()
        .unwrap_or_else(|| panic!("escaped factory read must fail"));
    assert!(error.contains("/src/App.tsx:6:"), "{error}");
    assert!(error.contains("/src/App.tsx:4:"), "{error}");
    assert!(error.contains("`made`"), "{error}");
}

#[test]
#[serial_test::serial]
fn element_read_when_factory_object_escapes_keeps_the_runtime_expression() {
    let source = r"import {Box} from '@devup-ui/react';const make=()=>({ p: 1 });const made=make();watch(made);const f=()=>made.p;export const a=<Box p={f()}/>;";
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    assert!(output.code.contains("f()"), "{}", output.code);
    assert!(output.code.contains("--"), "{}", output.code);
    assert_eq!(static_values(&output), Vec::<String>::new());
}

#[rstest]
#[case("")]
#[case("watch(made.p);")]
#[case("JSON.stringify(made);")]
#[case("String(made);")]
#[case("Number(made);")]
#[case("Object.keys(made);")]
#[case("Object.freeze(made);watch(made);")]
#[case("const alias=made;Object.freeze(alias);watch(made);")]
#[case("const frozen=Object.freeze(made);watch(made);")]
#[case("Object['freeze'](made);watch(made);")]
#[case("JSON['stringify'](made);")]
#[serial_test::serial]
fn factory_read_when_proven_safe_stays_static(#[case] use_site: &str) {
    let source = format!(
        "import {{css}} from '@devup-ui/react';const make=()=>({{p:1}});const made=make();{use_site}css({{p:made.p}});"
    );
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
}

#[rstest]
#[case("watch(made);Object.freeze(made);")]
#[case("if(flag)Object.freeze(made);watch(made);")]
#[case("const freeze=()=>Object.freeze(made);watch(made);")]
#[case("Object.seal(made);watch(made);")]
#[case("Object.preventExtensions(made);watch(made);")]
#[case("made.p=2;Object.freeze(made);watch(made);")]
#[serial_test::serial]
fn freeze_when_not_before_all_hazards_does_not_restore_exactness(#[case] hazard: &str) {
    let source = format!(
        "import {{css}} from '@devup-ui/react';const make=()=>({{p:1}});const made=make();{hazard}css({{p:made.p}});"
    );
    assert!(extracted(&source, "").is_err(), "{hazard}");
}

#[rstest]
#[case("const JSON={ stringify: watch };JSON.stringify(made);")]
#[case("const Object={ keys: watch };Object.keys(made);")]
#[case("const String=watch;String(made);")]
#[case("const Number=watch;Number(made);")]
#[case("JSON.stringify=watch;JSON.stringify(made);")]
#[case("Object.keys=watch;Object.keys(made);")]
#[case("JSON.stringify(made,watch);")]
#[case("Object.defineProperty(Object,'freeze',{ value: watch });Object.freeze(made);")]
#[case("Object=other;Object.keys(made);")]
#[case("Array=other;console.log(made);")]
#[case("JSON[key](made);")]
#[case("JSON[1](made);")]
#[case("({JSON}).JSON.stringify(made);")]
#[case("watch(Array);console.log(made);")]
#[serial_test::serial]
fn readonly_callee_when_identity_or_callbacks_are_uncertain_blocks_folding(#[case] hazard: &str) {
    let source = format!(
        "import {{css}} from '@devup-ui/react';const make=()=>({{p:1}});const made=make();{hazard}css({{p:made.p}});"
    );
    assert!(extracted(&source, "").is_err(), "{hazard}");
}

#[rstest]
#[case("css({p:f()});")]
#[case("globalCss({body:{p:f()}});")]
#[case("keyframes({from:{p:f()}});")]
#[case("styled.div`padding:${f()}px;`;")]
#[serial_test::serial]
fn build_only_consumer_when_factory_object_escapes_rejects_the_value(#[case] consumer: &str) {
    let source = format!(
        "import {{css,globalCss,keyframes,styled}} from '@devup-ui/react';const make=()=>({{p:1}});const made=make();watch(made);const f=()=>made.p;{consumer}"
    );
    assert!(extracted(&source, "").is_err(), "{consumer}");
}

#[test]
#[serial_test::serial]
fn shallow_freeze_when_nested_children_escape_preserves_only_own_scalar_slots() {
    let source = r"import {Box} from '@devup-ui/react';const make=()=>({p:1,child:{ p: 2 }});const made=Object.freeze(make());watch(made);export const a=<Box p={made.p} m={made.child.p}/>;";
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
    assert!(output.code.contains("made.child.p"), "{}", output.code);
    assert!(output.code.contains("--"), "{}", output.code);
}

#[rstest]
#[case("const made={p:1,map(){this.p=2}};made.map();")]
#[case("const made={p:1,join(){this.p=2}};made.join();")]
#[case("const made={p:1,get toJSON(){this.p=2;return ()=>0}};JSON.stringify(made);")]
#[case("const made={p:1,valueOf(){this.p=2}};Number(made);")]
#[case("const made={p:1,toString(){this.p=2}};String(made);")]
#[case("const made={p:1,set x(value){this.p=value}};made.x=2;")]
#[serial_test::serial]
fn hooks_when_called_are_not_readonly_proofs(#[case] declarations: &str) {
    let source = format!("import {{css}} from '@devup-ui/react';{declarations}css({{p:made.p}});");
    assert!(extracted(&source, "").is_err(), "{declarations}");
}

#[rstest]
#[case("const made={ p: 1 };const copied=made.p;watch(copied);css({ p: copied });")]
#[case("const made={ p: 1 };const copy={...made};watch(copy);css({ p: made.p });")]
#[case("const made=[1,2];const copy=[...made];watch(copy);css({p:made[0]});")]
#[case("const made={ p: 1 };function other(){const made={ p: 2 };watch(made)}css({ p: made.p });")]
#[case("const make=function(){return {p:1}};const made=make();css({p:made.p});")]
#[case("function make(){return {p:1}}const made=make();css({p:made.p});")]
#[case("const make=()=>{return {p:1}};const made=make();css({p:made.p});")]
#[case("const made={ p: 1 };const copied=made.p;watch(made);css({ p: copied });")]
#[case(
    "const make=()=>({ p: 1 });const made=make();const copied=made.p;watch(made);css({ p: copied });"
)]
#[case(
    "const make=()=>({ p: 1 });const made=Object.freeze(make());watch(made);const f=()=>made.p;css({p:f()});"
)]
#[case("const make=()=>[1,2];const made=Object.freeze(make());watch(made);css({p:made[0]});")]
#[serial_test::serial]
fn exact_values_when_independent_of_mutable_aliases_stay_static(#[case] declarations: &str) {
    let source = format!("import {{css}} from '@devup-ui/react';{declarations}");
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
}

#[rstest]
#[case(
    "const made={ p: 1 };const make=()=>({made});const box=make();watch(box);css({ p: made.p });"
)]
#[case("const made={ p: 1 };const make=()=>made;watch(make());css({ p: made.p });")]
#[case("const made={ p: 1 };const make=()=>made;make().p=2;css({ p: made.p });")]
#[case("const made={ p: 1 };const make=()=>made;const alias=make();alias.p=2;css({ p: made.p });")]
#[case("const made={ p: 1 };const a={made};const b={a};a.b=b;watch(b);css({ p: made.p });")]
#[case("const made={ p: 1 };watch(made);const copied=made.p;css({ p: copied });")]
#[case(
    "const made={ p: 1 };writer();const copied=made.p;function writer(){made.p=2}css({ p: copied });"
)]
#[serial_test::serial]
fn indirect_references_when_their_returned_container_escapes_are_not_static(
    #[case] declarations: &str,
) {
    let source = format!("import {{css}} from '@devup-ui/react';{declarations}");
    assert!(extracted(&source, "").is_err(), "{declarations}");
}

#[rstest]
#[case("let make=()=>({ p: 1 });make=()=>({ p: 5 });const made=make();")]
#[case("function make(){return { p: 1 }}make=()=>({ p: 5 });const made=make();")]
#[case("let make=()=>({ p: 1 });const made=make();make=()=>({ p: 5 });")]
#[serial_test::serial]
fn factory_when_its_binding_can_be_reassigned_is_not_evaluated(#[case] declarations: &str) {
    let source = format!("import {{css}} from '@devup-ui/react';{declarations}css({{p:made.p}});");
    assert!(extracted(&source, "").is_err(), "{declarations}");
}

#[test]
#[serial_test::serial]
fn shallow_frozen_array_when_nested_elements_escape_preserves_only_scalar_elements() {
    // Given
    let source = "import { Box } from '@devup-ui/react';const make=()=>[1,{ p:2 }];const made=Object.freeze(make());watch(made);export const view=<Box p={made[0]} m={made[1].p}/>;";
    // When
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
    assert!(
        output.code.contains("made[1].p") && output.code.contains("--"),
        "{}",
        output.code
    );
}

#[test]
#[serial_test::serial]
fn shallow_freeze_when_an_overflowing_literal_leaves_partial_fields_keeps_the_known_scalar() {
    // Given
    let source = "import { Box } from '@devup-ui/react';const make=()=>({ p:1,child:{ p:2 },n:1e999 });const made=Object.freeze(make());watch(made);export const view=<Box p={made.p}/>;";
    // When
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
}
