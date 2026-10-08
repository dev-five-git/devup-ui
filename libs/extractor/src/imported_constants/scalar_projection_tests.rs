use super::exact_tests::{extracted, static_values};
use rstest::rstest;

#[rstest]
#[case("const made={width: '13px',child: {n: 1}};watch(made.child);")]
#[case("const original={width:'13px'};const made={...original};watch(original);")]
#[case("const made=Object.freeze({width: '13px',child: {n: 1}});watch(made.child);")]
#[case("const made=Object.freeze({width: '13px',child: {n: 1}});watch(made);")]
#[case("const made={width: '12px',width: '13px',child: {n: 1}};watch(made.child);")]
#[case(
    "const made={width: '13px',child: {n: 1}};function other(){const made={width: '2px'};watch(made)}watch(made.child);"
)]
#[case("const original={width:'13px'};const made={...original};original.width='2px';")]
#[case("const made={width: '13px',child: {n: 1}};watch(made.width);watch(made.child);")]
#[serial_test::serial]
fn own_scalar_when_independent_of_child_or_source_hazards_is_exact(#[case] declarations: &str) {
    let source = format!(
        "import {{css}} from '@devup-ui/react';{declarations}const f=()=>made['width'];css({{width:f()}});"
    );
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(static_values(&output), vec!["13px".to_string()]);
}

#[rstest]
#[case("watch(made);")]
#[case("const alias=made;watch(alias);")]
#[case("const alias=made;")]
#[case("const box={made};watch(box);")]
#[case("const writer=()=>{made.width='2px'};")]
#[case("made.child.n=2;")]
#[case("made.width='2px';")]
#[case("watch(made[key]);")]
#[case("made.receiver();")]
#[case("export {made};")]
#[serial_test::serial]
fn own_scalar_when_any_later_hazard_reaches_the_root_is_rejected(#[case] hazard: &str) {
    let source = format!(
        "import {{css}} from '@devup-ui/react';const made={{width:'13px',child:{{n:1}}}};watch(made.child);const f=()=>made.width;css({{width:f()}});{hazard}"
    );
    assert!(extracted(&source, "").is_err(), "{hazard}");
}

#[rstest]
#[case("watch(original);const made={...original};")]
#[case("const writer=()=>watch(original);const made={...original};")]
#[case("const alias=original;const made={...original};watch(alias);")]
#[case("const made={...original};watch(made);")]
#[case("const made={...original};made.width='2px';")]
#[serial_test::serial]
fn copied_scalar_when_source_readiness_or_destination_safety_is_uncertain_is_rejected(
    #[case] declarations: &str,
) {
    let source = format!(
        "import {{css}} from '@devup-ui/react';const original={{width:'13px'}};{declarations}const f=()=>made.width;css({{width:f()}});"
    );
    assert!(extracted(&source, "").is_err(), "{declarations}");
}

#[rstest]
#[case("const made={width:'13px',child:{width:'2px'}};watch(made.child);")]
#[case("const made=Object.freeze({width:'13px',child:{width:'2px'}});watch(made.child);")]
#[case("const original={child:{width:'2px'}};const made={...original};watch(original.child);")]
#[serial_test::serial]
fn nested_child_when_escaped_stays_runtime_on_elements(#[case] declarations: &str) {
    let source = format!(
        "import {{Box}} from '@devup-ui/react';{declarations}const f=()=>made.child.width;export const a=<Box width={{f()}}/>;"
    );
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(static_values(&output), Vec::<String>::new());
    assert!(
        output.code.contains("f()") && output.code.contains("--"),
        "{}",
        output.code
    );
}

#[test]
#[serial_test::serial]
fn scalar_helper_when_root_is_uninitialized_is_rejected() {
    let source = "import {css} from '@devup-ui/react';const f=()=>made.width;css({width:f()});const made={width:'13px',child:{n:1}};watch(made.child);";
    assert!(extracted(source, "").is_err());
}

#[test]
#[serial_test::serial]
fn direct_scalar_when_child_escapes_is_exact_without_replacing_runtime_reads() {
    let source = "import {css} from '@devup-ui/react';const made={width: '13px',child: {n: 1}};watch(made.child);observe(made.width);css({width: made.width});";
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(static_values(&output), vec!["13px".to_string()]);
    assert!(
        output.code.contains("observe(made.width)"),
        "{}",
        output.code
    );
}
