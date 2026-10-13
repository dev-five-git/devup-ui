use super::literal_w38k_support::{GETTERS, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use crate::ExtractStyleValue;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn omitted_order_when_inline_css_is_static_selects_exact_red_on_each_render() {
    // Given: omission has its own oracle, not a comparator that can lose both classes.
    let source = fixture("${css({color:'red'})};", GETTERS)
        .replace("import {styled}", "import {styled,css}");
    // When: public extraction and the real Oxc/Boa component renders execute.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    // Then: the complete declarations, exact selections and source effects survive.
    assert_eq!(declarations(&output), red(&[None]));
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(
        tokens(&classes),
        vec![vec!["color-0-red--255-a"], vec!["color-0-red--255-a"]]
    );
}

#[rstest]
#[case("color:blue;${css({color:'red'})};", "red", vec!["red"])]
#[case("${css({color:'red'})};color:blue;", "blue", vec!["red", "blue"])]
#[serial]
fn omitted_order_when_declarations_surround_a_mixin_keeps_source_composition(
    #[case] body: &str,
    #[case] selected: &str,
    #[case] emitted: Vec<&str>,
) {
    // Given: swapping authored declaration order changes the source-derived winner.
    let source = fixture(body, GETTERS).replace("import {styled}", "import {styled,css}");
    let mut expected = emitted
        .iter()
        .map(|value| ExtractStyleValue::Static(ExtractStaticStyle::new("color", value, 0, None)))
        .collect::<Vec<_>>();
    expected.sort_unstable();
    let selected = format!("color-0-{selected}--255-a");
    // When: the generated component executes both original getter cases.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    // Then: complete emission and exact selected tokens obey later-source precedence.
    assert_eq!(declarations(&output), expected);
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(tokens(&classes), vec![vec![selected.as_str()]; 2]);
}
