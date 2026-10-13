use super::literal_w38k_support::{declarations, fixture, red};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>{opaque();return;}")]
#[case("function(p){opaque();return;}")]
#[case("p=>{if(p.stop)opaque();return;}")]
#[case("function(p){if(p.stop)opaque();return;}")]
#[case("(p,q=opaque())=>{return;}")]
#[case("function(p,q=opaque()){return;}")]
#[case("({missing=opaque()})=>{return;}")]
#[case("function({missing=opaque()}){return;}")]
#[serial]
fn opaque_operation_when_it_throws_propagates_the_identical_sentinel(#[case] callback: &str) {
    // Given: only the test observation catches the generated component's exception.
    let renders = "const sentinel={};let calls=0;let gets=0;const opaque=()=>{calls++;trace.push('opaque');throw sentinel};trace.push('built');let caught;try{Card({get stop(){gets++;trace.push('get');return true}},null);trace.push('after')}catch(error){caught=error}const a={props:{className:caught===sentinel?'same':'different'}};const b={props:{className:JSON.stringify([calls,gets])}};";
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), renders);
    // When: public extraction accepts the opaque operation and actual JS executes it.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (observation, trace) = evaluate(&output.code);
    // Then: no catch/replay is inserted, and the prior trace and identity survive.
    assert_eq!(observation, vec!["same", "[1,1]"]);
    assert_eq!(trace, vec!["built", "get", "opaque"]);
    assert_eq!(declarations(&output), red(&[None]));
}

#[rstest]
#[case("p=>{if(p.stop)return;return;}")]
#[case("function(p){if(p.stop)return;return;}")]
#[serial]
fn opaque_getter_when_it_throws_propagates_without_an_extra_read(#[case] callback: &str) {
    // Given: a throwing getter supplies the accepted callback's authored if test.
    let renders = "const sentinel={};let gets=0;trace.push('built');let caught;try{Card({get stop(){gets++;trace.push('get');throw sentinel}},null);trace.push('after')}catch(error){caught=error}const a={props:{className:caught===sentinel?'same':'different'}};const b={props:{className:String(gets)}};";
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), renders);
    // When: the actual generated component reaches the opaque getter.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (observation, trace) = evaluate(&output.code);
    // Then: the same sentinel escapes and exactly one getter read precedes it.
    assert_eq!(observation, vec!["same", "1"]);
    assert_eq!(trace, vec!["built", "get"]);
    assert_eq!(declarations(&output), red(&[None]));
}
