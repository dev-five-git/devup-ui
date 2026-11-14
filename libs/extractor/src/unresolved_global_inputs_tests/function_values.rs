use super::extract_fixture;
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(("imports:[{url:() => \"face.css\"}]", "() => \"face.css\""))]
#[case(("imports:[{url:'face.css',query:() => \"screen\"}]", "() => \"screen\""))]
#[case(("fontFaces:[{fontFamily:() => \"Face\",src:'face.woff2'}]", "() => \"Face\""))]
#[case(("fontFaces:[`font-family:${() => \"Face\"};src:url(face.woff2);`]", "() => \"Face\""))]
#[case(("fontFaces:[`font-family:${() => \" at build time: \"};src:url(face.woff2);`]", "() => \" at build time: \""))]
#[case(("fontFaces:[`font-family:${class {}};src:url(face.woff2);`]", "class {}"))]
#[case(("fontFaces:[`font-family:${function() {}};src:url(face.woff2);`]", "function() {}"))]
#[case(("body:{_hover:{params:() => [],color:'red'}}", "() => []"))]
#[case(("body:{_nthChild:{params:[() => \"2\"],color:'red'}}", "() => \"2\""))]
#[case(("imports:[{url:class {}}]", "class {}"))]
#[case(("fontFaces:[{fontFamily:function() {},src:'face.woff2'}]", "function() {}"))]
#[serial]
fn structural_functions_cannot_disappear_during_value_evaluation(
    #[case] fixture: (&str, &str),
    #[values(false, true)] single_css: bool,
) {
    let (properties, marker) = fixture;
    let statement = format!("globalCss({{{properties}}});");
    let column = crate::dead_properties_test_utils::position(&statement, marker) + 1;
    let source = format!("import {{globalCss}} from '@devup-ui/react';\n{statement}");
    let code = if matches!(marker, "class {}" | "function() {}") {
        format!("({marker})")
    } else {
        marker.to_string()
    };
    let error = crate::dead_properties_test_utils::failure(extract_fixture(&source, single_css));
    let (location, message) = error
        .split_once(": ")
        .unwrap_or_else(|| panic!("unlocated error: {error}"));
    assert_eq!(location, format!("structural.tsx:2:{column}"));
    assert!(message.contains(&format!("cannot use `{code}` at build time:")));
}

#[rstest]
#[serial]
fn a_pure_call_returning_text_still_uses_exact_value_evaluation(
    #[values(false, true)] single_css: bool,
) {
    let source = "import {globalCss} from '@devup-ui/react';\nglobalCss({imports:[{url:(() => 'pure.css')()}]});";
    let output = extract_fixture(source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let urls: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Import(import) => import.url.as_str(),
            other => panic!("expected import, got {other:?}"),
        })
        .collect();
    assert_eq!(urls, vec!["\"pure.css\""]);
}
