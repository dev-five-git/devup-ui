mod stylex_exact_keys {
    use super::*;
    include!("stylex_capture_tests.rs");
    include!("stylex_eval_tests.rs");
    include!("stylex_no_storage_tests.rs");

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

    #[rstest]
    #[case::logical("css && s.base")]
    #[case::ternary("css ? s.base : s.later")]
    #[case::copied_previous("css && s.base, on && s.later")]
    #[case::equal("css ? s.base : s.base")]
    #[case::empty("css ? null : null")]
    #[case::overridden("css && s.later, s.later")]
    #[serial]
    fn compiled_condition_read_is_located_when_stylex_keeps_the_condition(#[case] arguments: &str) {
        // Given: css is the imported compile-time binding, not an ordinary shadow.
        reset_class_map();
        reset_file_map();
        let source = format!(
            "import {{css}} from '@devup-ui/react';\nimport * as sx from '@stylexjs/stylex';\nconst s=sx.create({{base:{{color:'red',backgroundColor:'blue'}},later:{{color:'green'}}}});\nexport const output=sx.props({arguments});"
        );

        // When: the public native extractor lowers a retained condition.
        let error = extract("stylex-condition.js", &source, ExtractOption::default())
            .expect_err("a compiled binding cannot survive as a runtime value")
            .to_string();

        // Then: lexical diagnostics point at the authored read.
        assert!(error.starts_with("stylex-condition.js:4:"), "{error}");
        assert!(error.contains("`css` is read at runtime"), "{error}");
    }

    #[rstest]
    #[case::logical("css && s.base")]
    #[case::ternary("css ? s.base : s.empty")]
    #[case::copied_previous("css && s.base, false && s.later")]
    #[serial]
    fn local_condition_stays_ordinary_when_it_shadows_the_compiled_css_import(
        #[case] arguments: &str,
    ) {
        // Given: the parameter shadows the imported compile-time API.
        reset_class_map();
        reset_file_map();
        let source = format!(
            "import {{css}} from '@devup-ui/react'; import * as sx from '@stylexjs/stylex'; import {{jsx}} from 'react/jsx-runtime'; const s=sx.create({{base:{{color:'red',backgroundColor:'blue'}},later:{{color:'green'}},empty:{{}}}}); function render(css) {{ return jsx('div', {{...sx.props({arguments})}}); }}"
        );

        // When: real extracted output executes with the ordinary local value.
        let output = extract("stylex-shadow.js", &source, ExtractOption::default()).unwrap();
        let observation = render_observation(&output, "render(true)", Some("stylex-shadow.js"));
        let (trace, mut values): (Vec<String>, Vec<String>) =
            serde_json::from_str(&observation).unwrap();
        values.sort();

        // Then: no spelling-based diagnostic rejects the shadow, and both keys survive.
        assert_eq!((trace, values), (vec![], vec!["blue".into(), "red".into()]));
    }

    #[rstest]
    #[case::multi_key(("", "(trace.push('once'), true) && s.base", "[\"once\"]", "[\"blue\",\"red\"]"))]
    #[case::two_conditions(("", "(trace.push('first'), true) && s.base, (trace.push('second'), true) && s.later", "[\"first\",\"second\"]", "[\"blue\",\"green\"]"))]
    #[case::same_key_order(("", "(trace.push('first'), true) && s.later, (trace.push('second'), true) && s.later", "[\"first\",\"second\"]", "[\"green\"]"))]
    #[case::full_override(("", "(trace.push('overridden'), true) && s.base, s.full", "[\"overridden\"]", "[\"black\",\"green\"]"))]
    #[case::empty_selected(("", "(trace.push('empty'), true) && s.empty", "[\"empty\"]", "[]"))]
    #[case::empty_sides(("", "(trace.push('empty'), true) ? s.empty : s.empty", "[\"empty\"]", "[]"))]
    #[case::equal_sides(("", "(trace.push('equal'), true) ? s.base : s.base", "[\"equal\"]", "[\"blue\",\"red\"]"))]
    #[case::getter(("const state={get on(){trace.push('get');return true;}};", "state.on && s.base", "[\"get\"]", "[\"blue\",\"red\"]"))]
    #[case::mutation(("let n=0;", "(trace.push(String(++n)), n === 1) && s.base, (trace.push('second'), false) && s.later", "[\"1\",\"second\"]", "[\"blue\",\"red\"]"))]
    #[case::falsy(("", "(trace.push('false'), false) && s.base", "[\"false\"]", "[]"))]
    #[case::null_reset(("", "s.base, (trace.push('clear'), true) && s.clear", "[\"clear\"]", "[]"))]
    #[serial]
    fn authored_effects_run_once_in_order_when_stylex_merges_keys(
        #[case] sample: (&str, &str, &str, &str),
        #[values("props", "attrs")] api: &str,
    ) {
        // Given: conditions with effects, including styles with no surviving selected key.
        reset_class_map();
        reset_file_map();
        let (setup, arguments, expected_trace, expected_values) = sample;
        let attribute = if api == "props" { "className" } else { "class" };
        let source = format!(
            "import * as sx from '@stylexjs/stylex'; import {{jsx}} from 'react/jsx-runtime'; const s=sx.create({{base:{{color:'red',backgroundColor:'blue'}},later:{{color:'green'}},full:{{color:'green',backgroundColor:'black'}},empty:{{}},clear:{{color:null,backgroundColor:null}}}}); function render() {{ {setup} const out=sx.{api}({arguments}); return jsx('div', {{className:out.{attribute}}}); }}"
        );

        // When: the existing native Boa renderer executes the extracted call.
        let output = extract("stylex-effects.js", &source, ExtractOption::default()).unwrap();
        let observation = render_observation(&output, "render()", Some("stylex-effects.js"));
        let (trace, mut values): (Vec<String>, Vec<String>) =
            serde_json::from_str(&observation).unwrap();
        values.sort();
        let expected_trace: Vec<String> = serde_json::from_str(expected_trace).unwrap();
        let expected_values: Vec<String> = serde_json::from_str(expected_values).unwrap();

        // Then: argument-order effects and only the winning authored declarations are observed.
        assert_eq!(
            (trace, values),
            (expected_trace, expected_values),
            "{}",
            output.code
        );
    }

    #[rstest]
    #[case::inherited(("s.base,", "[\"red\",\"white\"]", "[\"black\",\"blue\"]"))]
    #[case::absent(("", "[\"red\"]", "[\"blue\"]"))]
    #[serial]
    fn alternate_only_key_preserves_winners_when_conditional_namespaces_are_disjoint(
        #[case] sample: (&str, &str, &str),
        #[values("props", "attrs")] api: &str,
        #[values(false, true)] on: bool,
    ) {
        // Given: each branch replaces a different key, with or without earlier defaults.
        reset_class_map();
        reset_file_map();
        let attribute = if api == "props" { "className" } else { "class" };
        let source = format!(
            "import * as sx from '@stylexjs/stylex'; import {{jsx}} from 'react/jsx-runtime'; const s=sx.create({{base:{{color:'black',backgroundColor:'white'}},left:{{color:'red'}},right:{{backgroundColor:'blue'}}}}); function tick(label,value){{trace.push(label);return value;}} function render(on){{const out=sx.{api}({}tick('branch',on)?s.left:s.right);return jsx('div',{{className:out.{attribute}}});}}",
            sample.0
        );

        // When: the public extractor's output executes the selected branch in Boa.
        let output = extract("stylex-disjoint.js", &source, ExtractOption::default()).unwrap();
        let observation = render_observation(
            &output,
            &format!("render({on})"),
            Some("stylex-disjoint.js"),
        );
        let (trace, mut values): (Vec<String>, Vec<String>) =
            serde_json::from_str(&observation).unwrap();
        values.sort();

        // Then: the missing side inherits only its prior key; the branch test runs once.
        let expected = if on { sample.1 } else { sample.2 };
        assert_eq!(trace, ["branch"], "{}", output.code);
        assert_eq!(
            values,
            serde_json::from_str::<Vec<String>>(expected).unwrap(),
            "{}",
            output.code
        );
        assert_eq!(callable_count(&output.code), callable_count(&source));
    }

    #[test]
    #[serial]
    fn throwing_condition_still_throws_when_all_its_keys_are_overridden() {
        // Given: an overridden argument whose getter must stop later argument evaluation.
        reset_class_map();
        reset_file_map();
        let source = "import * as sx from '@stylexjs/stylex'; import {jsx} from 'react/jsx-runtime'; const s=sx.create({base:{color:'red'},later:{color:'green'}}); function render() { const state={get on(){trace.push('get');throw new Error('stop');}}; try { sx.props(state.on && s.base, (trace.push('later'), true) ? s.later : s.later); } catch(error) { trace.push(error.message); } return jsx('div', {}); }";

        // When: real extracted output executes the call inside its authored catch boundary.
        let output = extract("stylex-throw.js", source, ExtractOption::default()).unwrap();
        let observation = render_observation(&output, "render()", Some("stylex-throw.js"));

        // Then: neither dropping the first getter nor evaluating the later condition is valid.
        assert_eq!(observation, "[[\"get\",\"stop\"],[]]", "{}", output.code);
    }
}
