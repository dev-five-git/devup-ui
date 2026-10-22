use super::*;

#[rstest]
#[case::effectful_after_alias(("className: OWN, ...getProps()", false, true), Binding::Alias)]
#[case::effectful_after_namespace(("className: OWN, ...getProps()", false, true), Binding::Namespace)]
#[case::identifier_after_alias(("className: OWN, ...p", false, false), Binding::Alias)]
#[case::identifier_after_namespace(("className: OWN, ...p", false, false), Binding::Namespace)]
#[case::effectful_before_alias(("...getProps(), className: OWN", true, true), Binding::Alias)]
#[case::effectful_before_namespace(("...getProps(), className: OWN", true, true), Binding::Namespace)]
#[case::effectful_before_shadow_alias(("...getProps(), className: OWN", true, true), Binding::ShadowAlias)]
#[case::effectful_before_shadow_namespace(("...getProps(), className: OWN", true, true), Binding::ShadowNamespace)]
#[case::literal_alias(("...{ className: OWN }", true, false), Binding::Alias)]
#[case::literal_namespace(("...{ className: OWN }", true, false), Binding::Namespace)]
#[case::literal_shadow_alias(("...{ className: OWN }", true, false), Binding::ShadowAlias)]
#[case::literal_shadow_namespace(("...{ className: OWN }", true, false), Binding::ShadowNamespace)]
#[serial]
fn spread_aliases_preserve_class_presence_and_once_only_evaluation(
    #[case] sample: (&str, bool, bool),
    #[case] binding: Binding,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let (fragment, explicit_wins, effectful) = sample;
    let props = fragment.replace("OWN", binding.call());
    let source = format!(
        "{} import {{ jsx as o }} from 'react/jsx-runtime'; function render(p, getProps, {}) {{ return o(t, {{ {props} }}); }}",
        binding.imports(),
        binding.parameter()
    );
    // When
    let output = extract_fixture(&source, single_css, false);
    // Then: an explicit undefined class differs from a missing class.
    assert_own_styles(&output, binding);
    for (spread, present, external) in [
        ("{}", false, false),
        ("{className:'external'}", true, true),
        ("{className:void 0}", true, false),
    ] {
        let own_selected = explicit_wins || !present;
        let helper_first = fragment.starts_with("className");
        let invocation = format!(
            "(() => {{ const result = render({spread}, () => {{ trace.push('spread'); return {spread}; }}, {}); const tokens = (result.props.className || '').split(/\\s+/); trace.push('external:' + tokens.includes('external'), 'local:' + tokens.includes('local')); return result; }})()",
            binding.argument(true)
        );
        let mut expected_trace = Vec::new();
        if binding.shadow() && helper_first {
            expected_trace.push("own:hidden".to_string());
        }
        if effectful {
            expected_trace.push("spread".to_string());
        }
        if binding.shadow() && !helper_first {
            expected_trace.push("own:hidden".to_string());
        }
        expected_trace.extend([
            format!("external:{}", !explicit_wins && external),
            format!("local:{}", binding.shadow() && own_selected),
        ]);
        let expected = if own_selected && !binding.shadow() {
            vec!["hidden".to_string()]
        } else {
            vec![]
        };
        assert_eq!(
            observe(&output, &invocation, single_css),
            (expected_trace, expected),
            "{fragment}/{binding:?}/{spread}\n{}",
            output.code
        );
    }
}

#[rstest]
#[case::native("props: { className: OWN }")]
#[case::custom("as: Target, props: { className: OWN }")]
#[serial]
fn passthrough_class_aliases_retain_imports_without_compiling_local_helpers(
    #[case] fragment: &str,
    #[values(
        Binding::Alias,
        Binding::Namespace,
        Binding::ShadowAlias,
        Binding::ShadowNamespace
    )]
    binding: Binding,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let props = fragment.replace("OWN", binding.call());
    let source = format!(
        "{} import {{ jsx as o }} from 'react/jsx-runtime'; function render(Target, {}) {{ return o(t, {{ {props} }}); }}",
        binding.imports(),
        binding.parameter()
    );
    // When
    let output = extract_fixture(&source, single_css, false);
    // Then
    assert_own_styles(&output, binding);
    let invocation = format!(
        "(() => {{ const result = render('article', {}); trace.push('target:' + result.target, 'local:' + (result.props.className || '').split(/\\s+/).includes('local')); return result; }})()",
        binding.argument(true)
    );
    let mut trace = Vec::new();
    if binding.shadow() {
        trace.push("own:hidden".to_string());
    }
    trace.push(format!(
        "target:{}",
        if fragment.starts_with("as:") {
            "article"
        } else {
            "div"
        }
    ));
    trace.push(format!("local:{}", binding.shadow()));
    let expected = if binding.shadow() {
        vec![]
    } else {
        vec!["hidden".to_string()]
    };
    assert_eq!(observe(&output, &invocation, single_css), (trace, expected));
}

#[rstest]
#[serial]
fn spread_argument_aliases_are_moved_before_factory_body_rewriting(
    #[values(false, true)] single_css: bool,
    #[values(
        Binding::Alias,
        Binding::Namespace,
        Binding::ShadowAlias,
        Binding::ShadowNamespace
    )]
    binding: Binding,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "{} import {{ jsx as o }} from 'react/jsx-runtime'; function render(getProps, {}) {{ return o(t, {{ ...getProps({}) }}); }}",
        binding.imports(),
        binding.parameter(),
        binding.call()
    );
    // When
    let output = extract_fixture(&source, single_css, false);
    // Then: getProps consumes the actual compiled class, not a copied expectation.
    assert_own_styles(&output, binding);
    let invocation = format!(
        "render(className => {{ trace.push('spread'); return {{className}}; }}, {})",
        binding.argument(true)
    );
    let trace = if binding.shadow() {
        vec!["own:hidden".to_string(), "spread".to_string()]
    } else {
        vec!["spread".to_string()]
    };
    let expected = if binding.shadow() {
        vec![]
    } else {
        vec!["hidden".to_string()]
    };
    assert_eq!(observe(&output, &invocation, single_css), (trace, expected));
}
