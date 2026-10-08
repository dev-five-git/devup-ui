use super::*;

#[rstest]
#[case("base:{color:'red'}", "s.base='external';", "s.base")]
#[case("base:{color:'red'}", "s=next;", "s.base")]
#[case("base:color=>({color})", "s.base=next;", "s.base(input)")]
#[serial]
fn stylex_original_namespace_when_written_is_located(
    #[case] namespace: &str,
    #[case] write: &str,
    #[case] argument: &str,
) {
    // Given original namespace writes with no intervening alias declaration.
    let source = format!(
        "import stylex from '@stylexjs/stylex';\nlet s=stylex.create({{{namespace}}});function render(next){{{write}return stylex.props({argument})}}"
    );
    // When original metadata would otherwise produce a stale class or dynamic assignment.
    let error = extract(&source)
        .expect_err("original written namespace")
        .to_string();
    // Then the original binding is located with an unchanged-namespace repair.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains("unchanged"), "{error}");
}

#[rstest]
#[case("{default:'red',default:null}")]
#[case("{'@media print':'red','@media print':null}")]
#[case("{default:{'@media print':'red','@media print':null}}")]
#[case("stylex.types.color({default:'red',default:null})")]
#[serial]
fn stylex_variable_conditions_when_final_assignment_is_null_emit_no_rule(
    #[values("defineVars", "createTheme")] api: &str,
    #[case] value: &str,
) {
    // Given duplicate variable conditions and the existing null-omission contract.
    let declaration = if api == "defineVars" {
        format!("const result=stylex.defineVars({{text:{value}}});")
    } else {
        format!(
            "const contract=stylex.createThemeContract({{text:null}});const result=stylex.createTheme(contract,{{text:{value}}});"
        )
    };
    let source = format!("import stylex from '@stylexjs/stylex';{declaration}");
    // When the actual extractor and emitted value are observed.
    let output = extract(&source).expect("final null condition");
    let actual = execute(&output, "", "typeof result");
    // Then references/theme values remain valid while no overwritten CSS assignment survives.
    assert_eq!(
        actual,
        if api == "defineVars" {
            "\"object\""
        } else {
            "\"string\""
        }
    );
    assert_eq!(output.styles.len(), 0);
}

#[rstest]
#[case("{default:'red',default:'blue'}")]
#[case("{'@media print':'red','@media print':'blue'}")]
#[case("{default:null,default:'blue'}")]
#[serial]
fn stylex_variable_conditions_when_final_assignment_has_value_keeps_only_value(
    #[values("defineVars", "createTheme")] api: &str,
    #[case] value: &str,
) {
    // Given distinct earlier and final values, including reverse null order.
    let declaration = if api == "defineVars" {
        format!("const result=stylex.defineVars({{text:{value}}});")
    } else {
        format!(
            "const contract=stylex.createThemeContract({{text:null}});const result=stylex.createTheme(contract,{{text:{value}}});"
        )
    };
    let source = format!("import stylex from '@stylexjs/stylex';{declaration}");
    // When the variable reader emits its CSS rules.
    let output = extract(&source).expect("final literal condition");
    let css = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Css(css) => Some(css.css.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    // Then only blue, not the overwritten red, is assigned.
    assert!(css.contains(":blue;"), "{css}");
    assert!(!css.contains(":red;"), "{css}");
}

#[test]
#[serial]
fn stylex_original_namespace_when_other_scope_writes_same_name_is_valid() {
    // Given immutable original metadata and an unrelated mutated parameter.
    let source = "import stylex from '@stylexjs/stylex';const s=stylex.create({base:{color:'red'}});function other(s){s.base='external';s=null}const result=stylex.props(s.base);";
    // When scoped publication and the emitted result execute.
    let output = extract(source).expect("different lexical symbol");
    // Then unrelated writes cannot discard the original class.
    assert_eq!(execute(&output, "", "result.className===s.base"), "true");
}

#[rstest]
#[case("defineVars")]
#[case("createTheme")]
#[serial]
fn stylex_variable_conditions_when_overwritten_value_is_unreadable_still_error(#[case] api: &str) {
    // Given an overwritten condition whose runtime evaluation cannot be discarded.
    let value = "{default:runtime(),default:null}";
    let declaration = if api == "defineVars" {
        format!("const result=stylex.defineVars({{text:{value}}});")
    } else {
        format!(
            "const contract=stylex.createThemeContract({{text:null}});const result=stylex.createTheme(contract,{{text:{value}}});"
        )
    };
    let source = format!("import stylex from '@stylexjs/stylex';\n{declaration}");
    // When every supplied condition value crosses the exactness boundary.
    let error = extract(&source)
        .expect_err("unreadable overwritten condition")
        .to_string();
    // Then the discarded value still receives a located runtime-value error.
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains("runtime()"), "{error}");
}
