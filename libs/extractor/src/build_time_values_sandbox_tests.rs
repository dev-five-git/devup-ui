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

#[test]
fn objects_spread_into_unknown_code_keep_their_reads_at_runtime() -> Result<(), String> {
    // Given objects whose every element is handed on, under any key, to a call
    // the build does not know.
    let code = "import { css } from '@devup-ui/react';\nconst sizes = ['b', 'a'].map((item) => ({ w: item.length }));\nmutate(...sizes);\nexport const a = css({ w: sizes[0].w });";
    // When the build evaluates the style value, then that code may have
    // changed the objects, so it computes nothing.
    assert!(
        evaluate(code)
            .map_err(|errors| format!("{errors:?}"))?
            .is_none()
    );
    Ok(())
}

#[test]
fn numbers_spread_into_unknown_code_still_compute() -> Result<(), String> {
    // Given only numbers handed on, under any key, to a call the build does not know.
    let code = "import { css } from '@devup-ui/react';\nconst sizes = ['b', 'a'].map((item) => item.length * 2);\nmutate(...sizes);\nexport const a = css({ w: sizes[0] });";
    // When the build evaluates the style value, then no object can change.
    let result = evaluate(code)
        .map_err(|errors| format!("{errors:?}"))?
        .ok_or("no computed value")?;
    assert!(result.contains("w: 2"), "{result}");
    Ok(())
}

#[rstest]
#[case(
    "const box = { w: 2 };\nconst read = () => { delete box.w; return 1; };",
    None
)]
#[case(
    "const read = () => { const box = { w: 2 }; delete box.w; return 3; };",
    Some("w: 3")
)]
fn delete_changes_only_what_the_value_owns(
    #[case] definitions: &str,
    #[case] expected: Option<&str>,
) -> Result<(), String> {
    // Given a value whose function deletes a key of an outside or its own object.
    let code = format!(
        "import {{ css }} from '@devup-ui/react';\n{definitions}\nexport const a = css({{ w: read() }});"
    );
    // When the build evaluates it, then only deleting its own key leaves it exact.
    let result = evaluate(&code).map_err(|errors| format!("{errors:?}"))?;
    match expected {
        Some(expected) => assert!(
            result
                .as_deref()
                .is_some_and(|code| code.contains(expected)),
            "{result:?}"
        ),
        None => assert!(result.is_none(), "{result:?}"),
    }
    Ok(())
}
