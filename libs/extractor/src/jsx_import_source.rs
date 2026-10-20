use oxc_ast::ast::Program;
use oxc_span::GetSpan;

/// Keep the enabled Emotion alias on React after the bundler lowers JSX.
pub(crate) fn emotion_replacements(program: &Program<'_>) -> Vec<(usize, usize, String)> {
    let header_end = program
        .body
        .first()
        .map_or(program.span.end, |body| body.span().start);
    let mut replacements = Vec::new();
    for comment in &program.comments {
        if comment.span.end > header_end {
            continue;
        }
        let span = comment.content_span();
        let original = span.source_text(program.source_text);
        let text = original.trim_start_matches('*');
        for (at, _) in text.match_indices("@jsxImportSource") {
            if at > 0 && !text[..at].ends_with(char::is_whitespace) {
                continue;
            }
            let rest = &text[at + "@jsxImportSource".len()..];
            let value = rest.trim_start();
            if value.len() == rest.len() {
                continue;
            }
            if let Some(after) = value.strip_prefix("@emotion/react")
                && after.chars().next().is_none_or(char::is_whitespace)
            {
                let start = span.start as usize + original.len() - text.len()
                    + at
                    + "@jsxImportSource".len()
                    + rest.len()
                    - value.len();
                replacements.push((start, start + "@emotion/react".len(), "react".into()));
            }
        }
    }
    replacements
}

/// Redirect project-wide Emotion JSX unless an exact header instruction overrides it.
pub(crate) fn project_replacement(program: &Program<'_>) -> Option<(usize, usize, String)> {
    let header_end = program
        .body
        .first()
        .map_or(program.span.end, |body| body.span().start);
    for comment in &program.comments {
        if comment.span.end > header_end {
            continue;
        }
        let mut tokens = comment
            .content_span()
            .source_text(program.source_text)
            .trim_start_matches('*')
            .split_whitespace()
            .peekable();
        while let Some(token) = tokens.next() {
            if matches!(
                token,
                "@jsx" | "@jsxFrag" | "@jsxRuntime" | "@jsxImportSource"
            ) && tokens.peek().is_some_and(|value| *value != "*")
            {
                return None;
            }
        }
    }
    let pragma = crate::css_prop::REACT_JSX_PRAGMA;
    let (start, text) = match &program.hashbang {
        Some(hashbang) => {
            let end = hashbang.span.end as usize;
            let rest = &program.source_text[end..];
            if rest.starts_with("\r\n") {
                (end + 2, pragma.to_string())
            } else {
                match rest.chars().next() {
                    Some(terminator @ ('\r' | '\n' | '\u{2028}' | '\u{2029}')) => {
                        (end + terminator.len_utf8(), pragma.to_string())
                    }
                    Some(_) | None => (end, format!("\n{pragma}")),
                }
            }
        }
        None => (0, pragma.to_string()),
    };
    Some((start, start, text))
}

#[cfg(test)]
#[path = "jsx_import_source_terminator_tests.rs"]
mod terminator_tests;

#[cfg(test)]
mod tests {
    use crate::{ImportAlias, import_alias_visit::transform_import_aliases_with_edits};
    use rstest::rstest;
    use std::collections::HashMap;

    #[rstest]
    #[case::plain("", true)]
    #[case::embedded("/* not@jsxImportSource custom */\n", true)]
    #[case::joined("/* @jsxImportSourcecustom */\n", true)]
    #[case::suffix("/* @jsxFactory h */\n", true)]
    #[case::empty("/* @jsxImportSource */\n", true)]
    #[case::doc_empty("/** @jsxImportSource\n * documentation */\n", true)]
    #[case::post_body("export const anchor=1;\n/* @jsxImportSource custom */\n", true)]
    #[case::source("/** @jsxImportSource custom */\n", false)]
    #[case::factory("/**@jsx h*/\n", false)]
    #[case::fragment("/** @jsxFrag Fragment */\n", false)]
    #[case::runtime("/** @jsxRuntime classic */\n", false)]
    #[case::directive("'use client';\n/* @jsxImportSource custom */\n", false)]
    fn project_jsx_uses_only_exact_header_overrides(#[case] header: &str, #[case] inserted: bool) {
        // Given: project-wide Emotion JSX plus a real or lookalike file override.
        let source = format!("{header}export const view=<Custom css={{{{padding:2}}}} />;");
        let aliases = HashMap::from([
            ("@emotion/react".into(), ImportAlias::NamedToNamed),
            (
                "@emotion/react/jsx-runtime".into(),
                ImportAlias::NamedToNamed,
            ),
        ]);

        // When: aliases compile JSX's css prop before the bundler lowers JSX.
        let rewritten = transform_import_aliases_with_edits(
            &source,
            "project.tsx",
            "@devup-ui/react",
            &aliases,
        );

        // Then: only an actual header instruction suppresses the React default.
        assert_eq!(
            rewritten
                .code
                .starts_with(crate::css_prop::REACT_JSX_PRAGMA),
            inserted
        );
        assert_eq!(rewritten.css_prop, crate::css_prop::CssProp::Everywhere);
        assert!(
            rewritten.code.contains("padding:\"2px\""),
            "{}",
            rewritten.code
        );
    }

    #[test]
    fn project_jsx_inserts_after_hashbang_and_maps_authored_offsets() -> Result<(), String> {
        // Given: an executable JSX source with a directive and project-only Emotion.
        let source = "#!/usr/bin/env node\n'use client';\nexport const view=<div />;";
        let aliases = HashMap::from([
            ("@emotion/react".into(), ImportAlias::NamedToNamed),
            (
                "@emotion/react/jsx-runtime".into(),
                ImportAlias::NamedToNamed,
            ),
        ]);

        // When: the project default is redirected to React.
        let rewritten =
            transform_import_aliases_with_edits(source, "project.jsx", "@devup-ui/react", &aliases);

        // Then: the hashbang stays first and every authored byte keeps its origin.
        assert_eq!(
            rewritten.code,
            "#!/usr/bin/env node\n/** @jsxImportSource react */\n'use client';\nexport const view=<div />;"
        );
        let authored = source.find("export").ok_or("missing authored export")?;
        let emitted = rewritten
            .code
            .find("export")
            .ok_or("missing emitted export")?;
        assert_eq!(
            crate::import_alias_visit::source_offset(&rewritten.edits, emitted),
            authored
        );
        Ok(())
    }

    #[test]
    fn project_jsx_terminates_a_hashbang_at_end_of_file() {
        // Given: a project JSX file containing only an unterminated hashbang line.
        let source = "#!/usr/bin/env node";
        let aliases = HashMap::from([
            ("@emotion/react".into(), ImportAlias::NamedToNamed),
            (
                "@emotion/react/jsx-runtime".into(),
                ImportAlias::NamedToNamed,
            ),
        ]);

        // When: alias preprocessing redirects the project default.
        let rewritten =
            transform_import_aliases_with_edits(source, "project.jsx", "@devup-ui/react", &aliases);

        // Then: the pragma starts on its own line, outside the hashbang.
        assert_eq!(
            rewritten.code,
            "#!/usr/bin/env node\n/** @jsxImportSource react */\n"
        );
    }

    #[rstest]
    #[case::plain("")]
    #[case::hashbang("#!/usr/bin/env node\n")]
    #[case::directive("'use client';\n")]
    #[serial_test::serial]
    fn project_jsx_preserves_original_parser_locations(#[case] header: &str) -> Result<(), String> {
        // Given: project-only JSX with a recoverable invalid attribute.
        let source = format!("{header}export const view=<div css={{}} />;");
        let option = crate::ExtractOption {
            import_aliases: HashMap::from([
                ("@emotion/react".into(), ImportAlias::NamedToNamed),
                (
                    "@emotion/react/jsx-runtime".into(),
                    ImportAlias::NamedToNamed,
                ),
            ]),
            ..crate::ExtractOption::default()
        };

        // When: the public extractor inserts the project pragma before parsing.
        let error = crate::extract("project.tsx", &source, option)
            .err()
            .ok_or("expected strict parsing error")?
            .to_string();

        // Then: strict parsing rejects it at its authored line, not the inserted line.
        let line = header.lines().count() + 1;
        assert_eq!(
            error,
            format!(
                "project.tsx:{line}:28: JSX attributes must only be assigned a non-empty 'expression'."
            )
        );
        Ok(())
    }

    #[rstest]
    #[case(
        "/** @jsxImportSource @emotion/react */",
        "/** @jsxImportSource react */"
    )]
    #[case("/*@jsxImportSource\t@emotion/react*/", "/*@jsxImportSource\treact*/")]
    #[case("// @jsxImportSource @emotion/react", "// @jsxImportSource react")]
    #[case(
        "/* @jsxImportSource @emotion/reactive */",
        "/* @jsxImportSource @emotion/reactive */"
    )]
    #[case(
        "/* not@jsxImportSource @emotion/react */",
        "/* not@jsxImportSource @emotion/react */"
    )]
    #[case(
        "/* @jsxImportSource@emotion/react */",
        "/* @jsxImportSource@emotion/react */"
    )]
    #[case(
        "/* @jsxImportSource @emotion/react more */",
        "/* @jsxImportSource react more */"
    )]
    #[case(
        "'use client'; // @jsxImportSource @emotion/react",
        "'use client'; // @jsxImportSource react"
    )]
    #[case(
        "/* text*@jsxImportSource @emotion/react */",
        "/* text*@jsxImportSource @emotion/react */"
    )]
    fn emotion_source_rewrite_uses_exact_comment_tokens(
        #[case] header: &str,
        #[case] expected: &str,
    ) {
        let source = format!(
            "{header}\nexport const anchor=1;\n/** @jsxImportSource @emotion/react */\nexport const later=2;"
        );

        let crate::import_alias_visit::Aliased {
            code: rewritten, ..
        } = transform_import_aliases_with_edits(
            &source,
            "pragma.tsx",
            "@devup-ui/react",
            &HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        );

        assert_eq!(
            rewritten,
            format!(
                "{expected}\nexport const anchor=1;\n/** @jsxImportSource @emotion/react */\nexport const later=2;"
            )
        );
    }
}
