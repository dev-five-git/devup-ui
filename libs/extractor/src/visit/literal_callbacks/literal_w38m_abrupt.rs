use super::literal_w38k_support::declarations;
use super::literal_w38m_support::{fixture, policy_error, rules};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>{opaque();trace.push('callback-later');return;}")]
#[case("({missing=opaque()})=>{trace.push('callback-later');return;}")]
#[serial]
fn abrupt_render_when_nested_callback_effect_or_default_throws_stops_later_operations(
    #[case] callback: &str,
) {
    // Given: test-only observation catches the original opaque sentinel, not production.
    let renders = "const sentinel={};let calls=0;let gets=0;const opaque=()=>{calls++;trace.push('opaque');throw sentinel};trace.push('built');let caught;try{Card({get stop(){gets++;trace.push('get');return true}},null);trace.push('after-render')}catch(error){caught=error}const a={props:{className:caught===sentinel?'same':'different'}};const b={props:{className:JSON.stringify([calls,gets])}};";
    let source = fixture(
        &format!("...rest,_hover:`style-order:${{{callback}}};color:blue`"),
        renders,
    );
    // When: the accepted opaque effect/default runs in the real component.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (observation, trace) = evaluate(&output.code);
    // Then: identical sentinel, one invocation/read, and no later operation survive.
    assert_eq!(observation, vec!["same", "[1,1]"]);
    assert_eq!(trace, vec!["built", "get", "opaque"]);
    assert_eq!(declarations(&output), rules("red", &[None]));
}

#[test]
#[serial]
fn comment_throw_when_spread_text_is_ineligible_retains_policy_rejection_before_execution() {
    // Given: the exact opaque construction fixture rejected by the ca8 receipts.
    let root = "_focus:`/*${trace.push('before')}*/color:green`,...{color:'red',_active:`/*${opaque()}*/color:yellow`},_disabled:`/*${trace.push('after')}*/color:gray`,_hover:`ORDERcolor:blue`";
    let setup =
        "const sentinel={};const opaque=()=>{trace.push('opaque');throw sentinel};let caught;try{";
    let tail = "trace.push('built');Card({},null);trace.push('after-render')}catch(error){caught=error}const a={props:{className:caught===sentinel?'same':'different'}};const b={props:{className:'stopped'}};";
    let make = |order: &str| {
        fixture(&root.replace("ORDER", order), tail)
            .replace("const Card=", &format!("{setup}const Card="))
    };
    // When: public extraction checks ordered and no-order forms without running them.
    for input in [make(""), make("style-order:${p=>{return;}};")] {
        let error = compile(&input)
            .err()
            .unwrap_or_else(|| panic!("ineligible text compiled"));
        // Then: every original policy rejection stays located at its real source operand.
        let expected = vec![
            policy_error(&input, "`/*${trace.push('before')}*/color:green`", 0),
            policy_error(&input, "`/*${opaque()}*/color:yellow`", 0),
            policy_error(&input, "`/*${trace.push('after')}*/color:gray`", 0),
        ];
        let actual = error
            .lines()
            .filter(|line| line.contains("`styled()`"))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}
