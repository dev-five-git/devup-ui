use super::parse;
use crate::{ExtractOption, ImportAlias, ResolvedModule, extract, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

#[rstest]
#[case::new_target("console.log(new.target);")]
#[case::top_level_return("return 1;")]
#[case::legacy_comment("<!-- legacy\nconsole.log(1);")]
#[serial]
fn public_extract_preserves_physical_commonjs_grammar(
    #[case] statement: &str,
    #[values(true, false)] single_css: bool,
) {
    // Given: the same physical-CJS statements accepted by the hybrid goal policy.
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\n{statement}\nexport const style = css({{ color: 'teal' }});"
    );
    // When: the public extraction path parses and compiles the hybrid module.
    let output = extract(
        "input.cjs",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then: CommonJS grammar does not prevent actual style extraction.
    assert!(output.styles.iter().any(|style| matches!(style, crate::extract_style::extract_style_value::ExtractStyleValue::Static(style) if style.property == "color" && style.value == "teal")));
}

#[rstest]
#[case::nested_import("function f() { import 'nested'; }", "input.cjs:2:16:")]
#[case::nested_export("function f() { export const nested = 2; }", "input.cjs:2:16:")]
#[case::import_meta("console.log(import.meta.url);", "input.cjs:2:")]
#[case::top_level_await("await Promise.resolve(1);", "input.cjs:2:")]
#[serial]
fn public_extract_rejects_real_commonjs_errors(#[case] statement: &str, #[case] location: &str) {
    // Given: invalid CJS syntax beside a permitted top-level ES declaration.
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\n{statement}\nexport const style = css({{ color: 'teal' }});"
    );
    // When: public extraction, rather than the test-only parser, validates input.
    let Err(error) = extract("input.cjs", &source, ExtractOption::default()) else {
        panic!("expected syntax error");
    };
    let error = error.to_string();
    // Then: the real error retains its physical location and #757 guidance.
    assert!(error.starts_with(location), "{error}");
    assert!(error.contains("Cannot parse source:"), "{error}");
    assert!(
        error.contains("Fix: correct the syntax at this location"),
        "{error}"
    );
}

#[rstest]
#[case("console.log(new.target);")]
#[case("return 1;")]
#[case("<!-- legacy\nconsole.log(1);")]
fn commonjs_grammar_survives_when_es_import_is_present(
    #[case] statement: &str,
    #[values("", "import 'dep';\n")] prefix: &str,
) {
    // Given: pure or hybrid CommonJS, not Module grammar.
    let arena = Allocator::default();
    let source = format!("{prefix}{statement}");
    // When
    let program = parse(&arena, "input.cjs", &source).unwrap_or_else(|error| panic!("{error}"));
    // Then: physical metadata and the CJS statement survive.
    assert!(program.source_type.is_commonjs());
    assert_eq!(program.body.len(), if prefix.is_empty() { 1 } else { 2 });
}

#[rstest]
#[case("import 'dep';")]
#[case("export const value = 1;")]
#[case("const value = 1; export { value };")]
#[case("export { value } from 'dep';")]
#[case("export default 1;")]
#[case("export * from 'dep';")]
#[case("export * as values from 'dep';")]
#[case("@decorate export class Value {}")]
#[case("export @decorate class Value {}")]
fn top_level_declarations_survive_when_filename_is_commonjs(#[case] source: &str) {
    // Given
    let arena = Allocator::default();
    // When
    let program = parse(&arena, "input.cjs", source).unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert!(program.source_type.is_commonjs());
    assert_ne!(program.body.len(), 0);
}

#[rstest]
#[case("import 'dep';\nfunction f() { import 'nested'; }", "input.cjs:2:16:")]
#[case(
    "export const value = 1;\nfunction f() { export const nested = 2; }",
    "input.cjs:2:16:"
)]
#[case("import 'dep';\nconsole.log(import.meta.url);", "input.cjs:2:")]
#[case("import 'dep';\nawait Promise.resolve(1);", "input.cjs:2:")]
#[case("import 'dep';\nconst view = <Box wrong={} />;", "input.cjs:2:")]
fn real_errors_remain_located_when_top_level_declarations_are_present(
    #[case] source: &str,
    #[case] location: &str,
) {
    // Given
    let arena = Allocator::default();
    // When
    let Err(error) = parse(&arena, "input.cjs", source) else {
        panic!("expected syntax error");
    };
    let error = error.to_string();
    // Then: the valid first-line declaration does not hide or prepend another error.
    assert!(error.starts_with(location), "{error}");
}

#[rstest]
#[case("const value: number = 1;")]
#[case("import { from 'broken';")]
#[case("const view = <Box")]
#[case("throw;")]
fn fatal_errors_keep_all_diagnostics_when_goal_mismatch_precedes_them(#[case] invalid: &str) {
    // Given
    let arena = Allocator::default();
    let source = format!("import 'dep';\n{invalid}");
    // When
    let Err(error) = parse(&arena, "input.cjs", &source) else {
        panic!("expected fatal syntax error");
    };
    let error = error.to_string();
    // Then: a fatal parse is never classified, including its earlier goal diagnostic.
    assert!(
        error.starts_with("input.cjs:1:1: Cannot use import statement outside a module"),
        "{error}"
    );
    assert!(
        error.lines().any(|line| line.starts_with("input.cjs:2:")),
        "{error}"
    );
}

#[rstest]
#[case(
    "input.mjs",
    "import 'dep'; console.log(import.meta.url); await Promise.resolve(1);"
)]
#[case("input.cts", "import 'dep'; export const value: number = 1;")]
#[case("input.jsx", "import 'dep'; export const view = <div />;")]
fn original_dialects_remain_accepted_when_they_are_not_javascript_commonjs(
    #[case] filename: &str,
    #[case] source: &str,
) {
    // Given
    let arena = Allocator::default();
    let physical = SourceType::from_path(filename).unwrap_or_else(|error| panic!("{error}"));
    // When
    let program = parse(&arena, filename, source).unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(
        program.source_type.is_typescript(),
        physical.is_typescript()
    );
    assert_eq!(program.source_type.is_commonjs(), physical.is_commonjs());
    assert_eq!(program.source_type.extension(), physical.extension());
    assert_eq!(program.source_type.is_jsx(), physical.is_javascript());
}

#[rstest]
#[serial]
fn hybrid_dependency_is_read_when_commonjs_contains_es_exports(
    #[values(true, false)] single_css: bool,
) {
    let _state = crate::test_state::TestStateGuard::new();
    // Given
    reset_class_map();
    reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./tokens").then(|| ResolvedModule {
            path: "tokens.cjs".into(),
            code: "export const color = 'teal'; console.log(new.target);".into(),
        })
    };
    // When
    let output = extract_with_modules("input.cjs", "import { css } from '@devup-ui/react'; import { color } from './tokens'; export const style = css({ color });", ExtractOption { single_css, ..ExtractOption::default() }, false, &resolver).unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(output.dependencies, ["tokens.cjs"]);
    assert!(output.styles.iter().any(|style| matches!(style, crate::extract_style::extract_style_value::ExtractStyleValue::Static(style) if style.property == "color" && style.value == "teal")));
}

#[rstest]
#[serial]
fn emotion_error_restores_written_callee_when_source_is_hybrid_commonjs(
    #[values(true, false)] single_css: bool,
) {
    let _state = crate::test_state::TestStateGuard::new();
    // Given
    let source = "import * as E from '@emotion/css';\nexport const result=E.cx`a b`;";
    // When
    let Err(error) = extract(
        "input.cjs",
        source,
        ExtractOption {
            single_css,
            import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    ) else {
        panic!("expected Emotion diagnostic");
    };
    let error = error.to_string();
    // Then
    assert_eq!(
        error,
        "input.cjs:2:21: `cx()` cannot use `E.cx`a b`` at build time: call it with class names, as in `cx('a', 'b')`"
    );
}
