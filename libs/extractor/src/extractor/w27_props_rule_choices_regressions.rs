use super::super::w27_props_rule_choices_tests::support::{active, declarations};
use rstest::rstest;

#[rstest]
#[case(
    "function make(undefined) { return BODY; }",
    "(p.on ? { color: 'red' } : undefined) ?? { color: 'blue' }"
)]
#[case(
    "const undefined = value; const Choice = BODY;",
    "(p.on ? { color: 'red' } : undefined) ?? { color: 'blue' }"
)]
#[case(
    "function make(undefined = value) { return BODY; }",
    "p.on ? undefined : { color: 'red' }"
)]
#[case(
    "function make(undefined) { return BODY; }",
    "undefined || { color: 'blue' }"
)]
#[case(
    "function make(undefined) { return BODY; }",
    "undefined ?? { color: 'blue' }"
)]
#[case("function make(undefined) { return BODY; }", "p.on && undefined")]
#[case(
    "const red = css({ color: 'red' }); function make(undefined) { return BODY; }",
    "p.on ? red : undefined"
)]
#[serial_test::serial]
fn bound_undefined_when_used_as_unknown_rule_shape_reports_original_callback_location(
    #[case] scope: &str,
    #[case] choice: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: the spelling is a real parameter, capture or defaulted binding.
    for body in [
        format!("styled.div(p => {choice})"),
        format!("styled.div`${{p => {choice}}}`"),
    ] {
        let source = format!(
            "import {{ styled, css }} from '@devup-ui/react';\n{}",
            scope.replace("BODY", &body)
        );
        let callback_start = source.find("p =>").ok_or("missing callback source")?;
        let line_start = source.find('\n').ok_or("missing callback line")? + 1;
        let location = format!("shadowed-choice.tsx:2:{}:", callback_start - line_start + 1);
        // When
        let error = crate::extract(
            "shadowed-choice.tsx",
            &source,
            crate::ExtractOption::default(),
        )
        .err()
        .unwrap_or_else(|| panic!("bound unknown rule shape compiled: {source}"))
        .to_string();
        // Then: the diagnostic still names the original source callback, not synthetic code.
        assert!(error.contains(&location), "{error}");
        assert!(error.contains("cannot use `"), "{error}");
        assert!(error.contains("undefined"), "{error}");
        assert!(
            error.contains("p.on") || choice.starts_with("undefined"),
            "{error}"
        );
    }
    Ok(())
}

#[rstest]
#[case("undefined")]
#[case("void 0")]
#[case("null")]
#[serial_test::serial]
fn global_nullish_when_rule_branch_is_absent_keeps_blue_fallback(#[case] sentinel: &str) {
    // Given
    let choice = format!("(p.on ? {{ color: 'red' }} : {sentinel}) ?? {{ color: 'blue' }}");
    for body in [
        format!("styled.div(p => {choice})"),
        format!("styled.div`${{p => {choice}}}`"),
    ] {
        // When
        let output = compile(&format!("const Choice = {body};"));
        // Then
        assert_eq!(
            declarations(&active(&output, "{ on: false }")),
            [("color".into(), "blue".into())]
        );
    }
}

#[test]
#[serial_test::serial]
fn bound_undefined_when_scalar_css_value_remains_a_css_variable() {
    // Given
    let source = "function make(undefined) { return styled.div(p => ({ color: undefined })); }";
    // When
    let output = compile(source);
    // Then
    assert!(output.code.contains("undefined"));
    assert!(output.code.contains("--"));
    assert!(output.styles.iter().any(|style| matches!(style, crate::ExtractStyleValue::Dynamic(value) if value.identifier().contains("undefined"))));
}

#[test]
#[serial_test::serial]
fn bound_undefined_when_unreachable_rule_branch_is_skipped() {
    // Given
    for choice in [
        "({ color: 'red' }) ?? undefined",
        "({ color: 'red' }) || undefined",
        "(false && undefined) || { color: 'red' }",
    ] {
        // When
        for body in [
            format!("styled.div(p => {choice})"),
            format!("styled.div`${{p => {choice}}}`"),
        ] {
            let output = compile(&format!("function make(undefined) {{ return {body}; }}"));
            // Then
            assert!(output.styles.iter().any(|style| matches!(style, crate::ExtractStyleValue::Static(value) if value.property() == "color" && value.value() == "red")));
        }
    }
}

fn compile(rules: &str) -> crate::ExtractOutput {
    let source = format!("import {{ styled }} from '@devup-ui/react'; {rules}");
    crate::extract(
        "w27-choices-active.tsx",
        &source,
        crate::ExtractOption {
            single_css: true,
            ..crate::ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

#[test]
#[serial_test::serial]
fn short_circuit_choices_preserve_active_atoms_when_props_are_truthy_falsy_or_nullish() {
    // Given: a prior atom and written choices with distinct falsy/nullish behavior.
    for (choice, off, nullish) in [
        (
            "(p.on && { color: 'red' }) || { color: 'blue' }",
            "blue",
            "blue",
        ),
        (
            "(p.on ? { color: 'red' } : null) ?? { color: 'blue' }",
            "blue",
            "blue",
        ),
        (
            "(p.on && { color: 'red' }) ?? { color: 'blue' }",
            "green",
            "blue",
        ),
        ("(p.on ? {} : false) ?? { color: 'blue' }", "green", "green"),
        (
            "(p.on ? {} : undefined) || { color: 'blue' }",
            "blue",
            "blue",
        ),
    ] {
        let rules = format!("const Choice = styled('div')({{ color: 'green' }}, p => {choice});");
        // When: the generated class expression runs with each props value.
        let output = compile(&rules);
        for (on, expected) in [
            ("true", "red"),
            ("false", off),
            ("0", off),
            ("''", off),
            ("null", nullish),
            ("undefined", nullish),
        ] {
            let expected = if choice.contains("? {}") && on == "true" {
                "green"
            } else {
                expected
            };
            // Then: only the selected atom is active, not every emitted branch.
            assert_eq!(
                declarations(&active(&output, &format!("{{ on: {on} }}"))),
                [("color".into(), expected.into())],
                "{choice}, {on}"
            );
        }
    }
}

#[test]
#[serial_test::serial]
fn nested_choices_keep_prior_properties_when_selected_object_omits_them() {
    // Given: recursive OR/nullish choices with a defaulted parameter.
    let rules = "const Choice = styled('div')({ color: 'green', padding: '9px' }, ({ on = true, alt }) => ((on && { color: 'red' }) || (alt ? { padding: '4px' } : null)) ?? { color: 'blue' });";
    let output = compile(rules);
    // When / Then: the whole-object choice overlays only its selected properties.
    for (props, color, padding) in [
        ("{}", "red", "9px"),
        ("{ on: false, alt: true }", "green", "4px"),
        ("{ on: false, alt: false }", "blue", "9px"),
    ] {
        assert_eq!(
            declarations(&active(&output, props)),
            [
                ("color".into(), color.into()),
                ("padding".into(), padding.into())
            ]
        );
    }
}

#[test]
#[serial_test::serial]
fn unknown_runtime_rule_shape_stays_located_when_short_circuit_chooses_it() {
    for choice in [
        "p.on || { color: 'blue' }",
        "p.rules ?? { color: 'blue' }",
        "(p.on && p.rules) || { color: 'blue' }",
    ] {
        let source = format!(
            "import {{ styled }} from '@devup-ui/react'; const Choice = styled.div(p => {choice});"
        );
        let error = crate::extract(
            "unknown-choice.tsx",
            &source,
            crate::ExtractOption::default(),
        )
        .err()
        .unwrap_or_else(|| panic!("unknown shape compiled"));
        assert!(error.to_string().contains("unknown-choice.tsx:1:"));
        assert!(error.to_string().contains("cannot use"));
    }
}

#[test]
#[serial_test::serial]
fn unreachable_rule_source_is_skipped_when_written_object_short_circuits() {
    for choice in [
        "({ color: 'red' }) || unknownRules()",
        "({ color: 'red' }) ?? unknownRules()",
        "(false && unknownRules()) || { color: 'red' }",
    ] {
        let output = compile(&format!("const Choice = styled.div(p => {choice});"));
        assert_eq!(
            declarations(&active(&output, "{}")),
            [("color".into(), "red".into())]
        );
    }
}
