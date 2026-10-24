use crate::{ExtractOption, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::distinct("function rules<T>(p:T){return {color:rules.name};}")]
#[case::same_name("function rules<rules>(p:rules){return {color:rules.name};}")]
#[case::default("function rules<rules>(p:rules,other=rules){return {color:'red'};}")]
#[case::destructured_default("function rules<rules>({color=rules.name}:rules){return {color};}")]
#[case::rest("function rules<rules>(...p:rules[]){return {color:rules.name};}")]
#[case::nested("function rules<rules>(p:rules){return {color:(()=>rules.name)()};}")]
#[case::nested_type("function rules(p){return {color:(<rules,>(p:rules)=>rules.name)(p)};}")]
#[serial]
fn value_self_reference_is_refused_when_type_names_overlap(
    #[case] callback: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: type bindings cannot shadow the callback's value identity.
    reset_class_map();
    reset_file_map();
    let source =
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({callback});");
    // When: the full extractor lowers the whole-rule callback.
    let result = extract(
        "generic-self-reference.tsx",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then: the original location reports the value self-reference, not a generic refusal.
    let error = match result {
        Ok(output) => panic!("value self-reference must be refused: {output:?}"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.starts_with("generic-self-reference.tsx:2:23:"),
        "{error}"
    );
    assert!(error.contains("cannot use `"), "{error}");
    assert!(
        error.contains("callback `rules` refers to itself; recursion is not compiled"),
        "{error}"
    );
}

#[rstest]
#[case::type_only("function rules<rules>(p:rules){return {color:'red'};}")]
#[case::type_query("function rules<T extends typeof rules>(p:T){return {color:'red'};}")]
#[case::parameter("function rules<rules>(rules:rules){return {color:rules.color};}")]
#[case::destructured(concat!("function rules<rules>(", "{", "color:rules", "}", ":rules){return ", "{", "color:rules", "}", ";}"))]
#[case::nested_parameter(
    "function rules<rules>(p:rules){return {color:((rules)=>rules)(p.color)};}"
)]
#[case::nested_name(
    "function rules<rules>(p:rules){return {color:(function rules(){return p.color;})()};}"
)]
#[case::block(
    "function rules<rules>(p:rules){return {color:(()=>{const rules=p.color;return rules;})()};}"
)]
#[case::default_shadow(
    "function rules<rules>(p:rules,other=((rules)=>rules)(0)){return {color:p.color};}"
)]
#[case::rest_shadow("function rules<rules>(...rules:rules[]){return {color:rules[0].color};}")]
#[serial]
fn callback_compiles_when_references_are_type_only_or_value_shadowed(
    #[case] callback: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: only type references or references to genuine local values use this spelling.
    reset_class_map();
    reset_file_map();
    let source =
        format!("import {{styled}} from '@devup-ui/react';const Card=styled.div({callback});");
    // When: the same full-extractor path processes a supported callback.
    let output = extract(
        "generic-controls.tsx",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("type-only names and value shadows must compile: {error}"));
    // Then: the callback survives lowering and produces actual extracted declarations.
    assert!(output.code.contains("function rules<"), "{}", output.code);
    assert_ne!(output.styles.len(), 0, "{output:?}");
}
