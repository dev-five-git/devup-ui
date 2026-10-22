use css::{
    add_selector_params,
    at_rule::{MediaCombination, combine_media_queries, normalize_query},
    merge_selector,
    optimize_value::optimize_value,
    style_selector::{AtRuleKind, StyleSelector},
};
use rstest::rstest;

#[rstest]
#[case(r#"&[title="&"]"#, r#"&:hover[title="&"]"#)]
#[case(r#"[title="&"] > &"#, r#"[title="&"] > &:hover"#)]
#[case(r"&[title='a\'&']", r"&:hover[title='a\'&']")]
fn nested_selector_keeps_quoted_ampersands(#[case] template: &str, #[case] expected: &str) {
    let parent = StyleSelector::Selector("&:hover".to_string());
    let actual = StyleSelector::nest_selector(Some(&parent), template);
    assert_eq!(actual.to_string(), expected);
}

#[test]
fn nested_media_keeps_quoted_parentheses_and_commas() {
    let actual = combine_media_queries("print", r#"(name:"a),b")"#);
    assert_eq!(
        actual,
        MediaCombination::Merged(r#"print and (name:"a),b")"#.to_string())
    );
}

#[rstest]
#[case("'bad\n0.5px", "'bad\n.5px")]
#[case("'bad\r0.5px", "'bad\r.5px")]
#[case("'bad\x0c0.5px", "'bad\x0c.5px")]
#[case("'0.5' 0.5px", "'0.5' .5px")]
fn bad_string_ends_at_an_unescaped_css_newline(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(optimize_value(input), expected);
}

#[rstest]
#[case(r#"&[title="a&b"]"#, r#".cls[title="a&b"]"#)]
#[case(r"&.a\&b", r".cls.a\&b")]
#[case(r#"&.a\" > &"#, r#".cls.a\" > .cls"#)]
#[case(r#"&[title="a\"&b"] > &"#, r#".cls[title="a\"&b"] > .cls"#)]
fn merge_preserves_literal_ampersands_when_emitting_a_selector(
    #[case] template: &str,
    #[case] expected: &str,
    #[values(false, true)] in_media: bool,
) {
    // Given a selector emitted directly or within an at-rule.
    let selector = if in_media {
        StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "print".to_string(),
            selector: Some(template.to_string()),
            outer: vec![],
            file: None,
        }
    } else {
        StyleSelector::Selector(template.to_string())
    };
    // When the generated class replaces nesting tokens.
    let actual = merge_selector("cls", Some(&selector));
    // Then quoted and escaped ampersands remain data.
    assert_eq!(actual, expected);
}

#[test]
fn nesting_preserves_strings_when_the_parent_is_a_selector_list() {
    // Given a list parent and a template with real and literal ampersands.
    let parent = StyleSelector::Selector("&:hover,&:focus".to_string());
    // When nesting distributes over the parent list.
    let actual = StyleSelector::nest_selector(Some(&parent), r#"&[title="&"] .a\&b"#);
    // Then only the nesting token is substituted in each branch.
    assert_eq!(
        actual.to_string(),
        r#"&:hover[title="&"] .a\&b,&:focus[title="&"] .a\&b"#
    );
}

#[test]
fn params_skip_punctuation_when_it_is_quoted() {
    // Given a real pseudo before an attribute containing fake closing syntax.
    let selector = StyleSelector::Selector(r#"&:nth-child[title="]:not"]"#.to_string());
    // When parameters are attached to the unparameterized pseudo.
    let actual = add_selector_params(selector, "2n");
    // Then the quoted pseudo is untouched.
    assert_eq!(actual.to_string(), r#"&:nth-child(2n)[title="]:not"]"#);
}

#[rstest]
#[case(r#"(name:"a) or b")"#, r#"print and (name:"a) or b")"#)]
#[case(r#"(name:"a),b")"#, r#"print and (name:"a),b")"#)]
#[case(
    r#"(name:"a),b"),(color)"#,
    r#"print and (name:"a),b"),print and (color)"#
)]
fn media_combination_ignores_structure_when_it_is_quoted(
    #[case] inner: &str,
    #[case] expected: &str,
) {
    // Given a condition/list containing quoted structure.
    // When a media type is folded into the condition.
    let actual = combine_media_queries("print", inner);
    // Then quoted punctuation never splits or wraps the condition.
    assert_eq!(actual, MediaCombination::Merged(expected.to_string()));
}

#[test]
fn prelude_normalizes_structure_when_a_quote_is_externally_escaped() {
    // Given a quote that is an escaped token, not a string opener.
    let query = r#"selector(.a\") and ( color : red )"#;
    // When whitespace outside strings is normalized.
    let actual = normalize_query(query);
    // Then the remaining prelude is not hidden behind a false quote.
    assert_eq!(actual, r#"selector(.a\")and (color:red)"#);
}

#[test]
fn escaped_quotes_keep_following_comments_visible() {
    // Given an escaped quote outside a string before an actual comment.
    let source = r#"a\"   b /* remove */ c"#;
    // When comments and outside whitespace are normalized.
    let actual = css::rm_css_comment::rm_css_comment(source);
    // Then the escaped quote does not hide the comment or whitespace.
    assert_eq!(actual, r#"a\" b c"#);
}

#[test]
fn escaped_quotes_keep_following_numbers_optimizable() {
    // Given an escaped quote outside a string before a number.
    let source = r#"\" 0.5"#;
    // When numeric CSS optimization runs.
    let actual = optimize_value(source);
    // Then the number is not falsely protected as string data.
    assert_eq!(actual, r#"\" .5"#);
}
