use super::*;

#[rstest]
#[case("base:{color:'red'}", "alias.base")]
#[case("base:color=>({color})", "alias.base(input)")]
#[serial]
fn stylex_alias_when_reassigned_reports_location(#[case] namespace: &str, #[case] argument: &str) {
    // Given an alias whose initializer is not its value at the styling call.
    let usage =
        format!("function render(next){{let alias=s;alias=next;return stylex.props({argument})}}");
    let source = format!(
        "import stylex from '@stylexjs/stylex';const s=stylex.create({{{namespace}}});\n{usage}"
    );
    // When the alias crosses the exact namespace boundary.
    let error = extract(&source)
        .expect_err("written alias cannot use initializer metadata")
        .to_string();
    // Then its initializer is located with an immutable-alias repair.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains("reassigned"), "{error}");
    assert!(error.contains("unchanged"), "{error}");
}

#[rstest]
#[case("s=next;")]
#[case("s.base=next;")]
#[serial]
fn stylex_alias_when_source_is_written_reports_location(#[case] write: &str) {
    // Given a source whose namespace/member cannot be proved unchanged.
    let source = format!(
        "import stylex from '@stylexjs/stylex';let s=stylex.create({{base:{{color:'red'}}}});\nfunction render(next){{const alias=s;{write}return stylex.props(alias.base)}}"
    );
    // When metadata would otherwise be copied from the written source.
    let error = extract(&source).expect_err("written source").to_string();
    // Then the original alias read is located, rather than cached speculatively.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains("reassigned"), "{error}");
}

#[test]
#[serial]
fn stylex_alias_when_other_scope_writes_same_name_keeps_symbol_identity() {
    // Given an unchanged let alias and a different binding with the same spelling.
    let source = "import stylex from '@stylexjs/stylex';const s=stylex.create({base:{color:'red'}});let alias=s;function unrelated(alias){alias=null}const result=stylex.props(alias.base);";
    // When scoped metadata is resolved and its emitted expression executes.
    let output = extract(source).expect("unchanged scoped alias");
    // Then another binding's writes do not invalidate this alias.
    assert_eq!(execute(&output, "", "result.className===s.base"), "true");
}

#[rstest]
#[case("function wrappers(){return stylex.types} const result=wrappers().color('red');")]
#[case("let t;t=stylex.types;const result=t.color('red');")]
#[case("function consume(t){return t.color('red')} const result=consume(stylex.types);")]
#[case("function wrappers(){return types} const result=wrappers().color('red');")]
#[case("let t;t=types;const result=t.color('red');")]
#[case("function consume(t){return t.color('red')} const result=consume(types);")]
#[serial]
fn stylex_types_object_when_escaping_reports_location(#[case] usage: &str) {
    // Given imported compiler placeholders escaping in non-declaration positions.
    let source = format!("import stylex,{{types}} from '@stylexjs/stylex';\n{usage}");
    // When final helper validation inspects the original object read.
    let error = extract(&source)
        .expect_err("types object escape")
        .to_string();
    // Then successful output cannot defer the compiler-required error to runtime.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains("function-value escape"), "{error}");
}

#[rstest]
#[case("undefined", "\"external\"")]
#[case("undefined,runtime", "\"external other\"")]
#[serial]
fn stylex_undefined_when_shadowed_preserves_runtime_class(
    #[values("props", "attrs")] api: &str,
    #[case] arguments: &str,
    #[case] expected: &str,
) {
    // Given a parameter, not the global undefined value.
    let source = format!(
        "import stylex from '@stylexjs/stylex';function render(undefined){{return stylex.{api}({arguments})}}const result=render('external');"
    );
    let output = extract(&source).expect("shadowed undefined class");
    let field = if api == "props" { "className" } else { "class" };
    // When actual emitted JavaScript reads the parameter.
    let actual = execute(
        &output,
        "const runtime='other';",
        &format!("result.{field}.trim()"),
    );
    // Then both fast and fallback composition keep its class value.
    assert_eq!(actual, expected);
}

#[rstest]
#[case("stylex.types")]
#[case("create")]
#[serial]
fn stylex_types_when_unrelated_nested_member_is_preserved(#[case] object: &str) {
    // Given the existing compatibility control, not a recognized types method.
    let source = format!(
        "import stylex, {{ create }} from '@stylexjs/stylex';const result={object}.defineVars({{a:'b'}});"
    );
    // When final validation walks the object of the unrelated member.
    let output = extract(&source).expect("unrelated nested member");
    // Then ordinary nested-member evaluation remains executable.
    assert_eq!(
        execute(
            &output,
            "const stylex={types:{defineVars:x=>x.a}}; const create={defineVars:x=>x.a};",
            "result"
        ),
        "\"b\""
    );
}
