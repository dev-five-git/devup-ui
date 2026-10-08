use crate::{ExtractOption, ImportAlias, emotion_namespace, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

fn aliases() -> HashMap<String, ImportAlias> {
    HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)])
}

fn normalized(code: &str) -> Result<String, String> {
    emotion_namespace::normalize(code, "namespace.tsx", &aliases())
        .map(|(code, _)| code.into_owned())
        .map_err(|errors| format!("{errors:?}"))
}

#[test]
fn alias_and_type_reads_compile_when_only_value_calls_need_lowering() -> Result<(), String> {
    // Given
    let source = r"import * as E from '@emotion/css'; let uninitialized; type Api=typeof E; const key='css', N=E, c=N[key]; type Call=typeof c; c({ padding: 8 });";

    // When
    let output = normalized(source)?;

    // Then
    assert!(output.contains("type Api=typeof E"), "{output}");
    assert!(output.contains("type Call=typeof c"), "{output}");
    assert!(output.contains("N={}"), "{output}");
    assert!(output.contains("c=void 0"), "{output}");
    assert!(!output.contains("N[key]"), "{output}");
    Ok(())
}

#[rstest]
#[case(r"(E).css({ padding: 8 });")]
#[case(r"(E.css as typeof E.css)({ padding: 8 });")]
#[case(r"(E.css satisfies typeof E.css)({ padding: 8 });")]
#[case(r"E.css!({ padding: 8 });")]
fn direct_calls_compile_when_wrapped_in_syntax_only_expressions(
    #[case] body: &str,
) -> Result<(), String> {
    // Given
    let source = format!("import * as E from '@emotion/css'; {body}");

    // When
    let output = normalized(&source)?;

    // Then
    assert!(output.contains("({ padding: 8 })"), "{output}");
    assert!(!output.contains("E.css"), "{output}");
    assert!(output.contains("css as __emotion_css_"), "{output}");
    Ok(())
}

#[test]
fn generated_import_avoids_collision_when_initial_name_is_taken() -> Result<(), String> {
    // Given: E, __emotion_css_3 and a are the three distinct symbols; no unresolved names.
    let source = r"import * as E from '@emotion/css'; export const __emotion_css_3=7; export const a=E.css({ padding: 8 });";

    // When
    let output = normalized(source)?;

    // Then
    assert!(output.contains("css as __emotion_css_4"), "{output}");
    assert!(
        output.contains(r"a=__emotion_css_4({ padding: 8 })"),
        "{output}"
    );
    assert!(
        output.contains("export const __emotion_css_3=7"),
        "{output}"
    );
    Ok(())
}

#[test]
fn static_string_properties_remain_when_not_namespace_reads() -> Result<(), String> {
    // Given
    let source = r"import * as E from '@emotion/css'; const key='css'; const length=key.length; E[key]({ padding: 8 });";

    // When
    let output = normalized(source)?;

    // Then
    assert!(output.contains("const length=key.length"), "{output}");
    assert!(!output.contains("E[key]"), "{output}");
    Ok(())
}

#[rstest]
#[case("E[E]({});", "E[E]")]
#[case("const c=E.css; E[c]({});", "E[c]")]
#[case("const c=E.css; const property=c.length;", "c")]
#[case("E[key]({});", "E[key]")]
#[case("const c=E.css; c=other;", "c")]
#[case("const N=E; N={};", "N")]
#[case("const key='css'; key='cx'; E[key]({});", "E[key]")]
#[case("consume(E);", "E")]
#[case("const [css]=E;", "E")]
#[serial]
fn unsafe_reads_report_locations_when_keys_or_bindings_escape(
    #[case] body: &str,
    #[case] offending: &str,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("import * as E from '@emotion/css';\n{body}");

    // When
    let error = match extract(
        "namespace.tsx",
        &source,
        ExtractOption {
            import_aliases: aliases(),
            ..ExtractOption::default()
        },
    ) {
        Ok(output) => panic!("unexpected success: {}", output.code),
        Err(error) => error.to_string(),
    };

    // Then
    assert!(error.starts_with("namespace.tsx:2:"), "{error}");
    assert!(
        error.contains(&format!("cannot use `{offending}`")),
        "{error}"
    );
    assert!(!error.contains("__emotion_"), "{error}");
}

#[test]
#[serial]
fn original_error_preserves_user_identifiers_when_bracket_macro_is_restored() {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import * as E from '@emotion/css';\nconst __emotion_user=external; export const a=E['cx']`a ${__emotion_user}`;";

    // When
    let error = match extract(
        "namespace.tsx",
        source,
        ExtractOption {
            import_aliases: aliases(),
            ..ExtractOption::default()
        },
    ) {
        Ok(output) => panic!("unexpected success: {}", output.code),
        Err(error) => error.to_string(),
    };

    // Then
    assert!(error.contains("namespace.tsx:2:47:"), "{error}");
    assert!(error.contains("E['cx']`a ${__emotion_user}`"), "{error}");
    assert!(!error.contains("__emotion_cx_"), "{error}");
}
