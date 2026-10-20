use crate::{ExtractOption, ImportAlias, import_alias_visit::transform_import_aliases_with_edits};
use rstest::rstest;
use std::collections::HashMap;

#[rstest]
#[case::cr("\r")]
#[case::crlf("\r\n")]
#[case::lf("\n")]
#[case::line_separator("\u{2028}")]
#[case::paragraph_separator("\u{2029}")]
fn project_jsx_preserves_adjacent_hashbang_terminator_and_offsets(
    #[case] terminator: &str,
) -> Result<(), String> {
    // Given: a different later LF must not determine the insertion point.
    let source = format!("#!/usr/bin/env node{terminator}export const view=<div />;\n");
    let aliases = HashMap::from([
        ("@emotion/react".into(), ImportAlias::NamedToNamed),
        (
            "@emotion/react/jsx-runtime".into(),
            ImportAlias::NamedToNamed,
        ),
    ]);

    // When: project-only JSX is redirected before bundler lowering.
    let rewritten =
        transform_import_aliases_with_edits(&source, "project.jsx", "@devup-ui/react", &aliases);

    // Then: insertion consumes only the adjacent terminator; authored bytes map back exactly.
    assert_eq!(
        rewritten.code,
        format!(
            "#!/usr/bin/env node{terminator}/** @jsxImportSource react */\nexport const view=<div />;\n"
        )
    );
    let emitted = rewritten
        .code
        .find("export")
        .ok_or("missing emitted export")?;
    let authored = source.find("export").ok_or("missing authored export")?;
    for offset in 0.."export const view=<div />;\n".len() {
        assert_eq!(
            crate::import_alias_visit::source_offset(&rewritten.edits, emitted + offset),
            authored + offset
        );
    }
    Ok(())
}

#[rstest]
#[case::cr("\r")]
#[case::crlf("\r\n")]
#[case::lf("\n")]
#[case::line_separator("\u{2028}")]
#[case::paragraph_separator("\u{2029}")]
#[serial_test::serial]
fn project_jsx_keeps_public_pragma_in_header_after_mixed_terminators(
    #[case] terminator: &str,
) -> Result<(), String> {
    // Given: importless project JSX with a hashbang and a later LF.
    let source = format!("#!/usr/bin/env node{terminator}export const view=<div />;\n");
    let option = ExtractOption {
        import_aliases: HashMap::from([
            ("@emotion/react".into(), ImportAlias::NamedToNamed),
            (
                "@emotion/react/jsx-runtime".into(),
                ImportAlias::NamedToNamed,
            ),
        ]),
        ..ExtractOption::default()
    };

    // When: the public extractor parses and regenerates the source.
    let output =
        crate::extract("project.jsx", &source, option).map_err(|error| error.to_string())?;

    // Then: a compiler-visible header instruction precedes the first body statement.
    let pragma = output
        .code
        .find("/** @jsxImportSource react */")
        .ok_or("missing project pragma")?;
    let body = output
        .code
        .find("export const view")
        .ok_or("missing export")?;
    assert!(pragma < body, "{}", output.code);
    assert!(output.code.starts_with("#!/usr/bin/env node"));
    Ok(())
}
