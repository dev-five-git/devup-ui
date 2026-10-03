use super::*;
use rstest::rstest;

#[test]
#[serial]
fn emotion_restoration_reports_malformed_source_when_normalization_rejects_a_read() {
    // Given: normalization can reject a namespace read before the main parser runs.
    let source = "import * as E from '@emotion/css';\nE.__emotion_user(); const broken;";

    // When
    let error = compile(source).unwrap_err();

    // Then
    assert!(error.starts_with("namespace.tsx:2:1:"), "{error}");
    assert!(error.contains("E.__emotion_user"), "{error}");
    assert!(error.contains("Emotion diagnostic restoration failed:"), "{error}");
}

#[test]
#[serial]
fn emotion_restoration_preserves_generic_runtime_read_errors_when_user_names_match_prefix() {
    // Given
    let source = "import {css as __emotion_user} from '@devup-ui/react';\nconsume(__emotion_user);";

    // When
    let error = compile(source).unwrap_err();

    // Then
    assert_eq!(error, "namespace.tsx:2:9: `__emotion_user` is read at runtime, where it does not exist: the build compiles it only where it is called or rendered");
}

#[rstest]
#[case("E.cx")]
#[case("(E.cx)")]
#[case("(E.cx as typeof E.cx)")]
#[case("(E.cx satisfies typeof E.cx)")]
#[case("E.cx!")]
#[case("(E.merge)")]
#[case("(E.merge as typeof E.merge)")]
#[case("(E.merge satisfies typeof E.merge)")]
#[case("E.merge!")]
#[case("(E.cx<never>)")]
#[serial]
fn emotion_tag_error_restores_written_callee_when_wrapped(#[case] callee: &str) {
    // Given
    let source = format!("import * as E from '@emotion/css';\nexport const result={callee}`a b`;");
    let api = if callee.contains("merge") { "merge" } else { "cx" };

    // When
    let error = compile(&source).unwrap_err();

    // Then
    assert_eq!(error, format!("namespace.tsx:2:21: `{api}()` cannot use `{callee}`a b`` at build time: call it with class names, as in `cx('a', 'b')`"));
}

#[rstest]
#[case("a __emotion_user")]
#[case("a __emotion_cx_2")]
#[case("a \\n__emotion_user")]
#[case("${\"__emotion_user\"}")]
#[case("${/* __emotion_user */ unknown}")]
#[case("${__emotion_user}")]
#[case("${__emotion_cx_2}")]
#[serial]
fn emotion_tag_error_preserves_user_text_when_restoring_callee(#[case] text: &str) {
    // Given
    let source = format!("import * as E from '@emotion/css';\nexport const result=E.cx`{text}`;");

    // When
    let error = compile(&source).unwrap_err();

    // Then: readable_code removes comments, but restoration must not rewrite text or references.
    let shown = text.replace("/* __emotion_user */ ", "");
    assert_eq!(error, format!("namespace.tsx:2:21: `cx()` cannot use `E.cx`{shown}`` at build time: call it with class names, as in `cx('a', 'b')`"));
}

#[test]
#[serial]
fn emotion_tag_error_preserves_bound_interpolation_when_restoring_callee() {
    // Given
    let source = "import * as E from '@emotion/css';\nconst __emotion_user = unknown;\nexport const result=E.merge`a ${__emotion_user}`;";

    // When
    let error = compile(source).unwrap_err();

    // Then
    assert_eq!(error, "namespace.tsx:3:21: `merge()` cannot use `E.merge`a ${__emotion_user}`` at build time: call it with class names, as in `cx('a', 'b')`");
}

#[rstest]
#[case("(<typeof E.cx>E.cx)", "cx")]
#[case("(<typeof E.merge>E.merge)", "merge")]
#[serial]
fn emotion_tag_error_uses_ts_source_type_when_restoring_assertions(
    #[case] callee: &str,
    #[case] api: &str,
) {
    // Given
    let source = format!("import * as E from '@emotion/css';\nexport const result={callee}`a b`;");
    reset_class_map();
    reset_file_map();

    // When
    let error = extract("assertion.ts", &source, ExtractOption {
        import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    }).unwrap_err().to_string();

    // Then
    assert_eq!(error, format!("assertion.ts:2:21: `{api}()` cannot use `{callee}`a b`` at build time: call it with class names, as in `cx('a', 'b')`"));
}

#[test]
fn emotion_restoration_preserves_comments_and_requirements_when_they_name_generated_identifiers() {
    // Given
    let source = "import * as E from '@emotion/css';\nexport const result=E.cx/* __emotion_cx_2 __emotion_user */`a b`;";
    let message = utils::build_time_error("cx", "__emotion_cx_2/* __emotion_cx_2 __emotion_user */`a b`", "keep __emotion_cx_2 verbatim");

    // When
    let error = emotion_namespace::original_error_code(source, source.find("E.cx").unwrap(), message, SourceType::tsx());

    // Then
    assert_eq!(error, "`cx()` cannot use `E.cx/* __emotion_cx_2 __emotion_user */`a b`` at build time: keep __emotion_cx_2 verbatim");
}
