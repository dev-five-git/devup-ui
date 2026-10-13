use super::literal_w38k_support::{declarations, tokens};
use super::literal_w38m_support::{GETTERS, fixture, rules, selected};
use super::{compile, evaluate};
use serial_test::serial;

#[test]
#[serial]
fn nested_order_when_root_has_real_spread_selects_red_and_ordered_hover_blue() {
    // Given: source inputs independently define all declarations and both branch orders.
    let source = fixture(
        "...rest,_hover:`style-order:${p=>p.stop?2:3};color:blue`",
        GETTERS,
    );
    let expected = rules("red", &[Some(2), Some(3)]);
    // When: public extraction and the actual Oxc/Boa renders run.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    // Then: no extra declarations, selectors, orders or selected tokens survive.
    assert_eq!(declarations(&output), expected);
    assert_eq!(tokens(&classes), selected("red", &[Some(2), Some(3)]));
    assert_eq!(trace, vec!["built", "get", "get"]);
}

#[test]
#[serial]
fn nested_bare_order_when_root_has_real_spread_matches_independent_omission() {
    // Given: the comparator differs only by the authored order callback.
    let root = "...rest,_hover:`style-order:${p=>{return;}};color:blue`";
    let source = fixture(root, GETTERS);
    let reference = fixture("...rest,_hover:`color:blue`", GETTERS);
    let expected = rules("red", &[None]);
    // When: both real source forms are extracted and rendered.
    let omitted = compile(&reference).unwrap_or_else(|error| panic!("{error}"));
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let actual = evaluate(&output.code);
    // Then: the independent oracle prevents two equally broken outputs passing.
    assert_eq!(declarations(&omitted), expected);
    assert_eq!(declarations(&output), expected);
    assert_eq!(tokens(&actual.0), selected("red", &[None, None]));
    assert_eq!(actual, evaluate(&omitted.code));
    assert_eq!(actual.1, vec!["built", "get", "get"]);
}
