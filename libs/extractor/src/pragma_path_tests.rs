mod support;

pub(super) use support::assert_anchor_map;
use support::{PRAGMAS, assert_header, reset};

use crate::{
    ExtractOption, ImportAlias, ModuleResolution, ResolvedModule, extract, extract_with_modules,
};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

#[rstest]
#[case("ts")]
#[case("tsx")]
#[serial]
fn styled_headers_survive_when_helper_imports_are_inserted(#[case] extension: &str) {
    // Given: headers on either side of the directive and a compiled styled renderer.
    reset();
    let filename = format!("styled-pragma.{extension}");
    let source = format!(
        "{}\n'use client';\n{}\nimport {{styled}} from '@devup-ui/react';\nexport const Card=styled('div', {{width:'407px'}});\nexport const anchor=1;",
        PRAGMAS[0],
        PRAGMAS[1..].join("\n")
    );
    // When: extraction inserts the renderer helper and stylesheet import.
    let output = extract(&filename, &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    // Then: the headers and directive remain ahead of all module body statements.
    assert_header(&filename, &output, &PRAGMAS);
    assert!(
        output
            .code
            .find(PRAGMAS[0])
            .unwrap_or_else(|| panic!("missing JSX pragma"))
            < output
                .code
                .find("use client")
                .unwrap_or_else(|| panic!("missing use client directive"))
    );
    assert!(
        output
            .code
            .find("use client")
            .unwrap_or_else(|| panic!("missing use client directive"))
            < output
                .code
                .find(PRAGMAS[1])
                .unwrap_or_else(|| panic!("missing JSX fragment pragma"))
    );
    assert_anchor_map(&source, &output);
}

#[test]
#[serial]
fn namespace_headers_survive_when_no_target_import_remains() {
    // Given: a namespace alias that normalization removes without generating styles.
    reset();
    let source = format!(
        "{}\n'use client';\nimport * as E from '@emotion/css';\nconst N=E;\nexport const anchor=1;",
        PRAGMAS.join("\n")
    );
    let options = ExtractOption {
        import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When: the namespace-only output seam emits the rewritten module.
    let output =
        extract("namespace-pragma.tsx", &source, options).unwrap_or_else(|error| panic!("{error}"));
    // Then: headers precede the surviving module with original-source mappings.
    assert_header("namespace-pragma.tsx", &output, &PRAGMAS);
    assert!(output.css_file.is_none());
    assert_anchor_map(&source, &output);
}

#[rstest]
#[case("ts")]
#[case("js")]
#[serial]
fn stylesheet_headers_survive_when_source_is_synthesized(
    #[case] extension: &str,
    #[values(true, false)] emotion_enabled: bool,
) {
    // Given: a stylesheet with an authored pragma and a retained side-effect import.
    reset();
    let filename = format!("pragma.css.{extension}");
    let source = format!(
        "{}\nimport './reset.css';\nimport {{style}} from '@vanilla-extract/css';\nexport const cls=style({{width:'409px'}});",
        PRAGMAS.join("\n")
    );
    let mut aliases = HashMap::from([("@vanilla-extract/css".into(), ImportAlias::NamedToNamed)]);
    if emotion_enabled {
        aliases.insert("@emotion/react".into(), ImportAlias::NamedToNamed);
    }
    let options = ExtractOption {
        import_aliases: aliases,
        ..ExtractOption::default()
    };
    // When: collection replaces the authored stylesheet with generated exports.
    let output = extract(&filename, &source, options).unwrap_or_else(|error| panic!("{error}"));
    // Then: headers precede retained imports and the map still targets generated source.
    let expected = PRAGMAS.map(|pragma| {
        if emotion_enabled && pragma == PRAGMAS[3] {
            "/** @jsxImportSource react */"
        } else {
            pragma
        }
    });
    assert_header(&filename, &output, &expected);
    assert!(output.code.contains("./reset.css"));
    assert!(!output.styles.is_empty());
    let map = oxc_sourcemap::SourceMap::from_json_string(
        output
            .map
            .as_deref()
            .unwrap_or_else(|| panic!("missing source map")),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let generated = map
        .get_source_content(0)
        .unwrap_or_else(|| panic!("missing generated source content"));
    assert!(generated.contains(PRAGMAS[0]));
    assert!(!generated.contains("@vanilla-extract/css"));
}

#[rstest]
#[case("")]
#[case("'use client';\n")]
#[serial]
fn headers_survive_when_the_only_import_is_consumed(#[case] directive: &str) {
    // Given: a hashbang and pragmas surrounding an optional directive.
    reset();
    let source = format!(
        "#!/usr/bin/env node\n{}\n{directive}{}\nimport {{css}} from '@devup-ui/react';",
        PRAGMAS[0],
        PRAGMAS[1..].join("\n")
    );
    // When: extraction removes the only body statement.
    let output = extract("empty-pragma.ts", &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    // Then: hashbang is first and the remaining directive prologue is intact.
    assert!(output.code.starts_with("#!/usr/bin/env node\n"));
    assert_header("empty-pragma.ts", &output, &PRAGMAS);
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::ts()).parse();
    assert_eq!(
        parsed.program.directives.len(),
        usize::from(!directive.is_empty())
    );
    assert!(parsed.program.body.is_empty());
}

#[test]
#[serial]
fn hashbang_remains_first_when_a_synthetic_import_takes_the_header() {
    // Given: a hashbang before the pragma attached to a compiled API import.
    reset();
    let source = format!(
        "#!/usr/bin/env node\n{}\n'use client';\n{}\nimport {{css}} from '@devup-ui/react';\nexport const cls=css({{width:'419px'}});\nexport const anchor=1;",
        PRAGMAS[0],
        PRAGMAS[1..].join("\n")
    );
    // When: the consumed import is replaced by a synthetic stylesheet import.
    let output = extract("hashbang-pragma.ts", &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    // Then: Oxc accounts for header emission after the hashbang in its mappings.
    assert!(output.code.starts_with("#!/usr/bin/env node\n"));
    assert_header("hashbang-pragma.ts", &output, &PRAGMAS);
    assert_anchor_map(&source, &output);
}

#[rstest]
#[case(PRAGMAS.join("\n"))]
#[case(String::new())]
#[serial]
fn headers_survive_when_imported_constants_require_evaluation(#[case] header: String) {
    // Given: a style value that evaluates a local function with an imported constant.
    reset();
    let source = format!(
        "{header}\nimport {{css}} from '@devup-ui/react';\nimport {{SIZE}} from './size';\nfunction twice(n){{return n*2}}\nexport const cls=css({{width:twice(SIZE)+'px'}});\nexport const anchor=1;"
    );
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./size").then(|| {
            ModuleResolution::Resolved(ResolvedModule {
                path: "/pragma-size.ts".into(),
                code: "export const SIZE=5;".into(),
                source_type: None,
            })
        })
    };
    // When: extraction re-enters after build-time evaluation rewrites the call.
    let output = extract_with_modules(
        "/pragma-app.ts",
        &source,
        ExtractOption::default(),
        true,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then: headers are emitted once and the surviving declaration maps to authored input.
    if !header.is_empty() {
        assert_header("/pragma-app.ts", &output, &PRAGMAS);
    }
    assert_anchor_map(&source, &output);
    assert!(!output.styles.is_empty());
}

#[rstest]
#[case("", "export const x=1;")]
#[case("/** @jsx h */", "")]
#[case("const value='@jsx';", "export const x=1;")]
fn stylesheet_generation_is_unchanged_when_no_header_is_carried(
    #[case] source: &str,
    #[case] generated: &str,
) {
    // Given: empty generation or source without an actual header pragma.
    // When: constructing the stylesheet intermediate source.
    let output = crate::jsx_pragmas::stylesheet_source(source, generated.into(), SourceType::ts());
    // Then: generated bytes retain their existing contract.
    assert_eq!(output, generated);
}

#[test]
#[serial]
fn body_comments_and_near_misses_stay_out_of_the_header() {
    // Given: an exact compact pragma, similar tokens, strings, and a body comment.
    reset();
    let source = "/**@jsx h*/\n/* @jsxFactory nope */\n/* @jsxFragExtra nope */\n/* @jsxRuntimeExtra nope */\n/* @jsxImportSourceExtra nope */\n/* @jsx* nope */\nimport {css} from '@devup-ui/react';\nexport const text='@jsxImportSource fake';\n/** @jsxFrag BodyFragment */\nexport const cls=css({width:'421px'});";
    // When: extraction removes the import that anchored the header comments.
    let output = extract("negative-pragma.ts", source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    // Then: only the exact authored header directive is hoisted.
    assert_header("negative-pragma.ts", &output, &["/**@jsx h*/"]);
    let first_import = output
        .code
        .find("import ")
        .unwrap_or_else(|| panic!("missing generated import"));
    for near_miss in [
        "@jsxFactory",
        "@jsxFragExtra",
        "@jsxRuntimeExtra",
        "@jsxImportSourceExtra",
        "@jsx*",
    ] {
        assert!(
            !output.code[..first_import].contains(near_miss),
            "{}",
            output.code
        );
    }
    assert!(!output.code[..first_import].contains("fake"));
    assert!(
        output
            .code
            .find("BodyFragment")
            .unwrap_or_else(|| panic!("missing body fragment comment"))
            > output
                .code
                .find("export const text")
                .unwrap_or_else(|| panic!("missing text export"))
    );
}
