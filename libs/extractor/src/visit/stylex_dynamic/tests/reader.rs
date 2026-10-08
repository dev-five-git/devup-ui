use super::*;

#[rstest]
#[case("const tone=s.tone; const result=stylex.props(tone(input));", "s.tone")]
#[case("const result=stylex.props(s.tone);", "s.tone")]
#[case("const result=stylex.attrs([s['tone']]);", "s['tone']")]
#[case("const result=consume(s.tone);", "s.tone")]
#[case("const alias=s; const result=stylex.props(alias.tone);", "alias.tone")]
#[serial]
fn stylex_dynamic_function_value_when_escaping_is_located(
    #[case] usage: &str,
    #[case] marker: &str,
) {
    // Given a dynamic namespace whose function value cannot survive lowering.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{tone:color=>({{color}})}});\n{usage}"
    );
    // When extraction sees the original member read.
    let error = extract(&source).expect_err("function escape").to_string();
    // Then the error points to that read instead of permitting a string call.
    let column = usage.find(marker).expect("member") + 1;
    assert!(
        error.contains(&format!("/src/stylex.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("uncalled dynamic namespace"), "{error}");
}

#[test]
#[serial]
fn stylex_namespace_when_static_alias_runtime_or_shadowed_remains_valid() {
    // Given a static member alias, runtime classes and a shadowed dynamic namespace name.
    let source = "import stylex from '@stylexjs/stylex'; const s=stylex.create({base:{color:'red'},tone:color=>({color})}); const alias=s.base; const namespace=s; function render(s){return stylex.props(s.tone)} const result=stylex.props(alias,namespace.base,runtime); const shadow=render({tone:'local'});";
    // When emitted output executes against plain class values.
    let output = extract(source).expect("valid member usage");
    let actual = execute(
        &output,
        "const runtime='external';",
        "[result.className.includes('external'),shadow.className]",
    );
    // Then static aliases and unrelated local values are not mistaken for functions.
    assert_eq!(actual, "[true,\"local\"]");
}

#[rstest]
#[case("...stylex.include(s.base,s.base)", "exactly one namespace")]
#[case(
    "color:stylex.types.color('red',runtime())",
    "extra arguments are evaluated"
)]
#[case("color:stylex.types.color()", "requires a value")]
#[case("color:{'@media':'red'}", "nonempty query")]
#[case("color:{'@mediaOops print':'red'}", "exact @media")]
#[serial]
fn stylex_create_when_helper_or_condition_is_malformed_reports_location(
    #[case] value: &str,
    #[case] cause: &str,
) {
    // Given malformed candidates previously consumed without validation.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{base:{{color:'red'}}}});\nconst other=stylex.create({{base:{{{value}}}}});"
    );
    // When the enclosing create consumes its value.
    let error = extract(&source).expect_err("malformed value").to_string();
    // Then successful output is replaced by a located actionable error.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains(cause), "{error}");
}

#[rstest]
#[case("stylex.firstThatWorks('red')")]
#[case("stylex.include(styles.base)")]
#[case("stylex.types.color('red')")]
#[case("fallback('red')")]
#[case("include(styles.base)")]
#[case("types.color('red')")]
#[serial]
fn stylex_helper_when_surviving_transform_is_rejected(#[case] expression: &str) {
    // Given namespace and named helper imports outside a consumed value.
    let source = format!(
        "import stylex,{{firstThatWorks as fallback,include,types}} from '@stylexjs/stylex';\nconst value={expression};"
    );
    // When the final, separate validation walk executes.
    let error = extract(&source).expect_err("surviving helper").to_string();
    // Then the recognized API cannot remain in successful emitted code.
    assert!(error.contains("/src/stylex.tsx:2:13:"), "{error}");
    assert!(error.contains("outside a consumed"), "{error}");
}

#[rstest]
#[case(
    "defineVars",
    "stylex.types.color('red',runtime())",
    "extra arguments are evaluated"
)]
#[case("defineVars", "{'@supports':'red'}", "nonempty query")]
#[case("createThemeContract", "{text:null}", "flat null/string")]
#[case("createThemeContract", "runtimeValue", "flat null/string")]
#[case("createThemeContract", "42", "flat null/string")]
#[case("createThemeContract", "false", "flat null/string")]
#[serial]
fn stylex_variables_when_invalid_retain_value_location(
    #[case] api: &str,
    #[case] value: &str,
    #[case] cause: &str,
) {
    // Given malformed variable values or non-flat contract placeholders.
    let source = format!(
        "import stylex from '@stylexjs/stylex';\nconst vars=stylex.{api}({{palette:{value}}});"
    );
    // When reading the contract before allocating references.
    let error = extract(&source).expect_err("invalid variable").to_string();
    // Then the value's location and repair are reported.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains(cause), "{error}");
}

#[rstest]
#[case("props")]
#[case("attrs")]
#[serial]
fn stylex_valid_consumed_helpers_and_flat_contract_when_compiled_leave_no_calls(#[case] api: &str) {
    // Given valid helpers, condition objects, null/string placeholders and an empty namespace.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const vars=stylex.createThemeContract({{text:null,other:'placeholder'}}); const theme=stylex.createTheme(vars,{{text:stylex.types.color({{default:'red','@media print':'blue'}}),other:null}}); const s=stylex.create({{base:{{color:stylex.types.color('red')}},empty:null}}); const more=stylex.create({{base:{{...stylex.include(s.base),fontFamily:stylex.firstThatWorks('Arial','sans-serif')}}}}); const result=stylex.{api}(more.base,s.empty,theme);"
    );
    // When extraction consumes the enclosing APIs.
    let output = extract(&source).expect("valid helpers");
    // Then emitted output runs without any recognized styling API calls.
    assert!(!output.code.contains("stylex."), "{}", output.code);
    assert_eq!(execute(&output, "", "typeof result"), "\"object\"");
}

#[rstest]
#[case("first()?s.a:s.b", "[\"first\"]")]
#[case("first()?s.a:s.a", "[\"first\"]")]
#[case("first()?s.empty:s.empty", "[\"first\"]")]
#[case("first()?s.a:s.b,s.a", "[\"first\"]")]
#[case("first()&&s.empty,s.a", "[\"first\"]")]
#[case("[first()?s.a:s.b,second()?s.b:s.a],s.a", "[\"first\",\"second\"]")]
#[case("first()?s.empty:s.empty,runtime", "[\"first\"]")]
#[case("first()&&s.empty,runtime", "[\"first\"]")]
#[serial]
fn stylex_static_conditions_when_composed_evaluate_once_in_source_order(
    #[values("props", "attrs")] api: &str,
    #[case] arguments: &str,
    #[case] expected: &str,
) {
    // Given known class choices with observable condition evaluation.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{a:{{color:'red',opacity:1}},b:{{color:'blue',opacity:0}},empty:{{}}}}); const result=stylex.{api}({arguments});"
    );
    let output = extract(&source).expect("conditional composition");
    // When executing the emitted expression.
    let actual = execute(
        &output,
        "const runtime='external';const trace=[];function first(){trace.push('first');return true}function second(){trace.push('second');return false}",
        "trace",
    );
    // Then multiple keys, overrides and identical/empty choices never duplicate or drop tests.
    assert_eq!(actual, expected);
}
