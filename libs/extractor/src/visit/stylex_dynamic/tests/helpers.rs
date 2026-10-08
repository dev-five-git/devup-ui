use super::*;

#[rstest]
#[case("s.empty", "\"\"")]
#[case("s.red,s.empty", "s.red")]
#[case("s.empty,s.red", "s.red")]
#[case("s.red,s.reset", "\"\"")]
#[case("s.reset,s.red", "s.red")]
#[case("s.red,s.conditionalEmpty", "s.red")]
#[case("s.conditionalEmpty,s.red", "s.red")]
#[serial]
fn stylex_empty_fallback_when_composed_omits_key_instead_of_cancelling(
    #[values("props", "attrs")] api: &str,
    #[case] arguments: &str,
    #[case] expected: &str,
) {
    // Given distinct omission and explicit-null namespaces.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{red:{{color:'red'}},empty:{{color:stylex.firstThatWorks()}},conditionalEmpty:{{color:{{default:stylex.firstThatWorks()}}}},reset:{{color:null}}}}); const result=stylex.{api}({arguments});"
    );
    // When the actual emitted composition runs.
    let output = extract(&source).expect("empty fallback accepted");
    let field = if api == "props" { "className" } else { "class" };
    let actual = execute(&output, "", &format!("result.{field} === {expected}"));
    // Then only explicit null can erase red, and omission emits no extra CSS.
    assert_eq!(actual, "true");
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Static(_)))
            .count(),
        1
    );
    assert!(!output.code.contains("stylex."), "{}", output.code);
}

#[test]
#[serial]
fn stylex_empty_fallback_when_alone_emits_no_css() {
    // Given the verified zero-argument helper form.
    let source = "import {create,firstThatWorks,props} from '@stylexjs/stylex'; const s=create({empty:{color:firstThatWorks()}}); const result=props(s.empty);";
    // When extraction consumes it.
    let output = extract(source).expect("empty fallback");
    // Then neither styles nor a color class exists.
    assert_eq!(output.styles.len(), 0);
    assert_eq!(
        execute(&output, "", "[s.empty,result.className]"),
        "[\"\",\"\"]"
    );
    assert!(!output.code.contains("firstThatWorks("));
}

#[rstest]
#[case("'ignored'")]
#[case("42")]
#[case("null")]
#[case("false")]
#[case("{nested:['ignored',42,null]}")]
#[case("EXTRA")]
#[serial]
fn stylex_types_when_extra_is_static_evaluates_then_ignores(
    #[values("defineVars", "createTheme", "create")] api: &str,
    #[case] extra: &str,
) {
    // Given valid exported defineVars, isolated from upstream outer-export validation.
    let wrapper = format!("stylex.types.color({{default:'red','@media print':'blue'}},{extra})");
    let usage = match api {
        "defineVars" => format!("export const result=stylex.defineVars({{text:{wrapper}}});"),
        "createTheme" => format!(
            "export const vars=stylex.defineVars({{text:'black'}}); const result=stylex.createTheme(vars,{{text:{wrapper}}});"
        ),
        "create" => format!("const result=stylex.create({{base:{{color:{wrapper}}}}});"),
        _ => panic!("unknown test API"),
    };
    let source = format!("import stylex from '@stylexjs/stylex'; const EXTRA=1+2; {usage}");
    // When the helper is consumed after normal constant folding.
    let output = extract(&source).expect("static extra accepted");
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.value.as_str()),
            ExtractStyleValue::Css(css) => Some(css.css.as_str()),
            _ => None,
        })
        .collect();
    // Then the primary conditional values survive, never the extra value or API call.
    assert!(
        values.iter().any(|value| value.contains("red")),
        "{values:?}"
    );
    assert!(
        values.iter().any(|value| value.contains("blue")),
        "{values:?}"
    );
    assert!(!output.code.contains("stylex."), "{}", output.code);
}

#[rstest]
#[case("runtimeValue")]
#[case("sideEffect()")]
#[case("{value:sideEffect()}")]
#[case("...values")]
#[serial]
fn stylex_types_when_extra_requires_runtime_reports_location(#[case] extra: &str) {
    // Given an exported variable whose extra helper argument cannot be evaluated statically.
    let source = format!(
        "import stylex from '@stylexjs/stylex';\nexport const vars=stylex.defineVars({{text:stylex.types.color('red',{extra})}});"
    );
    // When the value boundary examines every supplied argument.
    let error = extract(&source)
        .expect_err("runtime extra must not be discarded")
        .to_string();
    // Then a located error replaces output that would drop side effects.
    assert!(error.contains("/src/stylex.tsx:2:43:"), "{error}");
    assert!(error.contains("extra arguments are evaluated"), "{error}");
}

#[rstest]
#[case(
    "const f=stylex.types.color; const result=f('red');",
    "stylex.types.color"
)]
#[case("const f=types.color; const result=f('red');", "types.color")]
#[case(
    "const f=stylex.types['color']; const result=f('red');",
    "stylex.types['color']"
)]
#[serial]
fn stylex_types_when_method_value_escapes_is_located(#[case] usage: &str, #[case] marker: &str) {
    // Given helper method aliases unsupported by scoped API consumption.
    let source = format!("import stylex,{{types}} from '@stylexjs/stylex';\n{usage}");
    // When the final walk sees the unconsumed method value.
    let error = extract(&source)
        .expect_err("wrapper method escape")
        .to_string();
    // Then it points at the escape, not a surviving runtime call.
    let column = usage.find(marker).expect("marker") + 1;
    assert!(
        error.contains(&format!("/src/stylex.tsx:2:{column}:")),
        "{error}"
    );
    assert!(
        error.contains("wrapper-method function-value escape"),
        "{error}"
    );
}

#[test]
#[serial]
fn stylex_types_when_shadowed_member_is_not_a_helper_remains_callable() {
    // Given a parameter named types and an unrelated object with the same method name.
    let source = "import {types} from '@stylexjs/stylex'; function render(types){const f=types.color;return f('red')} const local={color:x=>x}; const result=render(local);";
    // When the scoped final walk checks only recognized imports.
    let output = extract(source).expect("shadowed types");
    // Then the ordinary function alias remains executable.
    assert_eq!(execute(&output, "", "result"), "\"red\"");
}

#[rstest]
#[case("stylex.firstThatWorks", "f('red')")]
#[case("fallback", "f('red')")]
#[case("stylex.include", "f(s.base)")]
#[case("include", "f(s.base)")]
#[case("stylex.types", "f.color('red')")]
#[case("types", "f.color('red')")]
#[serial]
fn stylex_helper_when_function_or_object_alias_escapes_is_located(
    #[case] value: &str,
    #[case] call: &str,
) {
    // Given an alias whose runtime placeholder would otherwise survive lowering.
    let usage = format!("const f={value}; const result={call};");
    let source = format!(
        "import stylex,{{firstThatWorks as fallback,include,types}} from '@stylexjs/stylex'; const s=stylex.create({{base:{{color:'red'}}}});\n{usage}"
    );
    // When final validation sees the unconsumed helper read.
    let error = extract(&source).expect_err("helper escape").to_string();
    // Then the alias initializer, not its later call, is located.
    assert!(error.contains("/src/stylex.tsx:2:9:"), "{error}");
    assert!(
        error.contains("unconsumed helper function-value escape"),
        "{error}"
    );
}
