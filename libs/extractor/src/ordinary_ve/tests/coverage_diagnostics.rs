use rstest::rstest;
use serial_test::serial;

use super::demand_support::{reset, run};

#[rstest]
#[case("window['color']", "ts")]
#[case("(window)", "ts")]
#[case("(window as string)", "ts")]
#[case("(window satisfies string)", "ts")]
#[case("window!", "ts")]
#[case("<string>window", "ts")]
#[case("window?.color", "ts")]
#[serial]
fn forbidden_input_diagnostic_walks_its_original_computed_member_and_syntax_wrappers(
    #[case] expression: &str,
    #[case] extension: &str,
) {
    reset();
    let code = format!(
        "import {{style}} from '@vanilla-extract/css';\nconst box=style({{color:{expression}}});throw new Error('host only');"
    );
    let filename = format!("/diagnostic.{extension}");
    let offset = code
        .find("window")
        .unwrap_or_else(|| panic!("fixture window input is missing"));
    let place = crate::locate(&filename, &code, offset);

    let result = run(&filename, &code, &[]);

    let error = match result {
        Ok(output) => panic!("forbidden native input was accepted: {}", output.code),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&format!("{place}: native styling cannot use `{expression}` at build time: this required expression needs an exact, static input. Fix: provide an exact static value or CSS variable for this expression")), "{error}");
}
