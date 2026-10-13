use serial_test::serial;

use super::{error_of, parts};

#[rstest::rstest]
#[case("new Date(0)", "Date")]
#[case("Math.random()", "Math.random")]
#[case("globalThis['window']", "window")]
#[case(
    "(() => { try { Date.now(); } catch (error) { delete error.message; throw error; } })()",
    "Date"
)]
#[case(
    "(() => { try { Date.now(); } catch (error) { error.message = 'forged'; } return 'red'; })()",
    "Date"
)]
#[serial]
fn forbidden_reads_keep_the_actual_original_site_and_immutable_cause(
    #[case] expression: &str,
    #[case] name: &str,
) -> Result<(), String> {
    let source =
        format!("import {{ style }} from '@devup-ui/react';\nexport const card = {expression};");
    let token = match name {
        "Math.random" => "Math",
        "window" => "globalThis",
        _ => name,
    };
    let offset = source.find(token).ok_or("missing read fixture")?;
    let expected = crate::locate("/a.css.ts", &source, offset);

    let error = error_of("/a.css.ts", &source, &[]);

    assert!(
        error.starts_with(&format!(
            "{expected}: JS execution error: ReferenceError: `{name}` cannot be read at build time:"
        )),
        "{error}"
    );
    assert_eq!(error.matches("Fix:").count(), 1, "{error}");
    assert!(!error.contains("forged"), "{error}");
    for internal in [
        "devup-ui-stylesheet",
        "<devup-sandbox>",
        "__devup_read_site_",
    ] {
        assert!(!error.contains(internal), "{error}");
    }
    Ok(())
}

#[test]
#[serial]
fn a_retained_stylesheet_function_reports_its_own_forbidden_read() {
    const IMPORTED: &str = "import { style } from '@devup-ui/react';\nexport const s = style({});\nexport const read = (): string => {\n  return Date.now();\n};";

    let error = error_of(
        "/a.css.ts",
        "import { read } from './b.css';\nexport const result = read();",
        &[("/b.css.ts", IMPORTED)],
    );

    assert!(
        error.starts_with("/b.css.ts:4:10: JS execution error: ReferenceError: `Date`"),
        "{error}"
    );
    assert!(!error.contains("devup-ui-stylesheet"), "{error}");
}

#[rstest::rstest]
#[case("[...null]")]
#[case("[...[...null]]")]
#[case("[...[], ...null]")]
#[serial]
fn an_iterable_operation_without_an_engine_frame_is_located_exactly(#[case] expression: &str) {
    let source = format!(
        "import {{ style }} from '@devup-ui/react';\nconst unused = {expression};\nexport const card = style({{ color: 'red' }});"
    );

    let error = error_of("/a.css.ts", &source, &[]);

    let (lead, _, _) = parts(&error);
    let column = match expression {
        "[...[...null]]" => 21,
        "[...[], ...null]" => 24,
        _ => 17,
    };
    assert_eq!(
        lead,
        format!(
            "/a.css.ts:2:{column}: JS execution error: TypeError: cannot convert 'null' or 'undefined' to object"
        ),
        "{error}"
    );
    assert!(!error.contains("devup-ui-stylesheet"), "{error}");
}

#[test]
#[serial]
fn iterable_markers_preserve_receiver_lexical_values_and_evaluation_order() -> Result<(), String> {
    let source = "const events = [];\nconst iterable = { value: 'red', *[Symbol.iterator]() { events.push(this.value); yield this.value; } };\nexport const values = (() => { const local = 'blue'; return [events.push('before'), ...iterable, local, events.push('after')]; })();\nexport const order = events;";

    let (collected, _) = crate::vanilla_extract::execute_stylesheet(
        source,
        "/a.css.ts",
        &crate::ExtractOption::default(),
        None,
    )?;

    assert_eq!(
        collected.constant_exports,
        [
            (
                "values".to_string(),
                "[1, \"red\", \"blue\", 3]".to_string()
            ),
            (
                "order".to_string(),
                "[\"before\", \"red\", \"after\"]".to_string()
            )
        ]
    );
    Ok(())
}
