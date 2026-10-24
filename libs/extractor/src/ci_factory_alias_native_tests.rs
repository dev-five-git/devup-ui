include!("ci_factory_alias_support_tests.rs");

mod spreads {
    include!("ci_factory_alias_spread_tests.rs");
}
mod active_css {
    include!("ci_factory_alias_active_tests.rs");
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Direct,
    DirectMerged,
    Conditional,
    Template,
    NestedTemplate,
    TailwindConditional,
    TailwindTest,
    Logical,
    Call,
    Array,
    Object,
    TailwindTemplate,
    StyleOrder,
}

impl Shape {
    fn props(self) -> &'static str {
        match self {
            Self::Direct => "className: OWN",
            Self::DirectMerged => "className: OWN, color: 'red'",
            Self::Conditional => "className: h ? OWN : void 0",
            Self::Template => "className: `external ${OWN}`",
            Self::NestedTemplate => "className: `external ${h ? OWN : ''}`",
            Self::TailwindConditional => "className: h ? 'flex' : OWN",
            Self::TailwindTest => "className: OWN ? 'flex' : 'external'",
            Self::Logical => "className: OWN || 'flex'",
            Self::Call => "className: clsx('flex', OWN)",
            Self::Array => "className: ['flex', OWN].join(' ')",
            Self::Object => "className: clsx({ 'flex': OWN })",
            Self::TailwindTemplate => "className: `flex ${OWN}`",
            Self::StyleOrder => "className: OWN, color: 'red', styleOrder: h ? 1 : 2",
        }
    }
}

#[rstest]
#[case::direct(Shape::Direct)]
#[case::direct_merged(Shape::DirectMerged)]
#[case::conditional(Shape::Conditional)]
#[case::template(Shape::Template)]
#[case::nested_template(Shape::NestedTemplate)]
#[case::tailwind_conditional(Shape::TailwindConditional)]
#[case::tailwind_test(Shape::TailwindTest)]
#[case::logical(Shape::Logical)]
#[case::call(Shape::Call)]
#[case::array(Shape::Array)]
#[case::object(Shape::Object)]
#[case::tailwind_template(Shape::TailwindTemplate)]
#[case::style_order(Shape::StyleOrder)]
#[serial]
fn class_aliases_select_emitted_styles_when_preprocessing_retains_bindings(
    #[case] shape: Shape,
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
    let props = shape.props().replace("OWN", binding.call());
    let source = format!(
        "{} import {{ jsx as o }} from 'react/jsx-runtime'; function render(h, {}, clsx) {{ return o(t, {{ {props} }}); }}",
        binding.imports(),
        binding.parameter()
    );
    // When
    let output = extract_fixture(&source, single_css, false);
    // Then: selected classes, not merely generated-but-unused rules, decide styles.
    assert_own_styles(&output, binding);
    for h in [false, true] {
        let truthy = !binding.shadow() || h;
        let called = match shape {
            Shape::Conditional | Shape::NestedTemplate => h,
            Shape::TailwindConditional => !h,
            _ => true,
        };
        let selected = match shape {
            Shape::TailwindTest | Shape::Object => false,
            _ => called,
        };
        let flex = match shape {
            Shape::TailwindConditional => h,
            Shape::TailwindTest | Shape::Object => truthy,
            Shape::Logical => !truthy,
            Shape::Call | Shape::Array | Shape::TailwindTemplate => true,
            _ => false,
        };
        let external = matches!(shape, Shape::Template | Shape::NestedTemplate)
            || (matches!(shape, Shape::TailwindTest) && !truthy);
        let local = binding.shadow() && h && selected;
        let invocation = format!(
            "(() => {{ const result = render({h}, {}, {CLSX}); const tokens = (result.props.className || '').split(/\\s+/); trace.push('external:' + tokens.includes('external'), 'local:' + tokens.includes('local')); return result; }})()",
            binding.argument(h)
        );
        let mut expected_trace = Vec::new();
        if binding.shadow() && called {
            expected_trace.push("own:hidden".to_string());
        }
        if matches!(shape, Shape::Call | Shape::Object) {
            expected_trace.push("clsx".to_string());
        }
        expected_trace.extend([format!("external:{external}"), format!("local:{local}")]);
        let mut expected = Vec::new();
        if selected && !binding.shadow() {
            expected.push("hidden".to_string());
        }
        if flex {
            expected.push("flex".to_string());
        }
        if matches!(shape, Shape::DirectMerged | Shape::StyleOrder) {
            expected.push("red".to_string());
        }
        expected.sort();
        assert_eq!(
            observe(&output, &invocation, single_css),
            (expected_trace, expected),
            "{shape:?}/{binding:?}/h={h}\n{}",
            output.code
        );
    }
}

mod style_metadata {
    include!("ci_factory_alias_style_tests.rs");
}
