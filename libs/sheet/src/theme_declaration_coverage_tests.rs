use super::theme::Theme;

#[test]
fn unknown_typography_preset_has_no_declarations() {
    // Given: a valid theme containing a different preset.
    let theme: Theme = serde_json::from_str(r#"{"typography":{"body":{"fontSize":"14px"}}}"#)
        .unwrap_or_else(|error| panic!("theme fixture: {error}"));
    // When: an absent preset is expanded from a nonzero breakpoint.
    let declarations = theme.typography_declarations("missing", 2);
    // Then: no fallback preset or partial declaration leaks into the result.
    assert_eq!(declarations, vec![]);
}

#[test]
fn typography_collapsed_breakpoints_keep_last_value_and_later_entries() {
    // Given: overlapping declarations, a sparse gap, a token, and a wider entry.
    let theme: Theme = serde_json::from_str(
        r#"{
        "typography":{"body":[
            {"fontSize":"14px","fontFamily":"$body","lineHeight":" 1.5 "},
            {"fontSize":"16px","lineHeight":" "},
            null,
            {"fontSize":"20px"}
        ]}
    }"#,
    )
    .unwrap_or_else(|error| panic!("theme fixture: {error}"));
    // When: applying at level two folds earlier entries into that breakpoint.
    let declarations = theme.typography_declarations("body", 2);
    // Then: later values overwrite only their own property, and wider entries remain wider.
    assert_eq!(
        declarations,
        vec![
            (2, "font-family", "var(--body)".into()),
            (2, "font-size", "16px".into()),
            (2, "line-height", "1.5".into()),
            (3, "font-size", "20px".into()),
        ]
    );
}
