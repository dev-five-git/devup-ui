use super::literal_w38k_support::{GETTERS, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use serial_test::serial;

#[test]
#[serial]
fn static_css_mixin_when_order_returns_bare_selects_exact_unlayered_red() {
    // Given: a real css() mixin followed by an all-bare order callback.
    let source = fixture(
        "${css({color:'red'})};style-order:${p=>{return;}};",
        GETTERS,
    )
    .replace("import {styled}", "import {styled,css}");
    // When: public extraction and the actual generated renders execute.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    // Then: each render selects precisely the emitted unlayered red token.
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(
        tokens(&classes),
        vec![vec!["color-0-red--255-a"], vec!["color-0-red--255-a"]]
    );
    assert_eq!(declarations(&output), red(&[None]));
}

#[test]
#[serial]
fn static_css_mixin_when_style_order_is_omitted_corrected_behavior_has_string_class_name() {
    // Given: the no-order baseline observes typeof on actual render results.
    let renders = concat!(
        "trace.push('built');",
        "const actualA=Card({get stop(){trace.push('get');return true}},null);",
        "const actualB=Card({get stop(){trace.push('get');return false}},null);",
        "const a={props:{className:typeof actualA.props.className}};",
        "const b={props:{className:typeof actualB.props.className}};"
    );
    let source = fixture("${css({color:'red'})};", renders)
        .replace("import {styled}", "import {styled,css}");
    // When: public extraction and actual generated components produce observations.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (observation, trace) = evaluate(&output.code);
    // Then: the corrected no-order result has a string className and still emits red.
    assert_eq!(observation, vec!["string", "string"]);
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(declarations(&output), red(&[None]));
}
