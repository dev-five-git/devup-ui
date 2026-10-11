use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

use super::super::{Selection, plan::EscapeKind};

fn selected(source: &str, package: &str) -> Selection {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    super::super::select_for_package(&parsed.program, &built.semantic, package)
}

#[rstest]
#[case("du.globalCss({body:{color:'orange'}});")]
#[case("du['globalCss']({body:{color:'orange'}});")]
#[case("export const base=du.css({color:'orange',p:2});")]
#[case("export const base=du['css']({color:'orange',p:2});")]
#[serial]
fn ordinary_members_are_safe_when_the_partial_namespace_owner_is_unselected(#[case] usage: &str) {
    // Given
    let source = format!("import * as du from '@devup-ui/react';{usage}");
    // When
    let plan = selected(&source, "@devup-ui/react");
    // Then
    assert_eq!(plan.roots.len(), 0);
    assert_eq!(plan.native_calls.len(), 0);
    assert_eq!(plan.escapes.len(), 0);
    assert_eq!(plan.checks.len(), 0);
    assert_eq!(plan.imports.len(), 1);
    assert!(plan.imports[0].preserved);
}

#[rstest]
#[case(
    "@devup-ui/react",
    "export function render(){return du.style({color:'red'})}",
    EscapeKind::NativeValue
)]
#[case(
    "@devup-ui/react",
    "export function render(){return du['style']({color:'red'})}",
    EscapeKind::NativeValue
)]
#[case("@devup-ui/react", "du.style=()=>'';", EscapeKind::NativeValue)]
#[case("@devup-ui/react", "register(du.style);", EscapeKind::NativeValue)]
#[case("@devup-ui/react", "du[key]({});", EscapeKind::DynamicNamespace)]
#[case("@vanilla-extract/css", "du.globalCss({});", EscapeKind::NativeValue)]
#[serial]
fn native_uses_are_refused_when_no_exact_initialization_owner_allows_them(
    #[case] package: &str,
    #[case] usage: &str,
    #[case] kind: EscapeKind,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!("import * as du from '{package}';{usage}");
    let start = source.find(usage).ok_or("authored usage missing")?;
    let offset = u32::try_from(start + usage.find("du").ok_or("namespace reference missing")?)?;
    // When
    let plan = selected(&source, package);
    // Then: the namespace reference itself is rejected, not just its exported helper.
    assert!(
        plan.escapes.iter().any(|escape| {
            escape.kind == kind
                && escape.span.start == offset
                && escape.span.source_text(&source) == "du"
        }),
        "{:#?}",
        plan.escapes
    );
    Ok(())
}
