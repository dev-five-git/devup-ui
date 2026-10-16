use crate::{ExtractOption, ImportAlias, extract, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

#[rstest]
#[serial]
fn leading_pragmas_survive_when_javascript_contains_jsx(
    #[values("js", "mjs", "cjs")] extension: &str,
    #[values(true, false)] source_map: bool,
) {
    // Given: leading pragmas attached to a Devup import in a physical JS file.
    reset_class_map();
    reset_file_map();
    let filename = format!("pragma-jsx.{extension}");
    let pragmas = [
        "/** @jsx createElement */",
        "/** @jsxFrag Fragment */",
        "/** @jsxRuntime classic */",
        "/** @jsxImportSource custom-jsx */",
    ];
    let source = format!(
        "{}\nimport {{Box}} from '@devup-ui/react';\nexport const view=<Box w='403px'/>;\nexport const anchor=1;",
        pragmas.join("\n")
    );

    // When: extraction parses JSX without changing the physical extension.
    let output = if source_map {
        extract(&filename, &source, ExtractOption::default())
    } else {
        extract_without_source_map(&filename, &source, ExtractOption::default())
    }
    .unwrap_or_else(|error| panic!("{error}"));

    // Then: the consumed import's pragmas lead real, statically styled output.
    let arena = Allocator::default();
    let parsed = crate::parsing::parse(&arena, &filename, &output.code)
        .unwrap_or_else(|error| panic!("{error}"));
    let first_statement =
        usize::try_from(parsed.body[0].span().start).unwrap_or_else(|error| panic!("{error}"));
    for pragma in pragmas {
        assert_eq!(output.code.matches(pragma).count(), 1, "{}", output.code);
        assert!(
            output.code[..first_statement].contains(pragma),
            "{}",
            output.code
        );
    }
    assert!(output.code.contains("<div className="), "{}", output.code);
    assert!(!output.code.contains("<Box"), "{}", output.code);
    assert!(!output.code.contains("import {"), "{}", output.code);
    assert_eq!(output.styles.len(), 1);
    assert!(output.styles.iter().any(|style| matches!(
        style,
        crate::extract_style::extract_style_value::ExtractStyleValue::Static(style)
            if style.property == "width" && style.value == "403px"
    )));
    if source_map {
        crate::pragma_path_tests::assert_anchor_map(&source, &output);
    } else {
        assert_eq!(output.map, None);
    }
}

#[rstest]
#[serial]
fn leading_jsx_pragmas_survive_removed_imports(
    #[values("js", "jsx", "ts", "tsx")] extension: &str,
    #[values(
        "/** @jsx createElement */",
        "/** @jsxFrag Fragment */",
        "/** @jsxRuntime classic */",
        "/** @jsxImportSource custom-jsx */"
    )]
    pragma: &str,
    #[values("// ordinary comment\n", "/*! License */\n")] leading: &str,
    #[values("before", "after", "none")] directive: &str,
    #[values(true, false)] source_map: bool,
) {
    let filename = format!("pragma.{extension}");
    let header = match directive {
        "before" => format!("{leading}'use client';\n{pragma}\n"),
        "after" => format!("{leading}{pragma}\n'use client';\n"),
        _ => format!("{leading}{pragma}\n"),
    };
    let source = format!(
        "{header}import {{css}} from '@devup-ui/react';\nexport const cls=css({{width:'401px'}});\nexport const anchor=1;"
    );
    reset_class_map();
    reset_file_map();

    let output = if source_map {
        extract(&filename, &source, ExtractOption::default())
    } else {
        extract_without_source_map(&filename, &source, ExtractOption::default())
    }
    .unwrap_or_else(|error| panic!("{error}"));

    let arena = Allocator::default();
    let parsed = Parser::new(
        &arena,
        &output.code,
        SourceType::from_path(&filename).unwrap_or_else(|error| panic!("{error}")),
    )
    .parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let first_statement = parsed.program.body[0].span().start as usize;
    assert!(
        output.code[..first_statement].contains(pragma),
        "{}",
        output.code
    );
    assert_eq!(
        parsed.program.directives.len(),
        usize::from(directive != "none")
    );
    if let Some(map) = output.map {
        let map = oxc_sourcemap::SourceMap::from_json_string(&map)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(map.get_source_content(0), Some(source.as_str()));
        let (line, generated) = output
            .code
            .lines()
            .enumerate()
            .find(|(_, text)| text.contains("anchor"))
            .unwrap_or_else(|| panic!("missing generated anchor line"));
        let column = generated
            .find("anchor")
            .unwrap_or_else(|| panic!("missing generated anchor column"));
        let token = map
            .get_tokens()
            .find(|token| {
                token.get_dst_line() as usize == line && token.get_dst_col() as usize == column
            })
            .unwrap_or_else(|| panic!("missing anchor source-map token"));
        assert_eq!(
            (token.get_src_line() as usize, token.get_src_col()),
            (header.lines().count() + 2, 13)
        );
    } else {
        assert!(!source_map);
    }
}

#[rstest]
#[serial]
fn jsx_import_source_keeps_the_enabled_emotion_compatibility_bridge(
    #[values("@emotion/react", "react", "preact", "custom")] import_source: &str,
    #[values(true, false)] emotion_enabled: bool,
) {
    let source = format!(
        "/** @jsxImportSource {import_source} */\nimport {{css}} from '@devup-ui/react';\nexport const cls=css({{width:'401px'}});\nexport const anchor=1;"
    );
    reset_class_map();
    reset_file_map();
    let aliases = if emotion_enabled {
        HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)])
    } else {
        HashMap::new()
    };

    let output = extract(
        "pragma.tsx",
        &source,
        ExtractOption {
            import_aliases: aliases,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));

    let expected = if emotion_enabled && import_source == "@emotion/react" {
        "react"
    } else {
        import_source
    };
    assert!(
        output
            .code
            .starts_with(&format!("/** @jsxImportSource {expected} */\n")),
        "{}",
        output.code
    );
    let map = oxc_sourcemap::SourceMap::from_json_string(
        output
            .map
            .as_deref()
            .unwrap_or_else(|| panic!("missing source map")),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(map.get_source_content(0), Some(source.as_str()));
}

#[test]
#[serial]
fn opted_out_emotion_only_file_is_unchanged() {
    let source = "/** @jsxImportSource @emotion/react */\nimport {css} from '@emotion/react';\nexport const cls=css({color:'red'});";

    let output = extract("pragma.tsx", source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.code, source);
    assert_eq!(output.map, None);
}

#[rstest]
#[serial]
fn enabled_emotion_pragma_is_rewritten_without_a_devup_import(
    #[values(true, false)] namespace: bool,
    #[values(true, false)] source_map: bool,
) {
    let namespace_code = if namespace {
        "import * as E from '@emotion/css'; const N=E;\n"
    } else {
        ""
    };
    let source =
        format!("/** @jsxImportSource @emotion/react */ {namespace_code}export const anchor=1;");
    reset_class_map();
    reset_file_map();
    let options = ExtractOption {
        import_aliases: HashMap::from([
            ("@emotion/react".into(), ImportAlias::NamedToNamed),
            ("@emotion/css".into(), ImportAlias::NamedToNamed),
        ]),
        ..ExtractOption::default()
    };

    let output = if source_map {
        extract("pragma.tsx", &source, options)
    } else {
        extract_without_source_map("pragma.tsx", &source, options)
    }
    .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.code.starts_with("/** @jsxImportSource react */\n"),
        "{}",
        output.code
    );
    if source_map {
        crate::pragma_path_tests::assert_anchor_map(&source, &output);
    } else {
        assert_eq!(output.map, None);
    }
}

#[test]
#[serial]
fn complete_pragma_header_keeps_directive_order() {
    let source = "/*! License */\n/** @jsxRuntime classic */\n'use client';\n/** @jsx createElement */\n/** @jsxFrag Fragment */\n/** @jsxImportSource custom-jsx */\nimport {Box,css} from '@devup-ui/react';\nexport const cls=css({width:'401px'});\nexport const box=<Box w='403px'/>;";
    reset_class_map();
    reset_file_map();

    let output = extract(
        "pragma.tsx",
        source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));

    insta::assert_snapshot!(output.code);
}
