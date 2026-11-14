use super::*;
use rstest::rstest;
use serial_test::serial;

fn source(factory: &str, props: &str) -> String {
    let module = match factory {
        "jsx" | "jsxs" => "react/jsx-runtime",
        "jsxDEV" => "react/jsx-dev-runtime",
        "createElement" => "react",
        _ => panic!("unknown factory: {factory}"),
    };
    format!(
        "{BOX}import {{{factory}}} from '{module}';\nexport const a=(rest)=>{factory}(Box,{props});"
    )
}

#[rstest]
#[serial]
fn factory_styles_when_unreadable_report_the_jsx_error_at_the_value(
    #[values("jsx", "jsxs", "jsxDEV", "createElement")] factory: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = source(factory, "{color: () => 1}");
    let jsx = format!("{BOX}export const a=()=> <Box color={{() => 1}}/>;");
    let option = || ExtractOption {
        single_css,
        ..ExtractOption::default()
    };
    let expected = compile_with(&jsx, option())
        .err()
        .unwrap_or_else(|| panic!("JSX unreadable control must fail"));
    let offset = source
        .find("() => 1")
        .unwrap_or_else(|| panic!("missing value"));
    let column = source[..offset]
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .len()
        + 1;
    let actual = compile_with(&source, option())
        .err()
        .unwrap_or_else(|| panic!("factory silently drops its explicit style: {source}"));
    let category = expected
        .split_once(": `<Box>`")
        .unwrap_or_else(|| panic!("{expected}"));
    assert_eq!(actual, format!("a.tsx:3:{column}: `<Box>`{}", category.1));
}

#[rstest]
#[serial]
fn factory_styles_when_shadowed_keep_the_winner_and_discarded_effects(
    #[values("jsx", "jsxs", "jsxDEV", "createElement")] factory: &str,
    #[values(false, true)] single_css: bool,
    #[values(
        "{color:()=>1,color:'red'}",
        "{...{color:()=>1},color:'red'}",
        "{color:(trace.push('discarded'),()=>1),color:'red'}"
    )]
    props: &str,
) {
    let source = source(factory, props);
    let compiled = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        static_styles(&compiled),
        vec![("color".into(), "red".into())]
    );
    let script = compiled
        .code
        .replace("jsxs(", "jsx(")
        .replace("jsxDEV(", "jsx(");
    let actual = whole::execute(&script, "a({})");
    let expected = if props.contains("trace.push") {
        serde_json::json!(["discarded"])
    } else {
        serde_json::json!([])
    };
    assert_eq!(actual.trace, expected);
}

#[rstest]
#[serial]
fn factory_styles_when_only_spread_keys_are_unreadable_pass_them_through(
    #[values("jsx", "jsxs", "jsxDEV", "createElement")] factory: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = source(factory, "{color:'red',...rest}");
    let compiled = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let script = compiled
        .code
        .replace("jsxs(", "jsx(")
        .replace("jsxDEV(", "jsx(");
    let actual = whole::execute(
        &script,
        "(()=>{const raw=()=>{throw 'called';};return a({color:raw}).props.color===raw;})()",
    );
    assert_eq!(actual.element, true);
    assert_eq!(
        static_styles(&compiled),
        vec![("color".into(), "red".into())]
    );
}

#[rstest]
#[serial]
fn factory_values_when_known_copy_keys_are_not_selectors_keep_d1(
    #[values("jsx", "jsxs", "jsxDEV", "createElement")] factory: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = source(factory, "{p:'4px',...{['__proto__']:null,p:'20px'}}");
    let compiled = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        static_styles(&compiled),
        vec![("padding".into(), "20px".into())]
    );
}
