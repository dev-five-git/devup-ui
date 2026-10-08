use super::*;

#[rstest]
#[case("color:'red',color:stylex.firstThatWorks()")]
#[case("color:{default:'red',default:stylex.firstThatWorks()}")]
#[case("':hover':{color:'red',color:stylex.firstThatWorks()}")]
#[case("color:'red',color:null")]
#[serial]
fn stylex_explicit_assignment_when_final_value_is_omitted_emits_no_color(
    #[values("props", "attrs")] api: &str,
    #[case] body: &str,
) {
    // Given repeated explicit assignments, distinct from namespace composition/includes.
    let source = format!(
        "import stylex from '@stylexjs/stylex';const s=stylex.create({{item:{{{body}}}}});const result=stylex.{api}(s.item);"
    );
    let output = extract(&source).expect("duplicate explicit assignments");
    let field = if api == "props" { "className" } else { "class" };
    // When the emitted result and emitted rules are observed.
    let actual = execute(&output, "", &format!("result.{field}"));
    // Then only the final explicit value contributes metadata/CSS.
    assert_eq!(actual, "\"\"");
    assert_eq!(output.styles.len(), 0);
}

#[rstest]
#[case("height:x,height:null", "8")]
#[case("height:x,height:null", "next(),extra()")]
#[case("height:8,height:null", "next(),extra()")]
#[serial]
fn stylex_dynamic_assignment_when_final_value_is_null_discards_height(
    #[values("props", "attrs")] api: &str,
    #[case] body: &str,
    #[case] argument: &str,
) {
    // Given a returned object whose final height assignment is null.
    let source = format!(
        "import stylex from '@stylexjs/stylex';const s=stylex.create({{size:x=>({{{body}}})}});const result=stylex.{api}(s.size({argument}));"
    );
    let output = extract(&source).expect("null duplicate body");
    let field = if api == "props" { "className" } else { "class" };
    // When emitted JavaScript executes even now-unused scalar arguments.
    let actual = execute(
        &output,
        "const trace=[];function next(){trace.push('next');return 8}function extra(){trace.push('extra');return 9}",
        &format!("[result.{field},trace]"),
    );
    // Then height is absent and supplied arguments still run once in source order.
    let trace = if argument == "8" {
        "[]"
    } else {
        "[\"next\",\"extra\"]"
    };
    assert_eq!(actual, format!("[\"\",{trace}]"));
    assert_eq!(output.styles.len(), 0);
}

#[rstest]
#[case("color:stylex.firstThatWorks(),color:'blue'", "blue")]
#[case("color:null,color:'blue'", "blue")]
#[case("color:{default:stylex.firstThatWorks(),default:'blue'}", "blue")]
#[serial]
fn stylex_explicit_assignment_when_final_value_is_present_keeps_value(
    #[case] body: &str,
    #[case] expected: &str,
) {
    // Given the reverse orders of empty/null overwrites.
    let source = format!(
        "import stylex from '@stylexjs/stylex';const s=stylex.create({{item:{{{body}}}}});const result=stylex.props(s.item);"
    );
    // When extraction publishes the final explicit value.
    let output = extract(&source).expect("final literal");
    // Then exactly its static rule survives.
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.value.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![expected]);
}

#[test]
#[serial]
fn stylex_dynamic_assignment_when_parameter_is_final_preserves_capture() {
    // Given a null assignment overwritten by a parameter with a default.
    let source = "import stylex from '@stylexjs/stylex';const s=stylex.create({size:(x=8)=>({height:null,height:x})});const result=stylex.props(s.size(next()));";
    let output = extract(source).expect("final parameter");
    // When actual emitted JavaScript captures undefined once.
    let actual = execute(
        &output,
        "let calls=0;function next(){calls++;return undefined}",
        "[calls,Object.values(result.style)]",
    );
    // Then the final height uses its default, unchanged by the discarded null.
    assert_eq!(actual, "[1,[\"8px\"]]");
}
