use super::*;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_sourcemap::{SourceMap, TokenChunk};
use rstest::rstest;
use std::borrow::Cow;

#[rstest]
#[case("function f(){return <div value={(first(),last())}/>;}", 2)]
#[case("function f(){return <div>{(first(),last())}</div>;}", 2)]
#[case(
    "function f(){return <div value={(first(),<span value={(inner(),2)}>{(child(),3)}</span>)}/>;}",
    6
)]
#[case("function f(){return <>{(first(),last())}</>;}", 2)]
#[case("function f(){return <><>{(first(),last())}</></>;}", 2)]
#[case("async function f(p){return <div>{(first(),await p)}</div>;}", 2)]
#[case("function* f(){return <div>{(first(),yield 1)}</div>;}", 2)]
fn grouping_changes_only_the_missing_parentheses(#[case] source: &str, #[case] added: usize) {
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let mut result = Codegen::new().build(&parsed.program);
    let before = result.code.clone();
    parenthesize(&mut result, SourceType::tsx()).unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(result.code.len(), before.len() + added);
    assert!(
        Parser::new(&arena, &result.code, SourceType::tsx())
            .parse()
            .diagnostics
            .is_empty()
    );
    // Parsing the result again and printing it produces the same ungrouped
    // Oxc output. Thus grouping adds no other AST operation, binding or scope.
    let grouped = Parser::new(&arena, &result.code, SourceType::tsx()).parse();
    assert_eq!(Codegen::new().build(&grouped.program).code, before);
}

#[test]
fn valid_output_is_byte_and_map_identical() {
    let source = "const value=(first(),last());";
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
    let mut result = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some("input.tsx".into()),
            ..CodegenOptions::default()
        })
        .build(&parsed.program);
    let before = result.code.clone();
    let map = result
        .map
        .as_ref()
        .unwrap_or_else(|| panic!("expected generated source map"))
        .to_json_string();
    parenthesize(&mut result, SourceType::tsx()).unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(result.code, before);
    assert_eq!(
        result
            .map
            .as_ref()
            .unwrap_or_else(|| panic!("expected generated source map"))
            .to_json_string(),
        map
    );
}

#[test]
fn other_generated_errors_are_not_recovered() {
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, "const ok=1;", SourceType::tsx()).parse();
    let mut result = Codegen::new().build(&parsed.program);
    result.code = "const = ;".into();
    let error = match parenthesize(&mut result, SourceType::tsx()) {
        Err(error) => error.to_string(),
        Ok(()) => panic!("invalid generated JSX must remain an error"),
    };
    assert!(error.starts_with("Cannot emit JSX:"), "{error}");
    assert_eq!(result.code, "const = ;");
    assert!(validate("<div>{first(),last()}</div>", SourceType::tsx()).is_err());
}

#[test]
fn generated_map_column_overflow_is_reported() {
    let parts = oxc_sourcemap::SourceMapParts {
        tokens: vec![oxc_sourcemap::Token::new(0, u32::MAX, 0, 0, None, None)].into_boxed_slice(),
        ..oxc_sourcemap::SourceMapParts::default()
    };
    let error = match crate::source_map::shift_generated(SourceMap::from_parts(parts), "a", &[0]) {
        Err(error) => error.to_string(),
        Ok(_) => panic!("generated source map column overflow must remain an error"),
    };
    assert_eq!(error, "Generated source map column overflow");
}

#[test]
#[serial_test::serial]
fn original_invalid_jsx_remains_a_located_source_error() {
    let error = match crate::extract(
        "original.tsx",
        "import {Box} from '@devup-ui/react';const value=<Box data-value={first(),last()}/>;",
        crate::ExtractOption::default(),
    ) {
        Err(error) => error.to_string(),
        Ok(_) => panic!("original invalid JSX must remain a located error"),
    };
    assert!(error.starts_with("original.tsx:1:"), "{error}");
    assert!(error.contains("comma operator"), "{error}");
}

#[test]
#[serial_test::serial]
fn compiled_elements_use_the_rare_emission_path() {
    let source = "import {Box} from '@devup-ui/react';function f(){return <Box data-value={(first(),last())}>{(before(),child())}</Box>;}";
    let output = crate::extract("compiled.tsx", source, crate::ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error:?}"));
    let arena = Allocator::default();
    assert!(
        Parser::new(&arena, &output.code, SourceType::tsx())
            .parse()
            .diagnostics
            .is_empty(),
        "{}",
        output.code
    );
    assert!(
        output.code.contains("(before(), child())"),
        "{}",
        output.code
    );
}

#[test]
fn unicode_nested_jsx_keeps_source_map_metadata_and_token_origins() {
    let source = "const label='🦀';\r\nfunction f(){return <div value={(first(),<span value={(inner(),label)}>é🦀\r\n\u{2028}\u{2029}{(child(),label)}</span>)}/>;}";
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let mut result = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some("input.tsx".into()),
            ..CodegenOptions::default()
        })
        .build(&parsed.program);
    let old_code = result.code.clone();
    assert!(
        old_code.contains("\r\n") && old_code.contains('\u{2028}') && old_code.contains('\u{2029}')
    );
    let mut before = result
        .map
        .take()
        .unwrap_or_else(|| panic!("expected generated source map"))
        .into_parts();
    before.file = Some(Cow::Borrowed("output.js"));
    before.source_root = Some(Cow::Borrowed("/root"));
    before.ignore_list = Some(vec![0]);
    before.debug_id = Some(Cow::Borrowed("emission-test"));
    let middle = before.tokens.len() / 2;
    let previous = before.tokens[middle - 1];
    let previous_name = before.tokens[..middle]
        .iter()
        .rev()
        .find_map(oxc_sourcemap::Token::get_name_id)
        .unwrap_or(0);
    before.token_chunks = Some(vec![
        TokenChunk::new(
            0,
            u32::try_from(middle).unwrap_or_else(|error| panic!("{error:?}")),
            0,
            0,
            0,
            0,
            0,
            0,
        ),
        TokenChunk::new(
            u32::try_from(middle).unwrap_or_else(|error| panic!("{error:?}")),
            u32::try_from(before.tokens.len()).unwrap_or_else(|error| panic!("{error:?}")),
            previous.get_dst_line(),
            previous.get_dst_col(),
            previous.get_src_line(),
            previous.get_src_col(),
            previous_name,
            previous.get_source_id().unwrap_or(0),
        ),
    ]);
    result.map = Some(SourceMap::from_parts(before.clone()));
    parenthesize(&mut result, SourceType::tsx()).unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(result.code.len(), old_code.len() + 6);
    let after = result
        .map
        .as_ref()
        .unwrap_or_else(|| panic!("expected generated source map"))
        .clone()
        .into_parts();
    assert_eq!(after.file, before.file);
    assert_eq!(after.names, before.names);
    assert_eq!(after.source_root, before.source_root);
    assert_eq!(after.sources, before.sources);
    assert_eq!(after.source_contents, before.source_contents);
    assert_eq!(after.ignore_list, before.ignore_list);
    assert_eq!(after.debug_id, before.debug_id);
    assert_eq!(after.tokens.len(), before.tokens.len());
    let old_lines = crate::source_map::Lines::new(&old_code);
    let new_lines = crate::source_map::Lines::new(&result.code);
    for (old, new) in before.tokens.iter().zip(after.tokens.iter()) {
        assert_eq!(
            (
                old.get_src_line(),
                old.get_src_col(),
                old.get_source_id(),
                old.get_name_id()
            ),
            (
                new.get_src_line(),
                new.get_src_col(),
                new.get_source_id(),
                new.get_name_id()
            )
        );
        assert_eq!(old.get_dst_line(), new.get_dst_line());
        let old_offset = old_lines.offset(old.get_dst_line(), old.get_dst_col());
        let new_offset = new_lines.offset(new.get_dst_line(), new.get_dst_col());
        assert_eq!(
            old_code[old_offset..].chars().next(),
            result.code[new_offset..].chars().next()
        );
    }
    let mut expected_chunks = before
        .token_chunks
        .unwrap_or_else(|| panic!("expected original token chunks"));
    for chunk in &mut expected_chunks {
        if let Some((_, new)) = before
            .tokens
            .iter()
            .zip(after.tokens.iter())
            .find(|(old, _)| {
                old.get_dst_line() == chunk.prev_dst_line && old.get_dst_col() == chunk.prev_dst_col
            })
        {
            chunk.prev_dst_col = new.get_dst_col();
        }
    }
    assert_eq!(
        after
            .token_chunks
            .as_ref()
            .unwrap_or_else(|| panic!("expected emitted token chunks")),
        &expected_chunks
    );
    let encoded = result
        .map
        .as_ref()
        .unwrap_or_else(|| panic!("expected generated source map"))
        .to_json_string();
    let decoded = SourceMap::from_json_string(&encoded).unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(
        decoded.get_tokens().collect::<Vec<_>>(),
        after.tokens.to_vec()
    );
}

#[rstest]
#[case("<>{(first(),last())}</>")]
#[case("<><>{(first(),last())}</></>")]
#[serial_test::serial]
fn compiled_siblings_detect_fragment_sequences(#[case] fragment: &str) {
    let source = format!(
        "import {{Box}} from '@devup-ui/react';function f(){{return [<Box color='red'/>,{fragment}];}}"
    );
    let output = crate::extract("fragment.tsx", &source, crate::ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error:?}"));
    let arena = Allocator::default();
    assert!(
        Parser::new(&arena, &output.code, SourceType::tsx())
            .parse()
            .diagnostics
            .is_empty(),
        "{}",
        output.code
    );
    assert!(output.code.contains("(first(), last())"), "{}", output.code);
}

#[rstest]
#[case("input.cjs", "return <div>{(first(),last())}</div>;", false)]
#[case("input.js", "const value=<div>{(first(),last())}</div>;", false)]
#[case(
    "input.mjs",
    "export const value=<div>{(first(),last())}</div>;",
    false
)]
#[case(
    "input.cts",
    "const count:number=1;const value=<div>{(first(),count)}</div>;",
    true
)]
#[case(
    "input.cjs",
    "import x from 'x';console.log(new.target);return <div>{(first(),last())}</div>;export {x};",
    false
)]
fn emission_preserves_physical_goal_and_generated_jsx(
    #[case] filename: &str,
    #[case] source: &str,
    #[case] typescript: bool,
) {
    let source_type = crate::parsing::source_type(filename)
        .unwrap_or_else(|error| panic!("{error:?}"))
        .with_jsx(true);
    assert_eq!(source_type.is_typescript(), typescript);
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, source, source_type).parse();
    assert!(
        !goal_policy::has_diagnostics(&parsed),
        "{:?}",
        parsed.diagnostics
    );
    let mut result = Codegen::new().build(&parsed.program);
    let before = result.code.clone();
    parenthesize(&mut result, source_type).unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(result.code.len(), before.len() + 2);
    validate(&result.code, source_type).unwrap_or_else(|error| panic!("{error:?}"));
    let grouped = Parser::new(&arena, &result.code, source_type).parse();
    assert_eq!(
        grouped.program.source_type.is_commonjs(),
        source_type.is_commonjs()
    );
    assert_eq!(Codegen::new().build(&grouped.program).code, before);
}

#[rstest]
#[case("input.js", "const value:number=1;")]
#[case("input.mjs", "return <div>{(first(),last())}</div>;")]
#[case("input.cjs", "const value=import.meta;")]
#[case("input.cjs", "await pending;")]
fn unrelated_dialect_errors_remain_errors(#[case] filename: &str, #[case] source: &str) {
    let source_type = crate::parsing::source_type(filename)
        .unwrap_or_else(|error| panic!("{error:?}"))
        .with_jsx(true);
    assert!(validate(source, source_type).is_err());
}

#[test]
fn recoverable_non_sequence_emission_error_preserves_code_and_map() {
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, "const original=1;", SourceType::tsx()).parse();
    let mut result = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some("original.tsx".into()),
            ..CodegenOptions::default()
        })
        .build(&parsed.program);
    result.code = "const invalid=/[z-a]/;".into();
    let checked = Parser::new(&arena, &result.code, SourceType::tsx())
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();
    assert!(
        !checked.fatal_error,
        "fixture must exercise a recoverable diagnostic"
    );
    assert_eq!(checked.diagnostics.len(), 1);
    assert_eq!(
        checked.diagnostics[0].to_string(),
        "Invalid regular expression: Character class atom range out of order"
    );
    assert_ne!(checked.diagnostics[0].code.number.as_deref(), Some("18007"));
    let before = result.code.clone();
    let before_map = result
        .map
        .as_ref()
        .map(oxc_sourcemap::SourceMap::to_json_string);
    let Err(error) = parenthesize(&mut result, SourceType::tsx()) else {
        panic!("only missing JSX sequence parentheses can be repaired")
    };
    assert_eq!(
        error.to_string(),
        "Cannot emit JSX: Invalid regular expression: Character class atom range out of order"
    );
    assert_eq!(result.code, before);
    assert_eq!(
        result
            .map
            .as_ref()
            .map(oxc_sourcemap::SourceMap::to_json_string),
        before_map
    );
}
