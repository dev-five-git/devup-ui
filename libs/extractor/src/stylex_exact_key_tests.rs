mod stylex_exact_keys {
    use super::*;

    #[rstest]
    #[case::leading_zero_select(("05", "5", "styles['05']", "red"))]
    #[case::integer_select(("05", "5", "styles['5']", "blue"))]
    #[case::integer_later(("05", "5", "styles['05'], styles['5']", "blue"))]
    #[case::leading_zero_later(("05", "5", "styles['5'], styles['05']", "red"))]
    #[case::decimal_select(("0.50", "0.5", "styles['0.50']", "red"))]
    #[case::canonical_decimal_select(("0.50", "0.5", "styles['0.5']", "blue"))]
    #[case::canonical_decimal_later(("0.50", "0.5", "styles['0.50'], styles['0.5']", "blue"))]
    #[case::decimal_later(("0.50", "0.5", "styles['0.5'], styles['0.50']", "red"))]
    #[serial]
    fn applied_color_matches_exact_namespace_when_selecting_or_composing(
        #[case] sample: (&str, &str, &str, &str),
        #[values(false, true)] single_css: bool,
    ) {
        // Given: distinct authored colors under keys that CSS numeric normalization collapses.
        reset_class_map();
        reset_file_map();
        let (noncanonical, canonical, arguments, expected) = sample;
        let source = format!(
            "import stylex from '@stylexjs/stylex'; import {{jsx}} from 'react/jsx-runtime'; const styles=stylex.create({{{noncanonical:?}:{{color:'red'}},{canonical:?}:{{color:'blue'}}}}); function render() {{ return jsx('div', {{...stylex.props({arguments})}}); }}"
        );

        // When: the public extractor generates the props and the existing renderer applies them.
        let output = extract(
            "stylex-exact-key.js",
            &source,
            ExtractOption {
                single_css,
                ..ExtractOption::default()
            },
        )
        .unwrap();
        let observation = render_observation(
            &output,
            "render()",
            (!single_css).then_some("stylex-exact-key.js"),
        );

        // Then: exactly the authored winning color is selected; unused atoms cannot satisfy this.
        assert_eq!(observation, format!("[[],[{expected:?}]]"));
    }

    #[rstest]
    #[case::leading_zero(("05", "5", "05", "red"))]
    #[case::canonical_integer(("05", "5", "5", "blue"))]
    #[case::decimal(("0.50", "0.5", "0.50", "red"))]
    #[case::canonical_decimal(("0.50", "0.5", "0.5", "blue"))]
    #[serial]
    fn applied_color_preserves_exact_namespace_when_dynamic_index_runs_once(
        #[case] sample: (&str, &str, &str, &str),
        #[values(false, true)] single_css: bool,
    ) {
        // Given: an unknown key supplied only at render time through an effectful callback.
        reset_class_map();
        reset_file_map();
        let (noncanonical, canonical, key, expected) = sample;
        let source = format!(
            "import stylex from '@stylexjs/stylex'; import {{jsx}} from 'react/jsx-runtime'; const styles=stylex.create({{{noncanonical:?}:{{color:'red'}},{canonical:?}:{{color:'blue'}}}}); function render(choose) {{ return jsx('div', {{...stylex.props(styles[choose()])}}); }}"
        );

        // When: the generated dynamic-index fallback is rendered with the chosen exact key.
        let output = extract(
            "stylex-dynamic-key.js",
            &source,
            ExtractOption {
                single_css,
                ..ExtractOption::default()
            },
        )
        .unwrap();
        let observation = render_observation(
            &output,
            &format!("render(() => {{ trace.push('key'); return {key:?}; }})"),
            (!single_css).then_some("stylex-dynamic-key.js"),
        );

        // Then: the actual selected class has the authored color and the key effect occurs once.
        assert_eq!(observation, format!("[[\"key\"],[{expected:?}]]"));
    }
}
