use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::call(false)]
#[case::template(true)]
#[serial]
fn callback_discarded_unknown_spread_keeps_evaluation_before_selected_rules(
    #[case] template: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: the callback default and every authored getter are observable.
    let callback =
        "(p,unused=(trace.push('default'),0))=>({...p.input.discarded})&&{color:p.input.after}";
    let factory = if template {
        format!("styled(Base)`${{{callback}}};`")
    } else {
        format!("styled(Base)({callback})")
    };
    let source = format!(
        "const trace=[];const input={{get discarded(){{trace.push('spread');return {{get color(){{trace.push('leaf');return 'red';}}}};}},get after(){{trace.push('after');return 'blue';}}}};const Base=styled.div({{color:'base'}});const Card={factory};const defined=trace.slice();"
    );
    // When: one complete generated render selects the known right object.
    let observed = execute_in_mode(
        "callback-discarded.ts",
        &source,
        "const element=render({target:Card,props:{input}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: the discarded spread is evaluated once, but its runtime key is not CSS.
    assert_eq!(
        observed,
        serde_json::json!([[], ["default", "spread", "leaf", "after"], ["blue"]])
    );
}

#[rstest]
#[case::call(false)]
#[case::template(true)]
#[serial]
fn callback_discarded_mapped_theme_leaves_do_not_read_runtime_theme(
    #[case] template: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: the discarded object has one mapped theme leaf and one ordinary getter leaf.
    let callback = "(p,unused=(trace.push('default'),0))=>({color:p.theme.colors.primary,margin:p.input.before})&&{color:p.input.after}";
    let factory = if template {
        format!("styled(Base)`${{{callback}}};`")
    } else {
        format!("styled(Base)({callback})")
    };
    let source = format!(
        "const trace=[];const input={{get before(){{trace.push('before');return 2;}},get after(){{trace.push('after');return 'blue';}}}};const Base=styled.div({{color:'base'}});const Card={factory};const defined=trace.slice();"
    );
    // When: the real renderer evaluates the callback with no runtime theme object.
    let observed = execute_in_mode(
        "callback-discarded-theme.ts",
        &source,
        "const element=render({target:Card,props:{input}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: mapping removes the theme read while authored ordinary evaluation remains ordered.
    assert_eq!(
        observed,
        serde_json::json!([[], ["default", "before", "after"], ["blue"]])
    );
}

#[rstest]
#[case::spread_before(
    "({...p.input.rules,margin:p.input.before,color:p.theme.colors.primary})",
    &["default", "spread", "leaf", "before", "after"]
)]
#[case::spread_after(
    "({color:p.theme.colors.primary,margin:p.input.before,...p.input.rules})",
    &["default", "before", "spread", "leaf", "after"]
)]
#[case::nested(
    "({nested:{color:p.theme.colors.primary},...p.input.rules})",
    &["default", "spread", "leaf", "after"]
)]
#[case::property_function(
    "({color:({theme})=>theme.colors.primary,...p.input.rules})",
    &["default", "spread", "leaf", "after"]
)]
#[case::ordinary_functions(
    "({...p.input.rules,color:()=>trace.push('called'),method(){trace.push('method');},get value(){trace.push('getter');return 'red';},[trace.push('key')]:p.input.before})",
    &["default", "spread", "leaf", "key", "before", "after"]
)]
#[serial]
fn callback_discarded_theme_traversal_preserves_spreads_and_order(
    #[case] object: &str,
    #[case] expected_trace: &[&str],
    #[values(false, true)] template: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: no runtime theme is supplied, and every authored effect is observable.
    let callback =
        format!("(p,unused=(trace.push('default'),0))=>{object}&&{{color:p.input.after}}");
    let factory = if template {
        format!("styled(Base)`${{{callback}}};`")
    } else {
        format!("styled(Base)({callback})")
    };
    let source = format!(
        "const trace=[];const input={{get before(){{trace.push('before');return 2;}},get rules(){{trace.push('spread');return {{get color(){{trace.push('leaf');return 'red';}}}};}},get after(){{trace.push('after');return 'blue';}}}};const Base=styled.div({{color:'base'}});const Card={factory};const defined=trace.slice();"
    );
    // When: the complete renderer selects the right-hand rules exactly once.
    let observed = execute_in_mode(
        "callback-discarded-theme-traversal.ts",
        &source,
        "const element=render({target:Card,props:{input}});JSON.stringify([defined,trace,values(element,'color')]);",
        single_css,
    );
    // Then: theme mapping removes only theme leaves, without invoking discarded functions.
    assert_eq!(observed, serde_json::json!([[], expected_trace, ["blue"]]));
}

#[rstest]
#[case::property_function("({color:({theme})=>theme.dynamic(),...p.input.rules})")]
#[case::nested("({nested:{color:p.theme.dynamic()},...p.input.rules})")]
#[case::after_spread("({...p.input.rules,color:p.theme.dynamic()})")]
#[case::before_spread("({color:p.theme.dynamic(),...p.input.rules})")]
#[case::nested_after_spread("({...p.input.rules,nested:{color:p.theme.dynamic()}})")]
#[case::function_after_spread("({...p.input.rules,color:({theme})=>theme.dynamic()})")]
#[case::spread_theme("({...p.input.rules,...p.theme.colors})")]
#[case::computed_theme("({...p.input.rules,[p.theme.colors.primary]:'red'})")]
#[case::method_theme("({...p.input.rules,color({theme}){return theme.colors.primary;}})")]
#[case::nested_function("({...p.input.rules,color:()=>({nested:({theme})=>theme.dynamic()})})")]
#[serial]
fn callback_discarded_nested_or_function_theme_remains_located_error(
    #[case] object: &str,
    #[values(false, true)] template: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: unsupported fields must not hide later theme reads.
    let callback = format!("p=>{object}&&{{color:'blue'}}");
    let factory = if template {
        format!("styled.div`${{{callback}}};`")
    } else {
        format!("styled.div({callback})")
    };
    let source = format!("import {{styled}} from '@devup-ui/react';\nconst Card={factory};");
    // When: native extraction validates the evaluated discarded object.
    let result = crate::extract(
        "callback-discarded-theme-traversal-error.ts",
        &source,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    );
    // Then: the existing theme diagnostic is located at the source file.
    let error = result
        .err()
        .unwrap_or_else(|| panic!("unmappable discarded theme must be refused"))
        .to_string();
    assert!(
        error.starts_with("callback-discarded-theme-traversal-error.ts:2:"),
        "{error}"
    );
}

#[rstest]
#[case::call(false)]
#[case::template(true)]
#[serial]
fn callback_discarded_unmappable_theme_leaves_remain_located_errors(
    #[case] template: bool,
    #[values(false, true)] single_css: bool,
    #[values(
        "p=>({color:p.theme.dynamic()})&&{color:'blue'}",
        "p=>({...p.input.rules,color:p.theme.dynamic()})&&{color:'blue'}"
    )]
    callback: &str,
) {
    // Given: even an earlier unsupported spread cannot hide an unmappable theme read.
    let source = if template {
        format!(
            "import {{styled}} from '@devup-ui/react';\nconst Card=styled.div`${{{callback}}};`;"
        )
    } else {
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({callback});")
    };
    // When: native extraction attempts to admit the discarded callback.
    let result = crate::extract(
        "callback-discarded-theme-error.ts",
        &source,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    );
    // Then: the unmappable theme read remains a located build-time error.
    let error = result
        .err()
        .unwrap_or_else(|| panic!("unmappable discarded theme must be refused"))
        .to_string();
    assert!(
        error.starts_with("callback-discarded-theme-error.ts:2:"),
        "{error}"
    );
}

#[rstest]
#[case::call(false)]
#[case::template(true)]
#[serial]
fn callback_unknown_spread_shapes_remain_located_errors(
    #[case] template: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: either a selected spread or a runtime-selected spread has unknown rule keys.
    let callback = "p=>p.on?({...p.input.rules}):{color:'red'}";
    let source = if template {
        format!(
            "import {{styled}} from '@devup-ui/react';\nconst Card=styled.div`${{{callback}}};`;"
        )
    } else {
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({callback});")
    };
    // When: native extraction attempts to admit the callback's unknown rule shape.
    let result = crate::extract(
        "callback-unknown-shape.ts",
        &source,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    );
    // Then: the source remains a located build-time error in either CSS mode and form.
    let error = result
        .err()
        .unwrap_or_else(|| panic!("selected unknown callback rule shape must be refused"))
        .to_string();
    assert!(error.starts_with("callback-unknown-shape.ts:2:"), "{error}");
}
