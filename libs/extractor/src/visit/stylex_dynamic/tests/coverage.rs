use super::*;

#[rstest]
#[case("props", "Object.values(result.style)", "[null]")]
#[case(
    "attrs",
    "result.style.split(';').map(x=>x.slice(x.indexOf(':')+1))",
    "[\"null\"]"
)]
#[serial]
fn stylex_null_default_when_runtime_call_returns_undefined_keeps_null(
    #[case] api: &str,
    #[case] values: &str,
    #[case] expected: &str,
) {
    // Given a genuine null default, not an explicit known null argument.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{tone:(color=null)=>({{color}})}}); const result=stylex.{api}(s.tone(next()));"
    );
    // When the captured runtime argument returns undefined.
    let output = extract(&source).expect("null default capture");
    let actual = execute(
        &output,
        "let calls=0; function next(){calls++;return undefined}",
        &format!("[calls,{values}]"),
    );
    // Then props retains null, attrs serializes null, and the call runs once.
    assert_eq!(actual, format!("[1,{expected}]"));
}

#[rstest]
#[case("s[key]")]
#[case("s?.empty")]
#[case("s?.missing")]
#[serial]
fn stylex_empty_namespace_when_lookup_is_computed_or_optional_adds_no_class(
    #[values("props", "attrs")] api: &str,
    #[case] member: &str,
) {
    // Given a namespace with no nonempty static class and an unrelated runtime class.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{empty:{{}}}}); const result=stylex.{api}({member},runtime);"
    );
    // When the fallback member resolver handles a runtime key or optional static access.
    let output = extract(&source).expect("empty namespace lookup");
    let actual = execute(
        &output,
        "const key='empty'; const runtime='external';",
        "Object.values(result)",
    );
    // Then the local empty/missing class is omitted without dropping the external one.
    assert_eq!(actual, "[\"external\"]");
    assert_eq!(output.styles.len(), 0);
}

#[rstest]
#[case("color:{[condition]:'red'}")]
#[case("':hover':{color:{[condition]:'red'}}")]
#[serial]
fn stylex_condition_when_key_is_runtime_reports_exact_key_location(#[case] body: &str) {
    // Given a real parsed computed condition key under either static-style caller.
    let usage = format!("const s=stylex.create({{base:{{{body}}}}});");
    let source = format!("import stylex from '@stylexjs/stylex';\n{usage}");
    // When shared condition decomposition parses the key before at-rule validation.
    let error = extract(&source)
        .expect_err("runtime condition key")
        .to_string();
    // Then the original unknown-key contract and source location are preserved.
    let column = usage.find("condition").expect("key location") + 1;
    assert!(
        error.contains(&format!("/src/stylex.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("`[condition]`"), "{error}");
    assert!(error.contains("its keys must be known"), "{error}");
}
