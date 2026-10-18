use super::execute_in_mode;
use crate::{ExtractOption, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::arrow(
    "p=>({color:p['theme'].colors.primary,margin:p.theme.space[2]})",
    false
)]
#[case::block_arrow(
    "p=>{return {color:p.theme.colors.primary,margin:p.theme.space[2]};}",
    false
)]
#[case::normal(
    "function(p){return {color:p.theme.colors.primary,margin:p['theme'].space[2]};}",
    false
)]
#[case::extra_default(
    "(p,unused=(trace.push('default'),0))=>({color:p.theme.colors.primary,margin:p.theme.space[2]})",
    true
)]
#[case::renamed_default(
    "({theme:palette},unused=(trace.push('default'),0))=>({color:palette['colors'].primary,margin:palette.space[2]})",
    true
)]
#[case::normal_renamed(
    concat!("function(", "{", "theme:palette", "}", ",unused=(trace.push('default'),0)){return {color:palette.colors.primary,margin:palette.space[2]};}"),
    true
)]
#[serial]
fn mapped_theme_leaves_stay_static_when_original_parameter_work_must_run(
    #[case] callback: &str,
    #[case] default_runs: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: static theme paths do not need a runtime theme object, but defaults still have effects.
    let source =
        format!("const trace=[];const Card=styled.div({callback});const defined=trace.slice();");
    let observation = "const first=render({target:Card,props:{}});const second=render({target:Card,props:{}});JSON.stringify([defined,trace,[first,second].map(element=>atoms(element).map(rule=>rule.slice(0,2)).sort())]);";
    // When: the generated original callback executes on each render with an empty dynamic vector.
    let observed = execute_in_mode("test.ts", &source, observation, single_css);
    let trace = if default_runs {
        serde_json::json!(["default", "default"])
    } else {
        serde_json::json!([])
    };
    let atoms = serde_json::json!([
        ["color", "var(--colors-primary)"],
        ["margin", "var(--space-2)"]
    ]);
    // Then: declarations remain static CSS variables and parameter effects are neither lost nor repeated.
    assert_eq!(observed, serde_json::json!([[], trace, [atoms, atoms]]));
}

#[rstest]
#[case::property_arrow("{color:(p,unused=p.theme.colors[key])=>'red'}")]
#[case::property_normal(
    "{color:function({theme:palette},unused=palette.colors[key]){return 'red';}}"
)]
#[case::whole_arrow("(p,unused=p['theme'].colors[key])=>({color:'red'})")]
#[case::whole_normal(concat!("function(", "{", "theme:palette", "}", ",unused=palette.colors[key]){return {color:'red'};}"))]
#[case::nested_default("({theme:palette,color=palette.colors[key]})=>({color:'red'})")]
#[case::property_nested_default("{color:({theme:palette,color=palette.colors[key]})=>'red'}")]
#[case::ordinary_mapper_must_keep_effects("{color:(p,unused=effect())=>p.theme.colors.primary}")]
#[case::ordinary_mapper_binding_default(
    "{color:({theme:palette,color=effect()})=>palette.colors.primary}"
)]
#[serial]
fn parameter_theme_reads_keep_located_refusals_when_extra_formals_or_defaults_hide_them(
    #[case] rules: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: a first-props-root theme read outside the conservative whole-function mapper.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{styled}} from '@devup-ui/react';\nconst make=(key,effect)=>styled.div({rules});"
    );
    // When: the shared theme scanner inspects both the original body and parameter initializers.
    let result = extract(
        "props-default-theme.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then: the original located theme guard is retained rather than emitting a runtime theme read.
    let error = match result {
        Ok(output) => panic!("parameter theme work must not escape mapping: {output:?}"),
        Err(error) => error.to_string(),
    };
    assert!(error.starts_with("props-default-theme.ts:2:"), "{error}");
    assert!(
        error.contains("a theme read must be a path of names or literal keys"),
        "{error}"
    );
}

#[rstest]
#[case::number(("2", serde_json::json!("2px"), serde_json::json!(2)))]
#[case::string(("'3em'", serde_json::json!("3em"), serde_json::json!("3em")))]
#[case::zero(("0", serde_json::json!(0), serde_json::json!(0)))]
#[case::negative(("-2", serde_json::json!("-2px"), serde_json::json!(-2)))]
#[serial]
fn whole_rule_numbers_keep_the_existing_conversion_when_ordinary_property_callbacks_do_not(
    #[case] fixture: (&str, serde_json::Value, serde_json::Value),
    #[values(false, true)] single_css: bool,
) {
    // Given: whole-rule conversion must not spread to ordinary authored property callbacks.
    let (input, whole_margin, ordinary_margin) = fixture;
    let source = "const trace=[];const Whole=styled.div(p=>({margin:p.margin,opacity:p.opacity,padding:1}));const Ordinary=styled.div({margin:(p,unused=(trace.push('ordinary'),0))=>p.margin});";
    let observation = format!(
        "const whole=render({{target:Whole,props:{{margin:{input},opacity:0.5}}}});const ordinary=render({{target:Ordinary,props:{{margin:{input}}}}});JSON.stringify([trace,values(whole,'margin'),values(whole,'opacity'),values(whole,'padding'),values(ordinary,'margin')]);"
    );
    // When: both supported callback forms execute with the same input.
    let observed = execute_in_mode("test.ts", source, &observation, single_css);
    // Then: dynamic nonzero px, unitless, static scale, zero, and ordinary values keep their old domains.
    assert_eq!(
        observed,
        serde_json::json!([
            ["ordinary"],
            [whole_margin],
            [0.5],
            ["4px"],
            [ordinary_margin]
        ])
    );
}
