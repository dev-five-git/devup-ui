use super::literal_w38k_support::{GETTERS, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>{}", vec!["built", "get", "get"])]
#[case("function(p){}", vec!["built", "get", "get"])]
#[case("p=>{'use strict';}", vec!["built", "get", "get"])]
#[case("p=>{;return;}", vec!["built", "get", "get"])]
#[case("p=>{{{return;}}}", vec!["built", "get", "get"])]
#[case("p=>{if(p.stop){return;}else{return;}}", vec!["built", "get", "get"])]
#[case("p=>{if(p.stop)return;}", vec!["built", "get", "get"])]
#[case("p=>{function inner(){return 255;}return;}", vec!["built", "get", "get"])]
#[case("p=>{const inner=()=>255;return;}", vec!["built", "get", "get"])]
#[case("p=>{return;trace.push('unreachable');}", vec!["built", "get", "get"])]
#[case(
    "p=>{const value=trace.push('init');return;}",
    vec!["built", "get", "init", "get", "init"]
)]
#[case(
    "p=>{if(p.stop)trace.push('yes');else trace.push('no');return;}",
    vec!["built", "get", "yes", "get", "no"]
)]
#[case("p=>{trace.push('body');}", vec!["built", "get", "body", "get", "body"])]
#[serial]
fn supported_normal_exits_when_owned_returns_are_bare_omit_order(
    #[case] callback: &str,
    #[case] trace: Vec<&str>,
) {
    // Given: real sources distinguish falloff, opaque operations and nested returns.
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), GETTERS);
    // When: extraction and the retained generated callback invocation execute.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, actual_trace) = evaluate(&output.code);
    // Then: every selected declaration is unlayered and authored effects run once.
    assert_eq!(actual_trace, trace);
    assert_eq!(tokens(&classes), vec![vec!["color-0-red--255-a"]; 2]);
    assert_eq!(declarations(&output), red(&[None]));
}

#[rstest]
#[case("p=>{if(p.stop)return 2;}", "color-0-red--2-a")]
#[case("function(p){if(p.stop)return 2;}", "color-0-red--2-a")]
#[case("p=>{return;return 2;}", "color-0-red--255-a")]
#[serial]
fn valued_candidates_when_body_also_has_absence_remain_in_the_sheet(
    #[case] callback: &str,
    #[case] first: &str,
) {
    // Given: falloff and unreachable valued returns are different from all-bare.
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), GETTERS);
    // When: both branches render from actual generated JavaScript.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    // Then: candidate 2 remains validated/emitted without replaying the getter.
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(
        tokens(&classes),
        vec![vec![first], vec!["color-0-red--255-a"]]
    );
    assert_eq!(declarations(&output), red(&[None, Some(2)]));
}
