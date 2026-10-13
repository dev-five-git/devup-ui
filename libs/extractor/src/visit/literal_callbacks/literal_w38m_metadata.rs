use super::literal_w38k_support::{declarations, tokens};
use super::literal_w38m_support::{GETTERS, class_tokens, fixture, located, static_color};
use super::{compile, evaluate};
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("style-order:255;style-order:${p=>{return;}};")]
#[case("style-order:${p=>255};style-order:${p=>{return;}};")]
#[case("style-order:${p=>p.stop?2:255};style-order:${p=>{return;}};")]
#[case("style-order:${false?255:2};style-order:${p=>{return;}};")]
#[case("style-order:${false&&255};style-order:${p=>{return;}};")]
#[case("style-order:${p=>{return;}};style-order:255;")]
#[case("style-order:${p=>{return;}};&:focus{style-order:255;color:green}")]
#[case("style-order:${p=>{return;return 255;}};")]
#[serial]
fn mixed_metadata_when_invalid_candidate_is_removed_or_unreachable_rejects_at_source(
    #[case] body: &str,
) {
    // Given: invalid removed, branch, later, inner and unreachable metadata under a spread.
    let source = fixture(&format!("...rest,_hover:`{body}color:blue`"), "");
    let location = located(&source, "255");
    // When: the actual styled supplier handles the mixed object.
    let error = compile(&source)
        .err()
        .unwrap_or_else(|| panic!("255 compiled"));
    // Then: the invalid token, not the valid bare callback, owns the located error.
    assert!(error.starts_with(&location), "{error}");
    assert!(error.contains("255"), "{error}");
    assert!(error.contains("styleOrder"), "{error}");
}

#[rstest]
#[case("style-order:${p=>{return;}};style-order:7;color:blue", 7, "&:hover")]
#[case(
    "style-order:${p=>{return;}};&:focus{style-order:3;color:blue}",
    3,
    "&:hover:focus"
)]
#[serial]
fn mixed_metadata_when_valid_later_or_inner_order_survives_keeps_exact_declarations(
    #[case] body: &str,
    #[case] order: u8,
    #[case] selector: &str,
) {
    // Given: later7 and independently nested3 are distinct from the omitted order.
    let source = fixture(&format!("...rest,_hover:`{body}`"), GETTERS);
    let mut expected = vec![
        static_color("red", None, None),
        static_color("blue", Some(selector), Some(order)),
    ];
    expected.sort_unstable();
    // When: real extraction and two component renders execute.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    // Then: omission never consumes the later or inner metadata boundary.
    assert_eq!(
        declarations(&output),
        expected
            .iter()
            .cloned()
            .map(ExtractStyleValue::Static)
            .collect::<Vec<_>>()
    );
    assert_eq!(trace, vec!["built", "get", "get"]);
    let mut expected_tokens = class_tokens(&expected);
    expected_tokens.sort_unstable();
    for class in tokens(&classes) {
        let mut actual_tokens = class;
        actual_tokens.sort_unstable();
        assert_eq!(actual_tokens, expected_tokens);
    }
}
