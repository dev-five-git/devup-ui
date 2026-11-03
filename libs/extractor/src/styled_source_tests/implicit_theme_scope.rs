use super::execute_in_mode;
use crate::{ExtractOption, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::catch(
    "(...all)=>({color:(()=>{try{throw 0;}catch(all){}return all[0].theme.colors.brand;})()})"
)]
#[case::for_of(
    "(...all)=>({color:(()=>{for(const all of []){}return all[0]['theme'].colors.brand;})()})"
)]
#[case::for_let(
    "(...all)=>({color:(()=>{for(let all=0;all<0;all++){}return all[0].theme.colors.brand;})()})"
)]
#[case::for_in(
    "(...all)=>({color:(()=>{for(const all in {}){}return all[0].theme.colors.brand;})()})"
)]
#[case::switch(
    "(...all)=>({color:(()=>{switch(0){case 0:const all=[];break;}return all[0].theme.colors.brand;})()})"
)]
#[case::block_let("(...all)=>({color:(()=>{{let all=[];}return all[0].theme.colors.brand;})()})")]
#[case::block_const(
    "(...all)=>({color:(()=>{{const all=[];}return all[0].theme.colors.brand;})()})"
)]
#[case::switch_discriminant(
    "(...all)=>({color:(()=>{switch(all[0].theme.colors.brand){case 'red':const all=[];}return 'black';})()})"
)]
#[serial]
fn outer_props_theme_roots_return_after_nested_lexical_scopes(
    #[case] rules: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: every local shadow ends before the actual-props theme read.
    reset_class_map();
    reset_file_map();
    let source =
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({rules});");
    // When: extraction determines the owner of the rest binding after each scope.
    let result = extract(
        "implicit-scope-boundary.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then: a completed nested scope cannot suppress the existing theme error.
    let error = match result {
        Ok(output) => panic!("outer props theme read must remain refused: {output:?}"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.starts_with("implicit-scope-boundary.ts:2:"),
        "{error}"
    );
    assert!(
        error.contains("a theme read must be a path of names or literal keys"),
        "{error}"
    );
}

#[rstest]
#[case::block_var(
    "(...all)=>({color:(()=>{{var all=[{theme:{colors:{brand:'orange'}}}];}return all[0].theme.colors.brand;})()})"
)]
#[case::if_var(
    "(...all)=>({color:(()=>{if(true){var all=[{theme:{colors:{brand:'orange'}}}];}return all[0].theme.colors.brand;})()})"
)]
#[case::catch_local(
    "(...all)=>({color:(()=>{try{throw [{theme:{colors:{brand:'orange'}}}];}catch(all){return all[0].theme.colors.brand;}})()})"
)]
#[case::for_local(
    "(...all)=>({color:(()=>{for(const all of [[{theme:{colors:{brand:'orange'}}}]])return all[0].theme.colors.brand;return 'black';})()})"
)]
#[serial]
fn local_theme_values_keep_their_binding_owner_across_var_catch_and_loop_scopes(
    #[case] rules: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: local data is orange while the actual props are red.
    let source = format!("const Card=styled.div({rules});");
    // When: the emitted dynamic value executes in its authored scope.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "JSON.stringify(values(render({target:Card,props:{color:'red'}}),'color'));",
        single_css,
    );
    // Then: local objects are not mistaken for the callback's actual props.
    assert_eq!(observed, serde_json::json!(["orange"]));
}

#[rstest]
#[case::root_rest_var("(...all)=>{var all;return {color:all[0].theme.colors.brand};}")]
#[case::root_rest_block_var("(...all)=>{{var all;}return {color:all[0].theme.colors.brand};}")]
#[case::root_normal_rest_var("function(...all){var all;return {color:all[0].theme.colors.brand};}")]
#[case::root_normal_rest_block_var(
    "function(...all){{var all;}return {color:all[0].theme.colors.brand};}"
)]
#[serial]
fn callback_statements_keep_the_existing_admission_error(
    #[case] rules: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: statements before the return are outside the whole-rule callback contract.
    reset_class_map();
    reset_file_map();
    let source =
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({rules});");
    // When: admission runs before theme-root analysis.
    let result = extract(
        "implicit-scope-boundary.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then: the established located explicit-object refusal still takes precedence.
    let error = match result {
        Ok(output) => panic!("unsupported callback statements must remain refused: {output:?}"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.starts_with("implicit-scope-boundary.ts:2:"),
        "{error}"
    );
    assert!(
        error.contains("a style function of the props must give one rule object at once"),
        "{error}"
    );
}
