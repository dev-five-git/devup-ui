use boa_engine::{Context, JsValue, Source};
use rstest::rstest;
use serial_test::serial;

use super::{Stylesheet, execute_located, stringify};
use crate::ExtractOption;

#[test]
fn stringify_fails_with_a_type_error_when_user_code_replaces_json_stringify() -> Result<(), String>
{
    // Given
    let mut context = Context::default();
    context
        .eval(Source::from_bytes("JSON.stringify = 7"))
        .map_err(|error| error.to_string())?;
    // When
    let error = stringify(&JsValue::from(1), JsValue::undefined(), &mut context)
        .err()
        .ok_or("noncallable stringify accepted")?;
    // Then
    assert_eq!(
        error.to_string(),
        "TypeError: JSON.stringify is not callable (native)"
    );
    Ok(())
}

#[rstest]
#[case("Date.now()", "Date")]
#[case("Math.random()", "Math.random")]
#[serial]
fn exported_getters_cannot_hide_forbidden_reads_until_after_stylesheet_execution(
    #[case] read: &str,
    #[case] name: &str,
) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = format!(
        "import {{ createVar }} from '@devup-ui/react';\nexport const token = {{ get value() {{ return {read}; }} }};"
    );
    // When
    let error = execute_located(
        Stylesheet {
            filename: "getter.css.ts",
            code: &code,
            source: &code,
            edits: &[],
        },
        &ExtractOption::default(),
        None,
    )
    .err()
    .ok_or("forbidden export getter accepted")?;
    // Then
    let expected = crate::locate(
        "getter.css.ts",
        &code,
        code.find(read).ok_or("missing fixture read")?,
    );
    assert!(
        error.starts_with(&format!(
            "{expected}: JS execution error: ReferenceError: `{name}`"
        )),
        "{error}"
    );
    assert!(
        error.contains(&format!("at <read> ({expected})")),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
fn setup_errors_are_told_against_the_script_they_set_up(#[case] instrumented: bool) {
    // Given an error of the sandbox the build sets up
    let script = crate::module_loader::Script::default();
    let located = crate::evaluation_sandbox::instrument_located(
        "1",
        crate::module_loader::SCRIPT_PATH,
        &|_| true,
    );
    let error = boa_engine::JsNativeError::typ().with_message("helper is not configurable");
    // When it is reported
    let told = super::setup_error(&script, instrumented.then_some(&located), "a.css.ts")(&error);
    // Then the script names the file and the instrumentation locates its frames
    let text = error.to_string();
    let expected = if instrumented {
        script.explain(&located.explain(&text), "a.css.ts")
    } else {
        script.explain(&text, "a.css.ts")
    };
    assert_eq!(told, expected);
    assert!(told.contains("helper is not configurable"), "{told}");
}

#[test]
fn registration_errors_name_the_api_object() {
    assert_eq!(
        super::registration_error("not extensible"),
        "Failed to register __vanilla_extract__: not extensible"
    );
}
