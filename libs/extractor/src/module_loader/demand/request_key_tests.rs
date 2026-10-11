use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};
use rstest::rstest;

use super::{Demand, entry};

fn requests(source: &str) -> Vec<(String, Demand, Span)> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{:?}", parsed.diagnostics);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .with_check_syntax_error(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{:?}", built.diagnostics);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    entry((&parsed.program, &built.semantic), false, &selection)
}

fn import_site(source: &str) -> Span {
    let marker = "'./data'";
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("authored import marker"));
    let end = start + marker.len();
    Span::new(
        u32::try_from(start).unwrap_or_else(|_| panic!("authored import start does not fit span")),
        u32::try_from(end).unwrap_or_else(|_| panic!("authored import end does not fit span")),
    )
}

fn native_site(source: &str) -> Span {
    let marker = "'@vanilla-extract/css'";
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("authored native import marker"));
    let end = start + marker.len();
    Span::new(
        u32::try_from(start).unwrap_or_else(|_| panic!("authored native start does not fit span")),
        u32::try_from(end).unwrap_or_else(|_| panic!("authored native end does not fit span")),
    )
}

#[rstest]
#[case("", "'space'", "space")]
#[case("", "0", "0")]
#[case("const key='space';", "key", "space")]
#[case("const first='space';const key=first;", "key", "space")]
#[case("const key=('space' as const);", "(key as string)", "space")]
#[case("const key=0;", "key", "0")]
fn precise_member_requests_when_the_key_is_known(
    #[case] declaration: &str,
    #[case] member: &str,
    #[case] key: &str,
) {
    // Given
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';import {{palette}} from './data';{declaration}export const box=style({{margin:palette[{member}]}});"
    );
    let expected = Demand::prefixed("palette", &Demand::prefixed(key, &Demand::whole()));
    let native = (
        "@vanilla-extract/css".into(),
        Demand::prefixed("style", &Demand::whole()),
        native_site(&source),
    );
    // When
    let actual = requests(&source);
    // Then
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0], native);
    assert_eq!(native.2, Span::new(20, 42));
    assert_eq!(
        actual,
        vec![native, ("./data".into(), expected, import_site(&source))]
    );
}

#[rstest]
#[case("let key='space';", "")]
#[case("const key='space';key='browser';", "")]
#[case("", "const key='space';")]
#[case("const key=key;", "")]
fn whole_palette_requests_when_the_key_cannot_be_proven(#[case] before: &str, #[case] after: &str) {
    // Given
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';import {{palette}} from './data';{before}export const box=style({{margin:palette[key]}});{after}"
    );
    let expected = Demand::prefixed("palette", &Demand::whole());
    let native = (
        "@vanilla-extract/css".into(),
        Demand::prefixed("style", &Demand::whole()),
        native_site(&source),
    );
    // When
    let actual = requests(&source);
    // Then
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0], native);
    assert_eq!(native.2, Span::new(20, 42));
    assert_eq!(
        actual,
        vec![native, ("./data".into(), expected, import_site(&source))]
    );
}
