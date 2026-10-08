use super::*;

#[rstest]
#[case("undefined", "8px")]
#[case("null", "null")]
#[case("4", "4px")]
#[serial]
fn attrs_when_scalar_is_captured_emit_exact_text_once(#[case] input: &str, #[case] expected: &str) {
    // Given one scalar reused by two declarations and unused/extra arguments.
    let source = "import stylex from '@stylexjs/stylex'; const s=stylex.create({size:(height=8,unused)=>({height,width:height})}); const result=stylex.attrs(s.size(next(),unused(),extra()));";
    let output = extract(source).expect("attrs extraction");
    // When the emitted expression executes.
    let actual = execute(
        &output,
        &format!(
            "const trace=[]; function next(){{trace.push('next');return {input}}} function unused(){{trace.push('unused')}} function extra(){{trace.push('extra')}}"
        ),
        "[trace,result.style.split(';').map(x=>x.slice(x.indexOf(':')+1))]",
    );
    // Then attrs retains its text-null behavior and evaluates arguments once in order.
    assert_eq!(
        actual,
        format!("[[\"next\",\"unused\",\"extra\"],[\"{expected}\",\"{expected}\"]]")
    );
}

#[rstest]
#[case("s.tone(input),s.blue")]
#[case("s.blue,s.tone(input)")]
#[case("s.tone(),s.blue")]
#[case("s.blue,s.tone('red')")]
#[case("s.tone(input),s.tone(other)")]
#[case("s.tone(input),active?s.blue:s.hover")]
#[serial]
fn composition_when_dynamic_call_precedence_is_inexact_reports_location(#[case] args: &str) {
    // Given overlapping static, selector and dynamic namespace keys.
    let source = format!(
        "{}\nconst result=stylex.props({args});",
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({tone:(color='red')=>({color}),blue:{color:'blue'},hover:{color:{':hover':'green'}}});"
    );
    // When compilation cannot use the merged static keyed-composition path.
    let error = extract(&source)
        .expect_err("inexact combination")
        .to_string();
    // Then the compiler rejects the call instead of emitting wrong cascade order.
    let column = source
        .lines()
        .nth(1)
        .expect("call line")
        .find("s.tone")
        .expect("dynamic call")
        + 1;
    assert!(
        error.contains(&format!("/src/stylex.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("keyed precedence"), "{error}");
    assert!(error.contains("use one direct scalar call"), "{error}");
}

#[rstest]
#[case("props")]
#[case("attrs")]
#[serial]
fn static_composition_when_conditional_selector_and_null_keys_mix_keeps_merged_semantics(
    #[case] api: &str,
) {
    // Given a conditional later namespace that replaces the whole color key.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const s=stylex.create({{base:{{color:'red',opacity:1}},hover:{{color:{{':hover':'blue'}}}},reset:{{color:null}}}}); const result=stylex.{api}(s.base,active?s.hover:s.reset);"
    );
    let output = extract(&source).expect("static composition");
    // When the reset branch executes.
    let actual = execute(
        &output,
        "const active=false;",
        "Object.values(result).map(x=>x.trim())",
    );
    // Then only opacity's class survives, not the base color class.
    let opacity = output
        .styles
        .iter()
        .find_map(|value| match value {
            ExtractStyleValue::Static(style) if style.property == "opacity" => {
                Some(css::sheet_to_classname(
                    &style.property,
                    style.level,
                    Some(&style.value),
                    None,
                    style.style_order,
                    None,
                ))
            }
            _ => None,
        })
        .expect("opacity class");
    assert_eq!(actual, format!("[\"{opacity}\"]"));
}

#[rstest]
#[case("<Box {...stylex.props(s.tone(input))} color='blue' />")]
#[case("<Box color='blue' {...stylex.props(s.tone(input))} />")]
#[serial]
fn jsx_spread_when_dynamic_call_combination_is_inexact_is_located(#[case] jsx: &str) {
    // Given JSX composition around a dynamic StyleX call.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; import {{Box}} from '@devup-ui/react'; const s=stylex.create({{tone:(color='red')=>({{color}})}});\nconst result={jsx};"
    );
    // When the #756 spread lowering encounters an unrepresentable combination.
    let error = extract(&source)
        .expect_err("inexact JSX spread")
        .to_string();
    // Then the unsupported spread is rejected at its source location.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
}
