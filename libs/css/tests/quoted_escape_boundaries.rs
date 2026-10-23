use css::{optimize_value::optimize_value, quoted::replace_unquoted};
use rstest::rstest;

#[rstest]
#[case("61")]
#[case("000061")]
fn hex_escape_whitespace_keeps_the_string_open(#[case] digits: &str) {
    let source = format!("'a\\{digits}\n0.5' 0.5px");
    assert_eq!(optimize_value(&source), format!("'a\\{digits}\n0.5' .5px"));
}

#[test]
fn a_seventh_hex_digit_leaves_the_following_newline_unescaped() {
    let source = "'a\\000061f\n0.5' 0.5px";
    assert_eq!(optimize_value(source), "'a\\000061f\n.5' 0.5px");
}

#[rstest]
#[case(r"&.\000061&", r".cls.\000061.cls")]
#[case(r"&.\26 &", r".cls.\26 .cls")]
fn a_nesting_token_after_a_complete_hex_escape_is_syntax(
    #[case] source: &str,
    #[case] expected: &str,
) {
    assert_eq!(replace_unquoted(source, b'&', ".cls"), expected);
}

#[test]
fn replacement_borrows_when_every_ampersand_is_data() {
    let source = r"'&' .a\&b";
    let actual = replace_unquoted(source, b'&', ".cls");
    assert!(matches!(actual, std::borrow::Cow::Borrowed(value) if value == source));
}
