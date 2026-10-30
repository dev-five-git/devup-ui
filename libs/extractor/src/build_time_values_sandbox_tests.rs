use rstest::rstest;

fn evaluate(code: &str) -> Result<Option<String>, Vec<(usize, String)>> {
    super::evaluate_located(
        code,
        "a.tsx",
        &crate::ExtractOption::default(),
        None,
        &crate::imported_constants::Unknown::default(),
    )
    .map(|evaluated| evaluated.map(|(code, ..)| code))
}

const COMPARE: &str = "import { css } from '@devup-ui/react';\nconst compare = Object.getOwnPropertyDescriptor(String.prototype, 'localeCompare').value;\n";

#[test]
fn a_later_value_reports_its_own_read_offset() -> Result<(), String> {
    // Given two separate computed values, with the forbidden read in the second.
    let code = "import { css } from '@devup-ui/react';\nexport const a = css({ w: Number('2'), h: Object.getOwnPropertyDescriptor(String.prototype, 'localeCompare').value('a', 'b') });";
    // When evaluation records the read.
    let errors = evaluate(code).err().ok_or("evaluation succeeded")?;
    // Then its location names the second value, not the first generated script.
    assert_eq!(
        errors[0].0,
        code.find("Object.getOwnPropertyDescriptor")
            .ok_or("missing read")?
    );
    Ok(())
}

#[rstest]
#[case("export const a = css({ w: compare('a', 'b') });")]
#[case(
    "const sorted = ['b', 'a'].map((item) => compare(item, 'a'));\nexport const a = css({ w: sorted[0] });"
)]
fn forbidden_helper_references_report_the_helper_not_the_style_value(
    #[case] tail: &str,
) -> Result<(), String> {
    let code = format!("{COMPARE}{tail}");
    let errors = evaluate(&code).err().ok_or("evaluation succeeded")?;
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].0,
        code.find("Object.getOwnPropertyDescriptor")
            .ok_or("fixture occurrence missing")?
    );
    assert!(errors[0].1.contains("String.prototype.localeCompare"));
    assert!(!errors[0].1.contains("Fix:"));
    assert!(
        errors[0]
            .1
            .contains("cannot use `Object.getOwnPropertyDescriptor")
    );
    Ok(())
}

#[test]
fn deterministic_code_still_computes_its_values() -> Result<(), String> {
    let code = "import { css } from '@devup-ui/react';\nconst double = (n) => Math.max(n, 1) * 2;\nexport const a = css({ w: double(2) });";
    let result = evaluate(code)
        .map_err(|errors| format!("{errors:?}"))?
        .ok_or("no computed value")?;
    assert!(result.contains("w: 4"));
    Ok(())
}

#[rstest]
#[case(
    "import { css } from '@devup-ui/react';\nconst fail = () => { throw 1; };\nexport const a = css({ w: fail() });"
)]
#[case("import { css } from '@devup-ui/react';\nconst NaN = 1;\nexport const a = css({ w: NaN });")]
#[case(
    "import { css } from '@devup-ui/react';\ndeclare const a: number, b: number;\nexport const x = css({ w: a + b });"
)]
#[case(
    "import { Box } from '@devup-ui/react';\ndeclare function getHover(): object;\nexport const x = <Box _hover={getHover()} />;"
)]
fn a_definition_that_fails_for_another_reason_leaves_runtime_values(
    #[case] code: &str,
) -> Result<(), String> {
    assert!(
        evaluate(code)
            .map_err(|errors| format!("{errors:?}"))?
            .is_none()
    );
    Ok(())
}
