use super::w38n_mixin_source::fixture;
use super::*;

#[test]
#[serial]
fn public_class_names_mixin_when_order_is_255_reports_one_exact_authored_diagnostic() {
    // Given: P1 uses the approved ClassNames shell and authored invalid token.
    let source = fixture("background:blue;${{color:'red'}};style-order:255");
    let column = source
        .lines()
        .nth(1)
        .required("authored second line")
        .find("255")
        .required("authored order token")
        + 1;
    let expected = format!(
        "a.tsx:2:{column}: `styleOrder()` cannot use `255` at build time: an explicit styleOrder must be an ECMAScript Number integer from 1 to 254 or canonical decimal string without signs, spaces or leading zeros"
    );
    // When: the existing real Emotion alias extraction entrypoint handles P1.
    let actual = compile_emotion(&source).required_err("invalid explicit order must reject");
    // Then: full equality checks text, location and diagnostic multiplicity together.
    assert_eq!(actual, expected);
}
