include!("ci_factory_alias_style_support_tests.rs");

#[rstest]
#[case::explicit(StyleShape::Explicit)]
#[case::vars(StyleShape::Vars)]
#[case::combined(StyleShape::Combined)]
#[case::computed_value(StyleShape::ComputedValue)]
#[case::computed_key(StyleShape::ComputedKey)]
#[case::computed_both(StyleShape::ComputedBoth)]
#[serial]
fn style_metadata_preserves_imported_calls_and_lexical_shadows(
    #[case] shape: StyleShape,
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
    let props = shape.props(binding);
    let source = format!(
        "{} import {{ jsx as o }} from 'react/jsx-runtime'; function render(h, {}) {{ return o(t, {{ {props} }}); }}",
        binding.imports(),
        binding.parameter()
    );
    // When
    let output = extract_fixture(&source, single_css, false);
    // Then
    for h in [false, true] {
        let mut expected_trace = shape.trace(binding);
        expected_trace.extend([
            "target:div".into(),
            format!(
                "count:{}",
                if matches!(shape, StyleShape::Combined) {
                    2
                } else {
                    1
                }
            ),
            "key:true".into(),
            "value:true".into(),
        ]);
        assert_eq!(
            observe(&output, &style_invocation(shape, binding, h), single_css),
            (expected_trace, vec![]),
            "{shape:?}/{binding:?}/h={h}\n{}",
            output.code
        );
        if binding.shadow() {
            assert_eq!(output.styles.len(), 0);
        } else {
            if shape.computed_key() {
                let (_, declarations) = observe(
                    &output,
                    &held_projection(shape, binding, h, true),
                    single_css,
                );
                assert_eq!(declarations.len(), 1);
                assert_eq!(declarations[0].parse::<f64>().unwrap(), 0.5);
            }
            if !matches!(shape, StyleShape::ComputedKey) {
                let value = if matches!(shape, StyleShape::ComputedBoth) {
                    format!(
                        "(() => {{ const result = render({h}, void 0); return {{props: {{className: Object.values(result.props.style)[0]}}}}; }})()"
                    )
                } else {
                    held_projection(shape, binding, h, false)
                };
                assert_eq!(observe(&output, &value, single_css).1, ["hidden"]);
            }
            if matches!(shape, StyleShape::Combined) {
                let invocation = format!(
                    "(() => {{ const result = render({h}, void 0); return {{props: {{className: result.props.style['--explicit-key']}}}}; }})()"
                );
                let (_, declarations) = observe(&output, &invocation, single_css);
                assert_eq!(declarations.len(), 1);
                assert_eq!(declarations[0].parse::<f64>().unwrap(), 0.5);
            }
        }
    }
}
