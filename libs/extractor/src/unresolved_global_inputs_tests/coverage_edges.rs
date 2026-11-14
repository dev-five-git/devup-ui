use super::{extract_fixture, rejects_at};
use crate::ExtractStyleValue;
use css::style_selector::StyleSelector;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn group_nth_params_require_an_argument(
    #[values("_groupNthChild", "_groupNthOfType", "_groupNthLastChild")] selector: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{Box}} from '@devup-ui/react';\nexport const view=<Box {selector}={{{{params:[],color:'red'}}}}/>;"
    );
    rejects_at(&source, "[]", single_css);
}

#[rstest]
#[case("unknownKey")]
#[case("tick()")]
#[serial]
fn font_descriptor_keys_reject_unknown_computation(
    #[case] key: &str,
    #[values(false, true)] single_css: bool,
) {
    let statement = format!("globalCss({{fontFaces:[{{[{key}]:'normal',src:'face.woff2'}}]}});");
    let column = crate::dead_properties_test_utils::position(&statement, key) + 1;
    let source = format!("import {{globalCss}} from '@devup-ui/react';\n{statement}");
    let error = crate::dead_properties_test_utils::failure(extract_fixture(&source, single_css));
    let (location, _) = error
        .split_once(": ")
        .unwrap_or_else(|| panic!("unlocated error: {error}"));
    assert_eq!(location, format!("structural.tsx:2:{column}"));
    assert!(error.contains(&format!("cannot use `[{key}]` at build time:")));
}

#[rstest]
#[case("{params:null,color:'red'}", None)]
#[case("{_nthChild:{params:['2'],color:'red'}}", Some("&:nth-child(2)"))]
#[serial]
fn params_keep_the_media_selector_context(
    #[case] rules: &str,
    #[case] expected: Option<&str>,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{Box}} from '@devup-ui/react';\nexport const view=<Box _print={{{rules}}}/>;"
    );
    let output = extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let selectors: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => match style.selector.as_ref() {
                Some(StyleSelector::At { selector, .. }) => selector.as_deref(),
                other => panic!("expected media selector, got {other:?}"),
            },
            other => panic!("expected static declaration, got {other:?}"),
        })
        .collect();
    assert_eq!(selectors, vec![expected]);
}

#[rstest]
#[case((":hover", "&:hover", false))]
#[case((":host", "&:host", false))]
#[case((":nth-child", "&:nth-child", true))]
#[case(("::part", "&::part", true))]
#[case((":is", "&:is", true))]
#[serial]
fn evaluated_params_shapes_match_shallow_when_bridge_is_forced(
    #[case] target: (&str, &str, bool),
    #[values((None, None), (Some("[]"), Some("")), (Some("['']"), Some("")), (Some("['','x']"), Some("x")), (Some("['2n']"), Some("2n")), (Some("['a','b']"), Some("a,b")))]
    shape: (Option<&str>, Option<&str>),
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: identical authored rule coordinates and an independent shallow result.
    let _state = crate::test_state::TestStateGuard::new();
    let (selector, bare, required) = target;
    let member = shape
        .0
        .map_or_else(String::new, |value| format!("params:{value},"));
    let rule = format!("export const card=css({{props:data,'{selector}':{{{member}p:2}}}});");
    let source = |setup| format!("import{{css}}from'@devup-ui/react';\n{setup}\n{rule}");
    let compile = |source: &str| {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        crate::extract_without_source_map(
            "/bridge-params.css.ts",
            source,
            crate::ExtractOption {
                single_css,
                ..crate::ExtractOption::default()
            },
        )
        .map_err(|error| error.to_string())
    };
    let expected = compile(&source(concat!(
        "let data=0;for(let i=0;i<1;i++)data={",
        "payload:data};"
    )));
    // When: the ignored payload requires indexed evaluated lowering.
    let actual = compile(&source(concat!(
        "let data=0;for(let i=0;i<16;i++)data={",
        "payload:data};"
    )));
    // Then: exact selector bytes or the exact authored empty-list refusal agree.
    if required && shape.1 == Some("") {
        let marker = if shape.0 == Some("[]") { "[]" } else { "''" };
        let column = rule.find(marker).ok_or("missing empty witness")? + 1;
        let expected = expected.err().ok_or("shallow empty params accepted")?;
        let actual = actual.err().ok_or("bridge empty params accepted")?;
        assert!(
            expected.starts_with(&format!("/bridge-params.css.ts:3:{column}:")),
            "{expected}"
        );
        assert!(
            expected.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
            "{expected}"
        );
        assert_eq!(actual, expected);
    } else {
        let expected = expected?;
        let actual = actual?;
        let selector = shape
            .1
            .filter(|text| !text.is_empty())
            .map_or_else(|| bare.to_string(), |text| format!("{bare}({text})"));
        let selectors: Vec<_> = actual
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Static(style) => {
                    style.selector.as_ref().map(ToString::to_string)
                }
                _ => None,
            })
            .collect();
        assert_eq!(selectors, vec![selector]);
        assert_eq!(actual.styles, expected.styles);
        assert_eq!(actual.code, expected.code);
    }
    Ok(())
}

#[rstest]
#[case("_hover", "null")]
#[case("_hover", "undefined")]
#[case("_host", "void 0")]
#[case("_host", "[]")]
#[serial]
fn evaluated_absence_keeps_optional_bridge_selectors(
    #[case] selector: &str,
    #[case] value: &str,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: the metadata comes from a real evaluated helper, not lexical spelling.
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "import{{css}}from'@devup-ui/react';let data=0;for(let i=0;i<16;i++)data={{payload:data}};function value(){{return {value}}}export const card=css({{props:data,{selector}:{{params:value(),p:2}}}});"
    );
    // When
    let output = crate::extract_without_source_map(
        "/bridge-absence.css.ts",
        &source,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string())?;
    // Then
    let expected = if selector == "_hover" {
        "&:hover"
    } else {
        "&:host"
    };
    let selectors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => style.selector.as_ref().map(ToString::to_string),
            _ => None,
        })
        .collect();
    assert_eq!(selectors, vec![expected]);
    Ok(())
}

#[rstest]
#[serial]
fn evaluated_shadowed_undefined_is_value_proof_but_unresolved_shadow_is_not(
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let source = concat!(
        "import{css}from'@devup-ui/react';\nlet data=0;for(let i=0;i<16;i++)data={",
        "payload:data};\nfunction make(undefined){return css({props:data,_hover:{params:undefined,p:2}})}\nexport const card=make(void 0);"
    );
    let option = crate::ExtractOption {
        single_css,
        ..crate::ExtractOption::default()
    };
    let error = crate::extract_without_source_map("/bridge-shadow.tsx", source, option.clone())
        .err()
        .ok_or("unresolved shadow accepted")?
        .to_string();
    let line = source.lines().nth(2).ok_or("missing helper")?;
    let column = line.find("params:undefined").ok_or("missing shadow")? + "params:".len() + 1;
    assert!(
        error.starts_with(&format!("/bridge-shadow.tsx:3:{column}:")),
        "{error}"
    );
    assert!(
        error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
        "{error}"
    );
    // When
    let output = crate::extract_without_source_map("/bridge-shadow.css.ts", source, option)
        .map_err(|error| error.to_string())?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.selector.as_ref().is_some_and(|selector| selector.to_string() == "&:hover"))));
    Ok(())
}

#[rstest]
#[case("_nthChild", "[]", None)]
#[case("_nthChild", "['']", None)]
#[case("_nthChild", "['2n']", Some("&:nth-child(2n)"))]
#[case("_host", "[]", Some("&:host"))]
#[case("_host", "['.scope']", Some("&:host(.scope)"))]
#[serial]
fn media_and_layer_wrappers_preserve_native_params_requiredness(
    #[case] target: &str,
    #[case] params: &str,
    #[case] selector: Option<&str>,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: media and layer frames wrap, rather than replace, the pending target.
    let _state = crate::test_state::TestStateGuard::new();
    let compile = |source: &str| {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        crate::extract_without_source_map(
            "/wrapped-params.css.ts",
            source,
            crate::ExtractOption {
                single_css,
                ..crate::ExtractOption::default()
            },
        )
        .map_err(|error| error.to_string())
    };
    let source = format!(
        "import{{css}}from'@devup-ui/react';let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card=css({{props:data,{target}:{{'@media':{{screen:{{'@layer':{{card:{{params:{params},p:2}}}}}}}}}}}});"
    );
    let expected = selector.map(|selector| compile(&format!(
        "import{{css}}from'@devup-ui/react';export const card=css({{'@layer':{{card:{{'@media':{{screen:{{selectors:{{'{selector}':{{p:2}}}}}}}}}}}}}});"
    ))).transpose()?;
    // When
    let actual = compile(&source);
    // Then
    if let Some(expected) = expected {
        assert_eq!(actual?.styles, expected.styles);
    } else {
        let witness = if params == "[]" { "[]" } else { "''" };
        let offset = source.find(witness).ok_or("missing empty params")?;
        let location = crate::locate("/wrapped-params.css.ts", &source, offset);
        let error = actual
            .err()
            .ok_or("wrapped required selector accepted empty params")?;
        let printed = if params == "[]" { "[]" } else { "\"\"" };
        assert_eq!(
            error,
            format!(
                "{location}: {}",
                crate::utils::build_time_error(
                    "css",
                    printed,
                    crate::extractor::selector_params::SELECTOR_PARAMS
                )
            )
        );
    }
    Ok(())
}

#[rstest]
#[case((
    "",
    "_nthChild:{selectors:{'& &':{params:['2n'],selectors:{'&':{params:['3n'],_host:{params:[],p:2},_cue:{params:[],w:3}}}}}}",
    concat!("selectors:{'&:nth-child(3n) &:nth-child(2n):host':{", "p:2},'&:nth-child(3n) &:nth-child(2n)::cue':{", "w:3}}")
))]
#[case((
    "",
    "_host:{selectors:{'&-context':{params:['.scope'],_hover:{params:[],p:2}}}}",
    concat!("selectors:{'&:host-context(.scope):hover':{", "p:2}}")
))]
#[case((
    "let left={params:[],p:2};left={_host:left};left={params:['3n'],...left};left={selectors:{'&':left}};left={params:['2n'],...left};let right={params:[],w:3};right={_host:right};right={params:['5n'],...right};right={selectors:{'&':right}};right={params:['4n'],...right};let forks={selectors:{'&':left,'.copy &':right}};",
    "_nthChild:{selectors:{'& &':forks}}",
    concat!("selectors:{'&:nth-child(3n) &:nth-child(2n):host':{", "p:2},'.copy &:nth-child(5n) &:nth-child(4n):host':{", "w:3}}")
))]
#[serial]
fn native_pending_target_joins_and_lexical_replacement_match_flat_selectors(
    #[case] fixture: (&str, &str, &str),
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: independent flat selectors specify argument attachment and lexical replacement.
    let _state = crate::test_state::TestStateGuard::new();
    let (setup, nested, flat) = fixture;
    let compile = |body: &str| {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        crate::extract_without_source_map(
            "/pending-params.css.ts",
            &format!("import{{css}}from'@devup-ui/react';{body}"),
            crate::ExtractOption {
                single_css,
                ..crate::ExtractOption::default()
            },
        )
        .map_err(|error| error.to_string())
    };
    let expected = compile(&format!("export const card=css({{{flat}}});"))?;
    let body = format!(
        "let data=0;for(let i=0;i<16;i++)data={{payload:data}};{setup}export const card=css({{props:data,{nested}}});"
    );
    // When
    let actual = compile(&body)?;
    // Then
    assert_eq!(actual.styles, expected.styles);
    assert_eq!(actual.code, expected.code);
    Ok(())
}

#[rstest]
#[serial]
fn native_cached_join_tail_preserves_required_error_in_second_fork(
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: the first fork consumes both targets; the second must retain one required target.
    let _state = crate::test_state::TestStateGuard::new();
    let rule = "export const card=css({props:data,_nthChild:{selectors:{'& &':forks}}});";
    let source = format!(
        "import{{css}}from'@devup-ui/react';\nlet data=0;for(let i=0;i<16;i++)data={{payload:data}};let left={{params:[],p:2}};left={{_host:left}};left={{params:['3n'],...left}};left={{selectors:{{'&':left}}}};left={{params:['2n'],...left}};let right={{params:[],w:3}};right={{selectors:{{'&':right}}}};right={{params:['4n'],...right}};let forks={{selectors:{{'&':left,'.copy &':right}}}};\n{rule}"
    );
    let column = rule.find("css(").ok_or("missing native css call")? + 1;
    // When
    let error = crate::extract_without_source_map(
        "/cached-required.css.ts",
        &source,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    )
    .err()
    .ok_or("cached pop discarded the remaining required target")?
    .to_string();
    // Then
    assert!(
        error.starts_with(&format!("/cached-required.css.ts:3:{column}:")),
        "{error}"
    );
    assert!(
        error.contains(&crate::utils::build_time_error(
            "css",
            "_nthChild -> selectors -> & & -> selectors -> .copy & -> selectors -> & -> params",
            crate::extractor::selector_params::SELECTOR_PARAMS,
        )),
        "{error}"
    );
    Ok(())
}
