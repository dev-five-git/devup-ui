use super::execute_in_mode;
use crate::{ExtractOption, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::arguments("arguments[0].color")]
#[case::receiver("this===factoryReceiver?'orange':p.color")]
#[serial]
fn whole_rule_normal_functions_keep_invocation_scope(
    #[case] value: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: a normal factory's receiver and argument differ from callback inputs.
    let source = format!(
        "const receiver={{color:'orange'}};function make(factoryReceiver){{return styled.div(function(p){{return {{color:{value}}};}});}}const Card=make.call(receiver,receiver);"
    );
    let observation = "const first=render({target:Card,props:{color:'red'}});const second=render({target:Card,props:{color:'blue'}});JSON.stringify([values(first,'color'),values(second,'color')]);";
    // When: the extracted whole-rule callback executes in two actual renders.
    let observed = execute_in_mode("test.ts", &source, observation, single_css);
    // Then: arguments belong to the callback; direct invocation never inherits the factory receiver.
    assert_eq!(observed, serde_json::json!([["red"], ["blue"]]));
}

#[rstest]
#[serial]
fn nested_props_bindings_run_once_per_render_across_dynamic_leaves(
    #[values(false, true)] single_css: bool,
) {
    // Given: both nested bindings have changing getters shared by two dynamic leaves.
    let source = "const trace=[];let cReads=0;let mReads=0;const data={get c(){trace.push('c');return ++cReads===1?'red':'blue';},get m(){trace.push('m');return `${++mReads*4}px`;}};const Card=styled.div(({data:{c,m}})=>({color:c,margin:m}));const defined=trace.slice();";
    let observation = "const first=render({target:Card,props:{data}});const firstTrace=trace.slice();const second=render({target:Card,props:{data}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin')]);";
    // When: real extracted code binds the same getter-backed props on each render.
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then: each render binds c,m once, and both leaves use that same binding pass.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["c", "m"],
            ["c", "m", "c", "m"],
            ["red"],
            ["4px"],
            ["blue"],
            ["8px"]
        ])
    );
}

#[rstest]
#[case::property_props("{color:(p,unused)=>p.theme.colors[key]}")]
#[case::property_theme_alias("{color:({theme:palette},unused)=>palette.colors[key]}")]
#[case::whole_rule_props("(p,unused)=>({color:p.theme.colors[key]})")]
#[case::whole_rule_theme_alias("({theme:palette},unused)=>({color:palette.colors[key]})")]
#[serial]
fn additional_params_keep_dynamic_theme_paths_as_located_errors(
    #[case] rules: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: a runtime key and an unused second formal for either supported theme root.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{styled,css}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';\nconst make=key=>styled.div({rules});"
    );
    let option = ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    // When: extraction sees the unprovable theme path, without executing runtime CSS.
    let result = extract("props-theme.ts", &source, option);
    // Then: extra formals cannot turn the existing located theme refusal into a CSS variable.
    let error = match result {
        Ok(output) => panic!("dynamic theme paths must remain refused: {output:?}"),
        Err(error) => error.to_string(),
    };
    let Some((column, _)) = error
        .strip_prefix("props-theme.ts:2:")
        .and_then(|located| located.split_once(':'))
    else {
        panic!("theme refusal must locate the authored callback: {error}");
    };
    assert!(
        matches!(column.parse::<usize>(), Ok(column) if column > 0),
        "{error}"
    );
    assert!(error.contains("cannot use"), "{error}");
    assert!(error.contains("at build time:"), "{error}");
    assert!(
        error.contains("a theme read must be a path of names or literal keys"),
        "{error}"
    );
}

#[rstest]
#[case::normal("function(p){return{color:arguments[0].theme.colors[key]};}")]
#[case::normal_without_formals("function(){return{color:arguments[0]['theme'].colors[key]};}")]
#[case::rest("(...all)=>({color:all[0].theme.colors[key]})")]
#[case::property_normal("{color:function(p){return arguments[0].theme.colors[key];}}")]
#[case::property_rest("{color:(...all)=>all[0]['theme'].colors[key]}")]
#[serial]
fn implicit_props_roots_keep_unprovable_theme_reads_as_located_errors(
    #[case] rules: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: these callback forms receive the actual props as their first argument.
    reset_class_map();
    reset_file_map();
    let source =
        format!("import {{styled}} from '@devup-ui/react';\nconst make=key=>styled.div({rules});");
    // When: an unprovable theme path is reached through own arguments or a rest formal.
    let result = extract(
        "implicit-theme.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then: alternate access to the actual props cannot emit runtime theme access.
    let error = match result {
        Ok(output) => panic!("implicit props theme reads must remain refused: {output:?}"),
        Err(error) => error.to_string(),
    };
    assert!(error.starts_with("implicit-theme.ts:2:"), "{error}");
    assert!(
        error.contains("a theme read must be a path of names or literal keys"),
        "{error}"
    );
}

#[rstest]
#[case::arguments_string_zero("function(){return{color:arguments['0']['theme'].colors.brand};}")]
#[case::rest_string_zero("(...all)=>({color:all['0'].theme.colors.brand})")]
#[case::lexical_arguments("function(){return{color:(()=>arguments[0].theme.colors.brand)()};}")]
#[case::captured_rest("(...all)=>({color:(function(){return all[0].theme.colors.brand})()})")]
#[case::property_no_formals("{color:function(){return arguments['0'].theme.colors.brand;}}")]
#[case::property_rest_normal("{color:function(...all){return all['0']['theme'].colors.brand;}}")]
#[case::mixed_mapped(
    "function(p){return{background:p.theme.colors.brand,color:arguments[0].theme.colors.brand};}"
)]
#[serial]
fn implicit_theme_roots_remain_errors_inside_inheriting_scopes(
    #[case] rules: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: implicit roots are not new statically mapped theme roots, even with literal paths.
    reset_class_map();
    reset_file_map();
    let source =
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({rules});");
    // When: extraction visits original callback scope before rewriting its leaves.
    let result = extract(
        "implicit-scope.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then: the same theme refusal locates the authored callback.
    let error = match result {
        Ok(output) => panic!("implicit theme roots must not become runtime reads: {output:?}"),
        Err(error) => error.to_string(),
    };
    assert!(error.starts_with("implicit-scope.ts:2:"), "{error}");
    assert!(
        error.contains("a theme read must be a path of names or literal keys"),
        "{error}"
    );
}

#[rstest]
#[case::normal("function(){return{color:arguments[0].color};}", "red")]
#[case::rest("(...all)=>({color:all['0'].color})", "red")]
#[case::property_normal("{color:function(){return arguments['0'].color;}}", "red")]
#[case::property_rest("{color:(...all)=>all[0].color}", "red")]
#[case::factory_arguments("()=>({color:arguments[0].theme.colors.brand})", "orange")]
#[case::property_factory_arguments("{color:()=>arguments[0].theme.colors.brand}", "orange")]
#[case::nested_normal(
    "function(){return{color:(function(){return arguments[0].theme.colors.brand})({theme:{colors:{brand:'orange'}}})};}",
    "orange"
)]
#[case::shadowed_rest(
    "(...all)=>({color:((all)=>all[0].theme.colors.brand)([{theme:{colors:{brand:'orange'}}}])})",
    "orange"
)]
#[case::later_rest(
    "(p,...all)=>({color:all.length===0?p.color:all[0].theme.colors.brand})",
    "red"
)]
#[case::mapped_and_dynamic(
    "function(p){return{background:p.theme.colors.brand,color:arguments[0].color};}",
    "red"
)]
#[serial]
fn implicit_theme_guard_keeps_non_theme_and_unrelated_argument_scopes(
    #[case] rules: &str,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: props differ from the factory's arguments and nested callbacks' local inputs.
    let source = format!(
        "function make(){{return styled.div({rules});}}const Card=make({{theme:{{colors:{{brand:'orange'}}}}}});"
    );
    let observation = "JSON.stringify(values(render({target:Card,props:{color:'red'}}),'color'));";
    // When: extracted code executes an ordinary dynamic value in its actual scope.
    let observed = execute_in_mode("test.ts", &source, observation, single_css);
    // Then: only actual-props theme roots are refused; other values retain their behavior.
    assert_eq!(observed, serde_json::json!([expected]));
}
