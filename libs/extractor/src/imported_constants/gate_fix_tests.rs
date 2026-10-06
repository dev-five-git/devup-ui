use super::exact_tests::{extracted, static_values};
use rstest::rstest;

#[rstest]
#[case("watch(made);function writer(){made.p=2}")]
#[case("function writer(){made.p=2}watch(made);")]
#[case("watch(alias);function writer(){alias.p=2}")]
#[case("function writer(){alias.p=2}watch(alias);")]
#[serial_test::serial]
fn gate_b1_when_deferred_write_precedes_copy_rejects_combined_hazards(#[case] hazards: &str) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';const make=()=>({{p:1}});const made=make();const alias=made;writer();const copied=made.p;{hazards}export const a=css({{p:copied}});"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{result:?}");
}

#[rstest]
#[case("watch(made);function writer(){made.w='99px'}")]
#[case("function writer(){made.w='99px'}watch(made);")]
#[case("watch(alias);function writer(){alias.w='99px'}")]
#[serial_test::serial]
fn gate_b1_when_element_copy_has_combined_hazards_keeps_runtime_width(#[case] hazards: &str) {
    // Given
    let source = format!(
        "import {{Box}} from '@devup-ui/react';const make=()=>({{w:'13px'}});const made=make();const alias=made;writer();const copied=made.w;{hazards}export const a=<Box w={{copied}}/>;"
    );
    // When
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), Vec::<String>::new());
    assert!(
        output.code.contains("copied") && output.code.contains("--"),
        "{}",
        output.code
    );
}

#[rstest]
#[case("watch(made);made.w='99px';")]
#[case("made.w='99px';watch(made);")]
#[case("const alias=made;watch(alias);alias.w='99px';")]
#[serial_test::serial]
fn gate_b1_when_copy_precedes_exclusively_eager_hazards_keeps_exact_width(#[case] hazards: &str) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';const made={{w:'13px'}};const copied=made.w;{hazards}export const a=css({{w:copied}});"
    );
    // When
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), vec!["13px".to_string()]);
}

#[rstest]
#[case("const f=()=>Math.imul(1,1)+'px';", "f()")]
#[case("const first=()=>Math.imul(1,1)+'px';const f=()=>first();", "f()")]
#[case(
    "function first(){return JSON.stringify(1)+'px'}const f=()=>first();",
    "f()"
)]
#[case("", "Math.imul(1,1)+'px'")]
#[case("", "JSON.stringify(1)+'px'")]
#[serial_test::serial]
fn gate_b2_when_indirect_eval_reaches_global_dependency_rejects_build_read(
    #[case] helpers: &str,
    #[case] value: &str,
) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';{helpers}(0,eval)('Math.imul=()=>9;JSON.stringify=()=>9');export const a=css({{w:{value}}});"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{result:?}");
}

#[rstest]
#[case("const f=()=>Math.imul(1,1)+'px';")]
#[case("const first=()=>JSON.stringify(1)+'px';const f=()=>first();")]
#[serial_test::serial]
fn gate_b2_when_element_helper_reads_opaque_global_keeps_css_variable(#[case] helpers: &str) {
    // Given
    let source = format!(
        "import {{Box}} from '@devup-ui/react';{helpers}(0,eval)(input);export const a=<Box w={{f()}}/>;"
    );
    // When
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), Vec::<String>::new());
    assert!(
        output.code.contains("f()") && output.code.contains("--"),
        "{}",
        output.code
    );
}

#[rstest]
#[case("const f=()=>13+'px';(0,eval)(input);", "f()")]
#[case("const first=()=>13;const f=()=>first()+'px';(0,eval)(input);", "f()")]
#[case("const copied=Math.imul(13,1)+'px';(0,eval)(input);", "copied")]
#[case("const copied=JSON.stringify(13)+'px';(0,eval)(input);", "copied")]
#[serial_test::serial]
fn gate_b2_when_global_barrier_cannot_change_local_snapshot_keeps_exact_width(
    #[case] prefix: &str,
    #[case] value: &str,
) {
    // Given
    let source =
        format!("import {{css}} from '@devup-ui/react';{prefix}export const a=css({{w:{value}}});");
    // When
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), vec!["13px".to_string()]);
}

#[rstest]
#[case("function make(){return {w:'13px'}}", "made.w")]
#[case("let make=()=>({w:'13px'});", "made.w")]
#[case("function make(){return {w:'13px'}}", "read()")]
#[case("let make=()=>({w:'13px'});", "read()")]
#[serial_test::serial]
fn gate_b3_when_factory_binding_is_written_reports_semantic_origin(
    #[case] factory: &str,
    #[case] value: &str,
) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';\n{factory}\nconst made=make();\nmake=()=>({{w:'99px'}});\nconst read=()=>made.w;\nexport const a=css({{w:{value}}});"
    );
    // When
    let error = extracted(&source, "")
        .err()
        .unwrap_or_else(|| panic!("factory read must fail"));
    // Then: consumer and original semantic write, not just generic rejection.
    assert!(
        error.contains("/src/App.tsx:6:")
            && error.contains("/src/App.tsx:4:1")
            && error.contains("`make`")
            && error.contains("make=()=>")
            && error.contains("use a direct value"),
        "{error}"
    );
}

#[rstest]
#[case("p", "1", "2")]
#[case("w", "'13px'", "'99px'")]
#[serial_test::serial]
fn gate_b1_when_original_public_source_has_both_hazards_rejects_stale_fold(
    #[case] property: &str,
    #[case] initial: &str,
    #[case] assigned: &str,
) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';const make=()=>({{{property}:{initial}}});const made=make();writer();const copied=made.{property};watch(made);function writer(){{made.{property}={assigned}}}export const a=css({{{property}:copied}});"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{result:?}");
}

#[test]
#[serial_test::serial]
fn gate_b2_when_transitive_global_is_rejected_reports_eval_origin() {
    // Given
    let source = "import {css} from '@devup-ui/react';\nconst first=()=>Math.imul(1,1)+'px';const f=()=>first();\n(0,eval)(input);\nexport const a=css({w:f()});";
    // When
    let error = extracted(source, "")
        .err()
        .unwrap_or_else(|| panic!("global read must fail"));
    // Then
    assert!(
        error.contains("/src/App.tsx:4:")
            && error.contains("/src/App.tsx:3:1")
            && error.contains("`Math`")
            && error.contains("(0,eval)(input)"),
        "{error}"
    );
}
