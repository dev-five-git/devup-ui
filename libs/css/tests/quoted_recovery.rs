use css::{
    at_rule::{normalize_query, split_at_rule_key},
    merge_selector,
    optimize_value::optimize_value,
    quoted::{QuoteState, split_unquoted, string_end},
    rm_css_comment::rm_css_comment,
    style_selector::split_selector_list,
};
use rstest::rstest;

#[rstest]
fn optimization_resumes_when_a_string_hits_a_newline(
    #[values("\n", "\r", "\x0c", "\r\n")] newline: &str,
) {
    // Given CSS Syntax's bad-string boundary: https://drafts.csswg.org/css-syntax-3/#consume-string-token
    let source = format!("'bad{newline}0.5px");
    // When the value is optimized after the unescaped newline.
    let actual = optimize_value(&source);
    // Then optimization resumes without deleting the raw token boundary.
    assert_eq!(actual, format!("'bad{newline}.5px"));
}

#[rstest]
fn string_span_keeps_the_boundary_when_a_newline_ends_it(
    #[values("\n", "\r", "\x0c", "\r\n")] newline: &str,
) {
    // Given a bad-string followed by a real declaration separator.
    let source = format!("'bad{newline};color:red");
    // When its protected span is located.
    let end = string_end(source.as_bytes(), 0, b'\'');
    // Then the raw newline is protected but following syntax is not.
    assert_eq!(&source[..end], format!("'bad{newline}"));
}

#[rstest]
fn continuation_bytes_stay_quoted_when_the_newline_is_escaped(
    #[values("\n", "\r", "\r\n", "\x0c")] newline: &str,
) {
    // Given an escaped newline inside a closed string.
    let source = format!("'a\\{newline}0.5' 0.5px");
    // When the value is optimized.
    let actual = optimize_value(&source);
    // Then the complete raw continuation stays inside the protected string.
    assert_eq!(actual, format!("'a\\{newline}0.5' .5px"));
}

#[rstest]
fn escape_terminator_stays_quoted_when_a_hex_escape_consumes_a_newline(
    #[values("\n", "\r", "\r\n", "\x0c")] newline: &str,
) {
    // Given the optional whitespace consumed by a CSS hexadecimal escape.
    let source = format!("'a\\a{newline}0.5' 0.5px");
    // When the following value is optimized.
    let actual = optimize_value(&source);
    // Then escape whitespace is not misclassified as a bad-string boundary.
    assert_eq!(actual, format!("'a\\a{newline}0.5' .5px"));
}

#[rstest]
#[case("'0.5  ")]
#[case("\"한글 0.5 \t")]
#[case("'0.5\\\n  ")]
#[case("'0.5\\\r  ")]
#[case("'0.5\\\r\n  ")]
#[case("'0.5\\\x0c  ")]
#[case("'bad\n")]
#[case("'bad\r\n")]
fn trailing_payload_survives_when_a_value_ends_inside_a_string(#[case] source: &str) {
    // Given an EOF string or a raw bad-string boundary at the end of the value.
    let input = format!("  {source}");
    // When surrounding CSS whitespace is trimmed for optimization.
    let actual = optimize_value(&input);
    // Then payload and boundary bytes are not mistaken for outside whitespace.
    assert_eq!(actual, source);
}

#[rstest]
fn comments_are_visible_when_a_bad_string_has_ended(
    #[values("\n", "\r", "\x0c", "\r\n")] newline: &str,
) {
    // Given a real comment after a bad-string boundary.
    let input = format!("content:'bad{newline}/* gone */opacity:0.5");
    // When comments are removed.
    let actual = rm_css_comment(&input);
    // Then the comment is removed without folding away the terminating newline.
    assert_eq!(actual, format!("content:'bad{newline}opacity:0.5"));
}

#[rstest]
fn separators_are_visible_when_a_bad_string_has_ended(#[values("\n", "\r", "\x0c")] newline: &str) {
    // Given a semicolon after an unescaped newline.
    let input = format!("'bad{newline};opacity:0.5");
    // When declaration separators are scanned.
    let actual: Vec<_> = split_unquoted(&input, b';').collect();
    // Then the recovered separator divides the original byte slices.
    assert_eq!(
        actual,
        vec![format!("'bad{newline}"), "opacity:0.5".to_string()]
    );
}

#[rstest]
fn selector_lists_recover_when_a_quoted_part_hits_a_newline(
    #[values("\n", "\r", "\x0c")] newline: &str,
) {
    // Given a comma after a malformed quoted selector part.
    let source = format!("&'bad{newline}, &:hover");
    // When the selector list is split.
    let actual = split_selector_list(&source);
    // Then the list recovers and the first part keeps its raw boundary.
    assert_eq!(
        actual,
        vec![format!("&'bad{newline}"), "&:hover".to_string()]
    );
}

#[rstest]
fn preludes_recover_when_a_string_hits_a_newline(
    #[values("\n", "\r", "\x0c", "\r\n")] newline: &str,
) {
    // Given normalizable prelude syntax after a malformed string.
    let source = format!("'bad{newline}( color : red )");
    // When the prelude is normalized.
    let actual = normalize_query(&source);
    // Then only the outside syntax changes and the raw newline survives.
    assert_eq!(actual, format!("'bad{newline}(color:red)"));
}

#[test]
fn prelude_keys_keep_payload_when_the_string_reaches_eof() {
    // Given an at-rule key ending inside a string with trailing spaces.
    let key = "@supports 'open  ";
    // When its query is separated from the at-rule name.
    let actual = split_at_rule_key(key).map(|(_, query)| query);
    // Then key trimming does not discard string content.
    assert_eq!(actual, Some("'open  "));
}

#[test]
fn prelude_normalization_keeps_payload_when_the_string_reaches_eof() {
    // Given a prelude ending inside a string.
    let source = "'open  ";
    // When its outside syntax is normalized.
    let actual = normalize_query(source);
    // Then the EOF string remains byte for byte.
    assert_eq!(actual, source);
}

#[test]
fn escaped_crlf_stays_quoted_when_fragments_split_between_cr_and_lf() {
    // Given an escaped CRLF continuation split across adjacent input fragments.
    let mut state = QuoteState::default();
    // When the fragments are consumed with the same quote state.
    let actual: Vec<_> = ["'a\\\r", "\n&'", ";"]
        .into_iter()
        .flat_map(str::bytes)
        .filter(|&byte| state.is_syntax(byte))
        .collect();
    // Then neither LF nor the ampersand becomes outside syntax.
    assert_eq!(actual, b";");
}

#[test]
fn class_expansion_resumes_when_a_bad_string_has_ended() {
    // Given a nesting token after an unescaped newline.
    let selector = css::style_selector::StyleSelector::Selector("&'bad\n &".to_string());
    // When the generated class is written into the selector.
    let actual = merge_selector("cls", Some(&selector));
    // Then the recovered nesting token expands while the newline stays intact.
    assert_eq!(actual, ".cls'bad\n .cls");
}

#[test]
fn selector_lists_keep_payload_when_a_string_reaches_eof() {
    // Given a selector list ending inside a quoted string.
    let source = "&'open  ";
    // When list parts are trimmed.
    let actual = split_selector_list(source);
    // Then the EOF string's trailing spaces remain in its original slice.
    assert_eq!(actual, vec![source]);
}

#[test]
#[serial_test::serial]
fn eof_selector_payload_keeps_distinct_counter_keys() {
    // Given two selectors differing only in trailing EOF-string payload.
    let _state = css::test_state::TestStateGuard::new();
    css::debug::set_debug(false);
    css::class_map::reset_class_map();
    // When class names are assigned to their distinct style identities.
    let actual: Vec<_> = ["&'open ", "&'open  "]
        .map(|selector| {
            css::sheet_to_classname("color", 0, Some("red"), Some(selector), None, None)
        })
        .into_iter()
        .collect();
    // Then generic key trimming cannot alias the two raw string payloads.
    assert_ne!(actual[0], actual[1]);
}
