use crate::ExtractOutput;
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

pub(super) const PRAGMAS: [&str; 4] = [
    "/** @jsx h */",
    "// @jsxFrag Fragment",
    "/* @jsxRuntime automatic */",
    "/** @jsxImportSource @emotion/react */",
];

pub(super) fn reset() {
    reset_class_map();
    reset_file_map();
}

pub(super) fn assert_header(filename: &str, output: &ExtractOutput, pragmas: &[&str]) {
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        &output.code,
        SourceType::from_path(filename).unwrap_or_else(|error| panic!("{error}")),
    )
    .parse();
    assert!(parsed.diagnostics.is_empty(), "{}", output.code);
    let end = parsed
        .program
        .body
        .first()
        .map_or(output.code.len(), |node| {
            usize::try_from(node.span().start).unwrap_or_else(|error| panic!("{error}"))
        });
    for pragma in pragmas {
        assert_eq!(output.code.matches(pragma).count(), 1, "{}", output.code);
        assert!(output.code[..end].contains(pragma), "{}", output.code);
    }
}

pub(crate) fn assert_anchor_map(source: &str, output: &ExtractOutput) {
    let map = oxc_sourcemap::SourceMap::from_json_string(
        output
            .map
            .as_deref()
            .unwrap_or_else(|| panic!("missing source map")),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(map.get_source_content(0), Some(source));
    let (line, text) = output
        .code
        .lines()
        .enumerate()
        .find(|(_, text)| text.contains("anchor"))
        .unwrap_or_else(|| panic!("missing generated anchor line"));
    let column = text
        .find("anchor")
        .unwrap_or_else(|| panic!("missing generated anchor column"));
    let token = map
        .get_tokens()
        .find(|token| {
            usize::try_from(token.get_dst_line()).unwrap_or_else(|error| panic!("{error}")) == line
                && usize::try_from(token.get_dst_col()).unwrap_or_else(|error| panic!("{error}"))
                    == column
        })
        .unwrap_or_else(|| panic!("missing anchor source-map token"));
    let (original_line, original) = source
        .lines()
        .enumerate()
        .find(|(_, text)| text.contains("anchor"))
        .unwrap_or_else(|| panic!("missing original anchor line"));
    assert_eq!(
        usize::try_from(token.get_src_line()).unwrap_or_else(|error| panic!("{error}")),
        original_line
    );
    assert_eq!(
        usize::try_from(token.get_src_col()).unwrap_or_else(|error| panic!("{error}")),
        original
            .find("anchor")
            .unwrap_or_else(|| panic!("missing original anchor column"))
    );
}
