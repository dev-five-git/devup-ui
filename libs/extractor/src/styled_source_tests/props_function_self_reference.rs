use super::super::execute_in_mode;
use crate::{ExtractOption, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::recursive(concat!("function rules(p){return {color:p.done?'red':rules(", "{", "done:true", "}", ").color};}"))]
#[case::name("function rules(p){return {color:rules.name==='rules'?p.color:'orange'};}")]
#[case::length("function rules(p){return {opacity:rules.length};}")]
#[case::default("function rules(p,other=rules){return {color:p.color};}")]
#[case::destructured_default("function rules({color=rules.name}){return {color};}")]
#[case::nested_arrow("function rules(p){return {color:(()=>rules(p).color)()};}")]
#[case::nested_function(
    "function rules(p){return {color:(function(){return rules(p).color;})()};}"
)]
#[case::leaf_capture("function rules(p){return {color:()=>rules(p).color};}")]
#[case::shorthand("function rules(p){return {color:({rules}).rules.name};}")]
#[case::computed_key("function rules(p){return {color:({[rules.name]:p.color}).rules};}")]
#[case::shadow_ends(
    "function rules(p){return {color:(()=>{{const rules='orange';}return rules.name;})()};}"
)]
#[case::catch_ends(
    "function rules(p){return {color:(()=>{try{}catch(rules){}return rules.name;})()};}"
)]
#[case::loop_ends(
    "function rules(p){return {color:(()=>{for(const rules of []){}return rules.name;})()};}"
)]
#[case::switch_ends(
    "function rules(p){return {color:(()=>{switch(0){case 0:const rules=0;break;}return rules.name;})()};}"
)]
#[case::body_var_does_not_shadow_default(
    "function rules(p,other=rules){return {color:(()=>{var rules='orange';return rules;})()};}"
)]
#[serial]
fn named_whole_rule_self_references_are_located_errors(
    #[case] callback: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: the reference resolves to the callback's own function binding.
    reset_class_map();
    reset_file_map();
    let source =
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({callback});");
    // When: the immediate-object whole-rule callback is lowered.
    let result = extract(
        "self-reference.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then: the original callback location and binding identify the unsupported construct.
    let error = match result {
        Ok(output) => panic!("self-reference must be refused: {output:?}"),
        Err(error) => error.to_string(),
    };
    assert!(error.starts_with("self-reference.ts:2:23:"), "{error}");
    assert!(error.contains("cannot use `"), "{error}");
    assert!(error.contains("function rules("), "{error}");
    assert!(
        error.contains("callback `rules` refers to itself; recursion is not compiled"),
        "{error}"
    );
}

#[rstest]
#[case::named_nonself("function rules(p){return {color:p.color};}")]
#[case::parameter("function rules(rules){return {color:rules.color};}")]
#[case::destructured_parameter(concat!("function rules(", "{", "color:rules", "}", "){return {color:rules};}"))]
#[case::member_name("function rules(p){return {color:({rules:p.color}).rules};}")]
#[case::method_name("function rules(p){return {color:({rules(){return p.color;}}).rules()};}")]
#[case::nested_parameter("function rules(p){return {color:((rules)=>rules)(p.color)};}")]
#[case::nested_function_name(
    "function rules(p){return {color:(function rules(){return p.color;})()};}"
)]
#[case::block_let("function rules(p){return {color:(()=>{let rules=p.color;return rules;})()};}")]
#[case::block_var("function rules(p){return {color:(()=>{{var rules=p.color;}return rules;})()};}")]
#[case::hoisted_var(
    "function rules(p){return {color:(()=>{rules=p.color;{var rules;}return rules;})()};}"
)]
#[case::loop_binding(
    "function rules(p){return {color:(()=>{for(const rules of [p.color])return rules;})()};}"
)]
#[case::switch_binding(
    "function rules(p){return {color:(()=>{switch(0){case 0:const rules=p.color;return rules;}})()};}"
)]
#[case::default_local("function rules(p,other=((rules)=>rules)(0)){return {color:p.color};}")]
#[case::ordinary_property(
    "{color:function rules(p){'use strict';return rules.length===1&&rules.name==='rules'?p.color:'orange';}}"
)]
#[serial]
fn unrelated_bindings_and_property_names_keep_their_values(
    #[case] callback: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: same-spelled identifiers belong to local bindings or property names.
    let source = format!("const Card=styled.div({callback});");
    // When: the generated renderer evaluates two different props objects.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const first=render({target:Card,props:{color:'red'}});const second=render({target:Card,props:{color:'blue'}});JSON.stringify([values(first,'color'),values(second,'color')]);",
        single_css,
    );
    // Then: no textual-name refusal or semantic retargeting changes the local values.
    assert_eq!(observed, serde_json::json!([["red"], ["blue"]]));
}

#[rstest]
#[serial]
fn catch_binding_keeps_its_local_value_without_referring_to_the_callback(
    #[values(false, true)] single_css: bool,
) {
    // Given: a catch parameter, not the callback binding, owns the reference.
    let source = "const Card=styled.div(function rules(p){return {color:(()=>{try{throw p.color;}catch(rules){return rules;}})()};});";
    // When: the compiled callback runs with two distinct actual props.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const first=render({target:Card,props:{color:'red'}});const second=render({target:Card,props:{color:'blue'}});JSON.stringify([values(first,'color'),values(second,'color')]);",
        single_css,
    );
    // Then: catch-local references retain their authored value, not the function binding.
    assert_eq!(observed, serde_json::json!([["red"], ["blue"]]));
}
