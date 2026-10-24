use crate::extract_style::ExtractStyleProperty;
use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, extract};

#[path = "props_rule_choice_regressions.rs"]
mod regressions;

#[path = "styled_choice_test_support.rs"]
pub(super) mod support;

fn selected_classes(output: &ExtractOutput, on: bool) -> Vec<String> {
    let mut classes: Vec<_> = crate::styled_source_tests::captured_render::active(
        ("props-rule-choices.tsx", Some("props-rule-choices.tsx")),
        output,
        &format!("{{on:{on},width:'24px'}}"),
    )
    .iter()
    .map(|style| {
        style
            .extract(Some("props-rule-choices.tsx"))
            .unwrap_or_else(|| panic!("selected declaration class identity"))
            .to_string()
    })
    .collect();
    classes.sort();
    classes
}

fn class_for(output: &ExtractOutput, property: &str, value: &str) -> String {
    let style = output
        .styles
        .iter()
        .find_map(|style| match style {
            ExtractStyleValue::Static(style)
                if style.property() == property && style.value() == value =>
            {
                Some(style)
            }
            ExtractStyleValue::Static(_)
            | ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Dynamic(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_)
            | ExtractStyleValue::LayerOrder(_)
            | ExtractStyleValue::Keyframes(_) => None,
        })
        .unwrap_or_else(|| panic!("missing {property}: {value}"));
    style.extract(Some("props-rule-choices.tsx")).to_string()
}

fn compile(rules: &str) -> ExtractOutput {
    let source =
        format!("import {{ styled }} from '@devup-ui/react'; {rules} export {{ Choice }};");
    extract("props-rule-choices.tsx", &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"))
}

fn values(output: &ExtractOutput, property: &str) -> Vec<String> {
    let mut values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == property => {
                Some(style.value().to_string())
            }
            _ => None,
        })
        .collect();
    values.sort();
    values
}

#[test]
#[serial_test::serial]
fn base_atom_survives_when_logical_branch_is_missing() {
    let rules = "const Base = styled('div')({ color: 'red' }); const Choice = styled(Base)(p => p.on && { color: 'blue' });";
    let output = compile(rules);
    assert_eq!(values(&output, "color"), ["blue", "red"]);
    assert!(output.code.contains("p.on"));
    assert!(!output.code.contains("styled("));
    for (on, color) in [(true, "blue"), (false, "red")] {
        assert_eq!(
            selected_classes(&output, on),
            [class_for(&output, "color", color)]
        );
    }
    assert!(output.code.contains("style={style}"));
}

#[test]
#[serial_test::serial]
fn distinct_keys_keep_base_when_ternary_branch_omits_them() {
    let rules = "const Choice = styled('div')({ color: 'red', backgroundColor: 'black' }, p => p.on ? { color: 'blue' } : { backgroundColor: 'white' });";
    let output = compile(rules);
    assert_eq!(values(&output, "color"), ["blue", "red"]);
    assert_eq!(values(&output, "background-color"), ["black", "white"]);
    for (on, color, background) in [(true, "blue", "black"), (false, "red", "white")] {
        let mut expected = [
            class_for(&output, "color", color),
            class_for(&output, "background-color", background),
        ];
        expected.sort();
        assert_eq!(selected_classes(&output, on), expected);
    }
    assert!(output.code.contains("style={style}"));
}

#[test]
#[serial_test::serial]
fn nested_selector_keeps_base_when_logical_branch_is_missing() {
    let rules = "const Base = styled('div')({ '&:hover': { color: 'red', opacity: 0.5 } }); const Choice = styled(Base)(p => p.on && { '&:hover': { color: 'blue' } });";
    let output = compile(rules);
    assert_eq!(values(&output, "color"), ["blue", "red"]);
    assert_eq!(values(&output, "opacity"), [".5"]);
    for (on, color) in [(true, "blue"), (false, "red")] {
        let mut expected = [
            class_for(&output, "color", color),
            class_for(&output, "opacity", ".5"),
        ];
        expected.sort();
        assert_eq!(selected_classes(&output, on), expected);
    }
    assert!(output.styles.iter().all(|value| match value {
        ExtractStyleValue::Static(style) => style.selector().is_some(),
        _ => false,
    }));
}

#[test]
#[serial_test::serial]
fn dynamic_leaf_uses_variable_when_rule_branch_is_selected() {
    let rules = "const Base = styled('div')({ width: '12px' }); const Choice = styled(Base)(p => p.on && { width: p.width });";
    let output = compile(rules);
    assert_eq!(values(&output, "width"), ["12px"]);
    assert!(output.styles.iter().any(
        |style| matches!(style, ExtractStyleValue::Dynamic(style) if style.property() == "width")
    ));
    assert!(output.code.contains("p.width"));
    assert!(output.code.contains("p.on"));
    assert!(output.code.contains("--"));
    assert!(!output.code.contains("styled("));
    assert_eq!(
        selected_classes(&output, false),
        [class_for(&output, "width", "12px")]
    );
    let observed = crate::styled_source_tests::captured_render::execute(
        ("props-rule-choices.tsx", Some("props-rule-choices.tsx")),
        &output,
        "const element=render({target:Choice,props:{on:true,width:'24px'}});JSON.stringify(values(element,'width'));",
    );
    assert_eq!(observed, serde_json::json!(["24px"]));
}

#[test]
#[serial_test::serial]
fn missing_branches_keep_base_when_ternary_returns_no_rules() {
    for absent in ["null", "false", "undefined", "{}"] {
        let rules = format!(
            "const Choice = styled('div')({{ color: 'red' }}, p => p.on ? {{ color: 'blue' }} : {absent});"
        );
        let output = compile(&rules);
        for (on, color) in [(true, "blue"), (false, "red")] {
            assert_eq!(
                selected_classes(&output, on),
                [class_for(&output, "color", color)]
            );
        }
    }
}

#[test]
#[serial_test::serial]
fn later_atom_wins_when_it_follows_conditional_rules() {
    let rules = "const Choice = styled('div')({ color: 'red' }, p => p.on && { color: 'blue' }, { color: 'green' });";
    let output = compile(rules);
    assert_eq!(values(&output, "color"), ["green"]);
    for on in [true, false] {
        assert_eq!(
            selected_classes(&output, on),
            [class_for(&output, "color", "green")]
        );
    }
}

#[test]
#[serial_test::serial]
fn theme_leaf_is_static_when_conditional_rules_read_theme() {
    let rules = "const Choice = styled('div')(p => p.on ? { color: p.theme.colors.primary } : { color: 'red' });";
    let output = compile(rules);
    assert_eq!(values(&output, "color"), ["red", "var(--colors-primary)"]);
    assert!(
        !output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
    );
}

#[test]
#[serial_test::serial]
fn pure_comparison_is_supported_when_rule_choice_reads_props() {
    let rules = "const Choice = styled('div')(({ on, count }) => on && count > 2 ? { color: 'blue' } : { color: 'red' });";
    let output = compile(rules);
    assert_eq!(values(&output, "color"), ["blue", "red"]);
    assert!(output.code.contains("count > 2"));
}

#[test]
#[serial_test::serial]
fn unknown_shapes_are_rejected_when_function_cannot_name_rules() {
    for rules in [
        "p => p.on ? p.styles : { color: 'red' }",
        "p => p.on && { ...p.styles }",
        "p => p.on && { [p.key]: 'red' }",
        "p => check(p) ? { color: 'blue' } : { color: 'red' }",
    ] {
        let source = format!(
            "import {{ styled }} from '@devup-ui/react'; const Choice = styled('div')({rules});"
        );
        let Err(error) = extract("unknown-rule.tsx", &source, ExtractOption::default()) else {
            panic!("unknowable rules must fail at build time");
        };
        assert!(error.to_string().contains("unknown-rule.tsx:"));
        assert!(error.to_string().contains("cannot use"));
    }
}

#[test]
#[serial_test::serial]
fn invalid_condition_prop_is_withheld_when_rules_are_lowered() {
    let rules = "const Choice = styled('div')(p => p.on && { color: p.title });";
    let output = compile(rules);
    assert!(
        output.code.contains("\"on\": __devupOmit"),
        "{}",
        output.code
    );
    assert!(!output.code.contains("\"title\": __devupOmit"));
    assert!(output.code.contains("p.title"));
}

#[test]
#[serial_test::serial]
fn invalid_value_prop_is_withheld_when_whole_rule_function_is_lowered() {
    let rules = "const Choice = styled('div')(p => ({ color: p.tone, opacity: p.tabIndex, cursor: p.onClick }));";
    let output = compile(rules);
    assert!(output.code.contains("\"tone\": __devupOmit"));
    assert!(!output.code.contains("\"tabIndex\": __devupOmit"));
    assert!(!output.code.contains("\"onClick\": __devupOmit"));
    assert!(output.code.contains("p.tone"));
}

#[test]
#[serial_test::serial]
fn inherited_reads_keep_condition_prop_when_extension_is_used() {
    let rules = "const Base = styled('div')(p => p.on ? { color: 'blue' } : { color: 'red' }); const Choice = styled(Base)({ opacity: 0.5 }); export const Example = <Choice on={enabled} title='valid' />;";
    let output = compile(rules);
    assert_eq!(
        output.code.matches("\"on\": __devupOmit").count(),
        2,
        "{}",
        output.code
    );
    assert!(output.code.contains("on={enabled}"));
    assert!(!output.code.contains("\"title\": __devupOmit"));
}

#[test]
#[serial_test::serial]
fn condition_flag_is_forwarded_when_explicit_policy_allows_it() {
    let rules = "const Choice = styled('div').withConfig({ shouldForwardProp: name => name === 'on' })(p => p.on && { color: 'blue' });";
    let output = compile(rules);
    assert!(!output.code.contains("\"on\": __devupOmit"));
}

#[test]
#[serial_test::serial]
fn condition_flag_is_forwarded_when_styled_base_is_custom_component() {
    let rules = "const Custom = props => <div />; const Choice = styled(Custom)(p => p.on && { color: 'blue' });";
    let output = compile(rules);
    assert!(!output.code.contains("\"on\": __devupOmit"));
    assert!(output.code.contains("DevupAs = Custom"));
}
