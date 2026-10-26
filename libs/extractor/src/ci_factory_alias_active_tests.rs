use super::*;

#[rstest]
#[case::direct("className: OWN, css: { color: 'red' }")]
#[case::template("className: `external ${OWN}`, css: { color: 'red' }")]
#[serial]
fn active_native_css_prop_merges_class_aliases_with_emitted_css(
    #[case] fragment: &str,
    #[values(Binding::Alias, Binding::Namespace, Binding::ShadowAlias, Binding::ShadowNamespace)] binding: Binding,
    #[values(false, true)] single_css: bool,
) {
    // Given: pragma and alias jointly enable css-prop lowering (v2 evidence).
    let _state = crate::test_state::TestStateGuard::new();
    let props = fragment.replace("OWN", binding.call());
    let source = format!("/** @jsxImportSource @emotion/react */\n{} import {{ jsx as o }} from 'react/jsx-runtime'; function render({}) {{ return o('div', {{ {props} }}); }}", binding.imports(), binding.parameter());
    // When
    let output = extract_fixture(&source, single_css, true);
    // Then: red belongs to css-prop; hidden must never come from a lexical shadow.
    assert_own_styles(&output, binding);
    let invocation = format!("(() => {{ const result = render({}); const tokens = (result.props.className || '').split(/\\s+/); trace.push('css:' + ('css' in result.props), 'external:' + tokens.includes('external'), 'local:' + tokens.includes('local')); return result; }})()", binding.argument(true));
    let mut trace = Vec::new();
    if binding.shadow() { trace.push("own:hidden".to_string()); }
    trace.extend(["css:false".to_string(), format!("external:{}", fragment.contains("external")), format!("local:{}", binding.shadow())]);
    let expected = if binding.shadow() { vec!["red".to_string()] } else { vec!["hidden".to_string(), "red".to_string()] };
    assert_eq!(observe(&output, &invocation, single_css), (trace, expected));
}
