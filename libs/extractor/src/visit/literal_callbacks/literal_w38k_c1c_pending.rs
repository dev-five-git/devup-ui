use super::literal_w38k_support::{GETTERS, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("${css({color:'red'})};")]
#[serial]
fn mixin_when_followed_by_absence_keeps_its_actual_emitted_declarations(#[case] mixin: &str) {
    // Given: actual callback and css() mixins, rather than manufactured carriers.
    let source = fixture(&format!("{mixin}style-order:${{p=>{{return;}}}};"), GETTERS)
        .replace("import {styled}", "import {styled,css}");
    let control_source = fixture(mixin, GETTERS).replace("import {styled}", "import {styled,css}");
    // When: production suppliers prepare the mixin and generated components run.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let control = compile(&control_source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    let (control_classes, control_trace) = evaluate(&control.code);
    // Then: the full declaration vector and exact selected classes retain the mixin.
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(trace, control_trace);
    assert_eq!(tokens(&classes), tokens(&control_classes));
    assert_eq!(declarations(&output), red(&[None]));
    assert_eq!(declarations(&output), declarations(&control));
}
