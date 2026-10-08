use super::*;

#[rstest]
#[case("opacity", "0.123456", ".123456")]
#[case("height", "0.123456", ".123456px")]
#[case("animationDuration", "0.123456", ".123456ms")]
#[case("opacity", "-0", "0")]
#[case("opacity", "1e-7", "1e-7")]
#[case("opacity", "1e21", "1e+21")]
#[serial]
fn stylex_dynamic_when_specialized_preserves_exact_numeric_scalar(
    #[values("", "value")] argument: &str,
    #[case] property: &str,
    #[case] value: &str,
    #[case] expected: &str,
) {
    // Given an exact numeric default and either an omitted or explicit scalar argument.
    let argument = argument.replace("value", value);
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{tone:(x={value})=>({{{property}:x}})}}); const result=stylex.props(s.tone({argument}));"
    );
    // When the dynamic function is specialized to a static CSS class.
    let output = extract(&source).expect("exact specialization");
    let actual: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.value.as_str()),
            _ => None,
        })
        .collect();
    // Then no four-decimal conversion changes its scalar text.
    assert_eq!(actual, vec![expected]);
    assert_eq!(execute(&output, "", "'style' in result"), "false");
}

#[rstest]
#[case("opacity", "0.123456", "0.123456")]
#[case("height", "0.123456", "0.123456px")]
#[case("animationDuration", "0.123456", "0.123456ms")]
#[case("opacity", "-0", "0")]
#[case("opacity", "1e-7", "1e-7")]
#[case("opacity", "1e21", "1e+21")]
#[serial]
fn stylex_dynamic_when_runtime_value_is_numeric_matches_specialized_scalar(
    #[case] property: &str,
    #[case] value: &str,
    #[case] expected: &str,
) {
    // Given the same numeric inputs as the specialized-call cases.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{tone:x=>({{{property}:x}})}}); const result=stylex.props(s.tone(input));"
    );
    // When the scalar is supplied through the CSS-variable path.
    let output = extract(&source).expect("runtime numeric scalar");
    let actual = execute(
        &output,
        &format!("const input={value};"),
        "Object.values(result.style).map(String)",
    );
    // Then JS numeric text and the dynamic unit agree with specialization.
    assert_eq!(actual, format!("[\"{expected}\"]"));
}

#[rstest]
#[case("true", "true")]
#[case("false", "false")]
#[serial]
fn stylex_dynamic_when_boolean_default_or_call_is_known_keeps_scalar(
    #[values("", "value")] argument: &str,
    #[case] value: &str,
    #[case] expected: &str,
) {
    // Given booleans accepted by the existing scalar dynamic contract.
    let argument = argument.replace("value", value);
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{tone:(x={value})=>({{opacity:x}})}}); const result=stylex.props(s.tone({argument}));"
    );
    // When specializing the call.
    let output = extract(&source).expect("boolean scalar");
    // Then the boolean is neither dropped nor interpreted as a number.
    assert!(
        output.styles.iter().any(
            |style| matches!(style, ExtractStyleValue::Static(style) if style.value == expected)
        )
    );
}

#[test]
#[serial]
fn stylex_dynamic_when_boolean_default_is_runtime_captured_keeps_boolean() {
    // Given a boolean default exercised through runtime capture.
    let source = "import stylex from '@stylexjs/stylex'; const s=stylex.create({tone:(x=false)=>({'opacity':x})}); const result=stylex.props(s.tone(input));";
    // When the runtime argument is undefined.
    let output = extract(source).expect("boolean capture");
    // Then the default remains a boolean, exercising the emitted scalar expression.
    assert_eq!(
        execute(
            &output,
            "const input=undefined;",
            "Object.values(result.style)"
        ),
        "[false]"
    );
}

#[rstest]
#[case("color(){return x}")]
#[case("get color(){return x}")]
#[case("set color(value){}")]
#[serial]
fn stylex_dynamic_when_body_property_is_method_getter_or_setter_is_located(#[case] body: &str) {
    // Given body accessors that cannot be converted to plain assignments.
    let usage = format!("const s=stylex.create({{tone:x=>({{{body}}})}});");
    let source = format!("import stylex from '@stylexjs/stylex';\n{usage}");
    // When extraction reads the body.
    let error = extract(&source).expect_err("accessor body").to_string();
    // Then it points to that property instead of ignoring its function behavior.
    let column = usage.find(body).expect("body") + 1;
    assert!(
        error.contains(&format!("/src/stylex.tsx:2:{column}:")),
        "{error}"
    );
    assert!(
        error.contains("method/getter/setter body property"),
        "{error}"
    );
}

#[rstest]
#[case("s.tone(input),s.base")]
#[case("s.base,s.tone(input)")]
#[serial]
fn stylex_dynamic_when_static_key_is_disjoint_retains_both(#[case] arguments: &str) {
    // Given a static opacity and a dynamic color with disjoint keys.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{base:{{opacity:1}},tone:color=>({{color}})}}); const result=stylex.props({arguments});"
    );
    // When the emitted composition executes with a runtime scalar.
    let output = extract(&source).expect("disjoint composition");
    // Then static classes and dynamic assignments both survive.
    assert_eq!(
        execute(
            &output,
            "const input='red';",
            "[result.className.split(' ').includes(s.base),Object.values(result.style)]"
        ),
        "[true,[\"red\"]]"
    );
}

#[rstest]
#[case("s.tone(input),...rest", "spread alongside dynamic calls")]
#[case("...rest,s.tone(input)", "spread alongside dynamic calls")]
#[case("s.base()", "unresolved namespace calls")]
#[case("s.base,s.reset(input)", "keyed precedence")]
#[case("s.tone(input),runtime", "keyed precedence")]
#[serial]
fn stylex_dynamic_when_composition_is_inexact_is_located(
    #[case] arguments: &str,
    #[case] cause: &str,
) {
    // Given spread order, static-member calls and null-body overlap edge cases.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{base:{{color:'red'}},tone:color=>({{color}}),reset:x=>({{color:null,height:x}})}});\nconst result=stylex.props({arguments});"
    );
    // When compilation cannot preserve the composition.
    let error = extract(&source)
        .expect_err("inexact composition")
        .to_string();
    // Then no runtime class or null-overridden static color is emitted silently.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains(cause), "{error}");
}
