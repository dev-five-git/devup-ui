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

#[cfg(test)]
mod tests {
    use crate::{ImportAlias, import_alias_visit::transform_import_aliases_with_edits};
    use rstest::rstest;
    use std::collections::HashMap;

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
