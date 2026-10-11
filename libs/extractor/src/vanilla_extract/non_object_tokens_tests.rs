use crate::ExtractOption;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("null")]
#[case("undefined")]
#[case("'tokens'")]
#[case("7")]
#[case("false")]
#[serial]
fn themes_reject_when_token_inputs_are_not_objects(
    #[case] tokens: &str,
    #[values("createTheme", "createGlobalTheme")] api: &str,
) -> Result<(), String> {
    let arguments = if api == "createGlobalTheme" {
        format!("':root', {tokens}")
    } else {
        tokens.to_string()
    };
    let code = format!(
        "import {{ {api} }} from '@devup-ui/react';\nexport const result = {api}({arguments});"
    );
    let error =
        super::execute_stylesheet(&code, "non-object.css.ts", &ExtractOption::default(), None)
            .err()
            .ok_or("non-object theme tokens were accepted")?;
    assert!(error.starts_with("non-object.css.ts:2:"), "{error}");
    assert!(
        error.contains("TypeError: Theme tokens must be an object"),
        "{error}"
    );
    assert!(error.contains("Fix: supply a token object"), "{error}");
    Ok(())
}
