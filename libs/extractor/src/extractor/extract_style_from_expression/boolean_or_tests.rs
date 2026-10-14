use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate, extracted};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("state.a === state.b || state.a", true, "[[\"a\",\"b\"],[],[]]")]
#[case(
    "state.a === state.b || state.a",
    false,
    "[[\"a\",\"b\",\"a\"],[\"margin:9px:0\"],[\"9px\"]]"
)]
#[case("state.a === state.b || '1px'", true, "[[\"a\",\"b\"],[],[]]")]
#[case(
    "state.a === state.b || '1px'",
    false,
    "[[\"a\",\"b\"],[\"margin:1px:0\"],[]]"
)]
#[case("[null, state.a === state.b || '4px']", true, "[[\"a\",\"b\"],[],[]]")]
#[case(
    "[null, state.a === state.b || '4px']",
    false,
    "[[\"a\",\"b\"],[\"margin:4px:1\"],[]]"
)]
#[case(
    "[null, state.a === state.b || state.a]",
    true,
    "[[\"a\",\"b\"],[],[]]"
)]
#[case(
    "[null, state.a === state.b || state.a]",
    false,
    "[[\"a\",\"b\",\"a\"],[\"margin:9px:1\"],[\"9px\"]]"
)]
#[serial]
fn boolean_or_selects_no_css_when_left_is_true(
    #[case] expression: &str,
    #[case] flag: bool,
    #[case] expected: &str,
) {
    // Given: the three browser failures and their dynamic responsive counterpart.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box m={{{expression}}}/>}}let trace=[];const node=render({{get a(){{trace.push('a');return '9px'}},get b(){{trace.push('b');return {flag}?'9px':'8px'}}}});"
    );
    // When: emitted class and inline projections execute jointly in Boa.
    let actual = selected(
        &source,
        "JSON.stringify([trace,assigned(node),Object.values(node.style??{})]);",
    );
    // Then: true supplies neither consumer, false supplies only the RHS at its level.
    assert_eq!(actual, expected);
}

#[rstest]
#[case("!state.flag || state.rhs", false)]
#[case("!state.flag || state.rhs", true)]
#[case("((!state.flag) as boolean) || state.rhs", false)]
#[case("((!state.flag) as boolean) || state.rhs", true)]
#[serial]
fn boolean_or_keeps_rhs_lazy_when_controller_is_negated(
    #[case] expression: &str,
    #[case] flag: bool,
) {
    // Given: an unselected RHS throws, so eager evaluation cannot pass.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box m={{{expression}}}/>}}let trace=[];const node=render({{get flag(){{trace.push('flag');return {flag}}},get rhs(){{trace.push('rhs');if(!{flag})throw Error('unselected');return '7px'}}}});"
    );
    // When: the real logical controller is evaluated exactly once.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: only a false left operand reads/selects the dynamic RHS.
    let expected = if flag {
        "[[\"flag\",\"rhs\"],[\"margin:7px:0\"]]"
    } else {
        "[[\"flag\"],[]]"
    };
    assert_eq!(actual, expected);
}

#[rstest]
#[case("'6px'", "[[\"left\"],[\"margin:6px:0\"]]")]
#[case("''", "[[\"left\",\"right\"],[\"margin:7px:0\"]]")]
#[serial]
fn value_or_retains_css_when_left_is_a_runtime_string(#[case] left: &str, #[case] expected: &str) {
    // Given: a value-producing OR, not a boolean controller.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box m={{state.left||state.right}}/>}}let trace=[];const node=render({{get left(){{trace.push('left');return {left}}},get right(){{trace.push('right');return '7px'}}}});"
    );
    // When: the source logical expression selects its actual CSS value.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: a truthy string remains CSS and each selected getter runs once.
    assert_eq!(actual, expected);
}

#[rstest]
#[case(true, "[2,0]")]
#[case(false, "[2,1]")]
#[serial]
fn static_css_boolean_or_compiles_when_rhs_is_a_literal(
    #[case] flag: bool,
    #[case] expected: &str,
) {
    // Given: css() can know every possible declaration without a boolean variable rule.
    let source = format!(
        "import {{css}}from '@devup-ui/react';function render(state){{return css({{margin:state.a===state.b||'1px'}})}}let reads=0;const className=render({{get a(){{reads++;return 'x'}},get b(){{reads++;return {flag}?'x':'y'}}}});JSON.stringify([reads,String(className??'').split(/\\s+/).filter(Boolean).length]);"
    );
    // When: extraction succeeds and its emitted class expression executes.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: no dynamic-css build error or true-branch class is introduced.
    assert_eq!(actual, expected);
}

#[rstest]
#[case("state.a === state.b || state.a", 0)]
#[case("[null, state.a === state.b || state.a]", 1)]
#[serial]
fn boolean_or_rhs_keeps_whole_source_site_when_responsive_slots_are_skipped(
    #[case] expression: &str,
    #[case] level: u8,
) {
    // Given: one authored assignment, with a skipped level before the responsive RHS.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box m={{{expression}}}/>}}"
    );
    let at = source
        .find(expression)
        .unwrap_or_else(|| panic!("fixture source"));
    // When: the remaining CSS consumer is extracted from the false branch.
    let output = extracted(&source);
    let sites = output
        .styles
        .iter()
        .filter_map(|value| match value {
            crate::ExtractStyleValue::Dynamic(style) => style
                .site
                .as_ref()
                .map(|site| (site.at, site.role, style.level())),
            _ => None,
        })
        .collect::<Vec<_>>();
    // Then: the RHS consumes the whole expression site and its original level role.
    assert_eq!(sites, vec![(at, usize::from(level), level)]);
}

#[rstest]
#[case("state.a === state.b || '1px'")]
#[case("[null, state.a === state.b || '4px']")]
#[serial]
fn boolean_or_preserves_existing_class_when_true_has_no_css(#[case] expression: &str) {
    // Given: the authored existing class must keep supplying its prior margin rule.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box className='prior exists' m={{{expression}}}/>}}const node=render({{a:'x',b:'x'}});JSON.stringify([String(node.className).split(/\\s+/).filter(Boolean),Object.values(node.style??{{}})]);"
    );
    // When: true selects the classless boolean branch.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: neither a generated margin rule nor an inline value overrides the prior class.
    assert_eq!(actual, "[[\"prior\",\"exists\"],[]]");
}
