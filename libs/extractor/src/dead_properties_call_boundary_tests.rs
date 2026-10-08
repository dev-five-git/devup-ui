use crate::dead_properties_test_utils::{failure, position};
use crate::{ExtractOption, ImportAlias, dead_properties, extract};
use boa_engine::{Context, JsValue, Source};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;
use std::{error::Error, io};

fn evaluate_instrumented(code: &str) -> Result<JsValue, Box<dyn Error>> {
    let source = format!(
        "const __vanilla_extract__ = {{ __at: (location, api, thunk) => thunk() }};\n{code}"
    );
    Ok(Context::default()
        .eval(Source::from_bytes(&source))
        .map_err(|error| io::Error::other(error.to_string()))?)
}

#[rstest]
#[case(
    "import fallback from '@devup-ui/react';",
    "fallback({ boxOrient: 'vertical' })"
)]
#[case(
    "import { css } from '@devup-ui/react';",
    "css.call(null, { boxOrient: 'vertical' })"
)]
fn ordinary_import_calls_keep_their_behavior_when_not_styling_apis(
    #[case] import: &str,
    #[case] call: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: default imports and function members do not declare styling arguments.
    let source = format!("{import}\n{call};");

    // When: instrument actual parsed source, then execute its call with ordinary functions.
    let instrumented = dead_properties::instrument(&source, "ordinary.ts", "@devup-ui/react")
        .map_err(io::Error::other)?;
    let (_, body) = instrumented
        .code
        .split_once('\n')
        .ok_or_else(|| io::Error::other("fixture import must end with a newline"))?;
    let result = evaluate_instrumented(&format!(
        "const fallback = rules => rules.boxOrient === 'vertical' ? 7 : 0; const css = fallback;\n{body}"
    ))?;

    // Then: no styling-call metadata or declaration rejection, and the original result survives.
    assert_eq!(instrumented.calls, vec![]);
    assert_eq!(result.as_number(), Some(7.0));
    Ok(())
}

#[test]
fn direct_eval_keeps_lexical_scope_when_nested_calls_are_instrumented() -> Result<(), Box<dyn Error>>
{
    // Given: direct eval declares a variable in the calling function's scope.
    let source = "function inspect() { eval(String('var local = 42')); return local; } inspect();";

    // When: execute the real instrumented program, including the nested String call.
    let instrumented = dead_properties::instrument(source, "eval.ts", "@devup-ui/react")
        .map_err(io::Error::other)?;
    let result = evaluate_instrumented(&instrumented.code)?;

    // Then: the declaration belongs to inspect, not an instrumentation thunk.
    assert_eq!(result.as_number(), Some(42.0));
    Ok(())
}

#[test]
fn suspending_call_keeps_generator_resume_value() -> Result<(), Box<dyn Error>> {
    // Given: the outer echo argument yields, but its inner echo call is synchronous.
    let source = "const echo = value => value; function* values() { return echo(yield echo(7)); } const iter = values(); iter.next(); iter.next(13).value;";

    // When: instrument and execute valid generator source.
    let instrumented = dead_properties::instrument(source, "yield.ts", "@devup-ui/react")
        .map_err(io::Error::other)?;
    let result = evaluate_instrumented(&instrumented.code)?;

    // Then: yield remains in the generator body and the resumed argument reaches echo.
    assert_eq!(result.as_number(), Some(13.0));
    Ok(())
}

#[test]
#[serial]
fn evaluated_call_mapping_has_no_ranges_when_authored_declarations_are_rejected() {
    // Given: an aliased, valid source declaration rejected before evaluation.
    reset_class_map();
    reset_file_map();
    let source = "import { css } from '@emotion/react';\nconst good = css({ color: 'red' }); const c = css({ boxOrient: 'vertical' });";
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };

    // When: request evaluated origins for a source that cannot be evaluated.
    let calls = dead_properties::evaluated_calls(source, "rejected.tsx", &option);

    // Then: there are no partial ranges to misattribute the authored diagnostic.
    assert_eq!(calls, vec![]);
}

#[rstest]
#[case(r"const c = css('box-align\\:center; color:red');")]
#[case(r"const c = css('content:box-align\\;center; color:red');")]
#[case(r"const c = css('content:box-align\\}center; color:red');")]
#[case(r"const c = css('');")]
#[serial]
fn css_escaped_delimiters_are_not_dead_declarations(#[case] statement: &str) {
    // Given: escaped CSS punctuation is not a declaration boundary.
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ css }} from '@devup-ui/react';\n{statement}");

    // When: extract through the normal public source entrypoint.
    let result = extract("escapes.tsx", &source, ExtractOption::default());

    // Then: text that merely resembles a dead declaration is accepted.
    assert!(result.is_ok(), "{result:?}");
}

#[rstest]
#[case(r"const c = css('content:é😀; \x62ox-align:center');", r"\x62ox-align")]
#[case(
    r"const c = css('content:\u{1F600}é; \u0062ox-align:center');",
    r"\u0062ox-align"
)]
#[case(
    r"const c = css('content:\uD83D\uDE00é; \u{62}ox-align:center');",
    r"\u{62}ox-align"
)]
#[serial]
fn escaped_declaration_names_keep_authored_columns_after_unicode(
    #[case] statement: &str,
    #[case] key: &str,
) {
    // Given: decoded Unicode occupies different byte lengths from the authored escapes.
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ css }} from '@devup-ui/react';\n{statement}");
    let column = statement[..position(statement, key)].chars().count() + 1;

    // When: the escaped name decodes to a dead declaration.
    let error = failure(extract("unicode.tsx", &source, ExtractOption::default()));

    // Then: report the authored escape, not a byte position in the decoded string.
    assert!(
        error.starts_with(&format!("unicode.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("box-align"), "{error}");
}
