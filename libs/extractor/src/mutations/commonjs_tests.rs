use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

use super::{Use, uses};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq)]
enum ObservedUse {
    Changes(u32, usize),
    Escapes(u32, Vec<Option<String>>, Option<String>),
    Calls(u32, Vec<Option<String>>),
}

fn observed(source: &str) -> Vec<(String, ObservedUse)> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    uses(&parsed.program, &built.semantic)
        .into_iter()
        .map(|(name, usage)| {
            let usage = match usage {
                Use::Changes { at, depth } => ObservedUse::Changes(at, depth),
                Use::Escapes { at, path, into } => ObservedUse::Escapes(at, path, into),
                Use::Calls { at, path } => ObservedUse::Calls(at, path),
            };
            (name, usage)
        })
        .collect()
}

fn offset(source: &str, read: &str) -> TestResult<u32> {
    Ok(u32::try_from(
        source.find(read).ok_or("fixture read missing")?,
    )?)
}

#[rstest]
#[case("import './side';module['exports'].tokens.space=8;")]
#[case("export const marker=1;external(module['exports'].tokens);")]
#[case("export {};exports.tokens.space=8;")]
fn carrier_uses_are_excluded_when_program_has_esm_declaration(#[case] source: &str) {
    // Given
    let source = format!("// 한글\r\n{source}");
    // When
    let found = observed(&source);
    // Then
    assert_eq!(found, vec![]);
}

#[rstest]
#[case("module['exports'].tokens.space=8;")]
#[case("module['exports'].tokens.space++;")]
#[case("delete module['exports'].tokens.space;")]
#[case("Object.assign(module['exports'].tokens,{});")]
#[case("module['exports'].tokens.push(8);")]
fn changes_are_normalized_when_global_carrier_uses_computed_exports(
    #[case] body: &str,
) -> TestResult {
    // Given
    let source = format!("// 한글\r\n{body}");
    // When
    let found = observed(&source);
    // Then
    assert_eq!(
        found,
        vec![(
            "module.exports".to_string(),
            ObservedUse::Changes(offset(&source, "module['exports']")?, 2)
        )]
    );
    Ok(())
}

#[rstest]
#[case("external(module['exports'].tokens.space);", vec![Some("tokens".to_string()), Some("space".to_string())], None)]
#[case("external(module['exports'][key]);", vec![None], None)]
#[case("const alias=module['exports'].tokens;", vec![Some("tokens".to_string())], Some("alias".to_string()))]
#[case("external(module['exports']);", vec![], None)]
fn handoff_preserves_exact_path_and_holder_when_computed_carrier_is_read(
    #[case] body: &str,
    #[case] path: Vec<Option<String>>,
    #[case] into: Option<String>,
) -> TestResult {
    // Given
    let source = format!("// 한글\r\n{body}");
    // When
    let found = observed(&source);
    // Then
    assert_eq!(
        found,
        vec![(
            "module.exports".to_string(),
            ObservedUse::Escapes(offset(&source, "module['exports']")?, path, into)
        )]
    );
    Ok(())
}

#[test]
fn receiver_call_preserves_path_when_computed_carrier_method_may_mutate_this() -> TestResult {
    // Given
    let source = "// 한글\r\nmodule['exports'].tokens.touch();";
    // When
    let found = observed(source);
    // Then
    assert_eq!(
        found,
        vec![(
            "module.exports".to_string(),
            ObservedUse::Calls(
                offset(source, "module['exports']")?,
                vec![Some("tokens".to_string())]
            )
        )]
    );
    Ok(())
}

#[rstest]
#[case("const module={exports:{}};module['exports'].tokens=8;")]
#[case("function local(module){external(module['exports']);}")]
#[case("function local(exports){external(exports.tokens);}")]
#[case("const exports={};exports.tokens=8;")]
#[case("other['exports'].tokens=8;")]
#[case("module['notExports'].tokens=8;")]
#[case("module[key].tokens=8;")]
fn carrier_uses_are_absent_when_name_is_shadowed_or_member_is_unrelated(#[case] source: &str) {
    // Given
    let source = format!("// 한글\r\n{source}");
    // When
    let found = observed(&source);
    // Then
    assert_eq!(found, vec![]);
}

#[test]
fn only_global_carrier_is_classified_when_nested_bindings_share_its_name() -> TestResult {
    // Given
    let source = "function local(module,exports){external(module['exports']);external(exports);}\nexternal(module['exports'].tokens);";
    // When
    let found = observed(source);
    // Then
    assert_eq!(
        found,
        vec![(
            "module.exports".to_string(),
            ObservedUse::Escapes(
                offset(source, "module['exports'].tokens")?,
                vec![Some("tokens".to_string())],
                None
            )
        )]
    );
    Ok(())
}
