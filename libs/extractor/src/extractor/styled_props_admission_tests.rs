use super::super::StyledBindings;
use super::*;
use oxc_ast::ast::Statement;

#[rstest::rstest]
#[case("p=>({color:p.color})", false, serde_json::json!([{"color":"red"},["color"]]))]
#[case("p=>({color:p.color})", true, serde_json::json!([{"color":"red"},["color"]]))]
#[case("p=>p.on?{color:p.color}:{color:'blue'}", true, serde_json::json!([{"color":"red"},["color"]]))]
#[case("p=>p.on?{color:p.color}:null", true, serde_json::json!([{"color":"red"},["color"]]))]
#[case("p=>!p.on?{color:p.color}:{color:'blue'}", true, serde_json::json!([{"color":"blue"},[]]))]
#[case("p=>({...{color:p.color},m:1})", false, serde_json::json!([{"color":"red","m":1},["color"]]))]
#[case("p=>({...{color:p.color},m:1})", true, serde_json::json!([{"color":"red","m":1},["color"]]))]
#[case("p=>({...({...{color:p.color}})})", true, serde_json::json!([{"color":"red"},["color"]]))]
#[case("p=>({...{color:p.color},color:p.color})", true, serde_json::json!([{"color":"red"},["color","color"]]))]
fn props_admission_when_callback_is_supported_preserves_values_and_once_reads(
    #[case] source: &str,
    #[case] bound: bool,
    #[case] expected: serde_json::Value,
) {
    // Given: parsed callbacks with real semantic bindings, not invented reference IDs.
    let arena = oxc_allocator::Allocator::default();
    let builder = AstBuilder::new(&arena);
    let parsed = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expected callback")
    };
    let mut style = statement.expression.clone_in_with_semantic_ids(&arena);
    let values = crate::style_values::StyleValues::new(std::rc::Rc::new(scoping));
    let inline_css = rustc_hash::FxHashMap::default();
    let bindings = StyledBindings {
        imports: &|_| None,
        values: &values,
        inline_css: &inline_css,
    };
    let mut evaluations = Evaluations::new(&builder, source.into());
    // When: lower the callback and execute its real invocation and resulting rule skeleton.
    let result = lower(&mut evaluations, &mut style, bound.then_some(&bindings))
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Some(Plan::Whole(invocation)) = result else {
        panic!("supported callback must have a whole invocation")
    };
    let script = format!(
        "const trace=[];const rest={{on:true,get color(){{trace.push('color');return 'red'}}}};const {}={};JSON.stringify([{},trace]);",
        invocation.name,
        readable_code(&invocation.value),
        readable_code(&style)
    );
    let observed = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("expected JSON observation"))
        .to_std_string_escaped();
    // Then: selected rules match the authored branch; only selected reads execute, once.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&observed)
            .unwrap_or_else(|error| panic!("{error}")),
        expected
    );
}

#[rstest::rstest]
#[case(
    "p=>({...{_hover:{color:(p.data.color,'discarded')}},_hover:{...{color:p.on?[p.data.color,null,'blue']:null}}})",
    3
)]
#[case("p=>({_hover:{...{color:p.on?[p.data.color,null,'blue']:null}}})", 1)]
#[serial_test::serial]
fn props_admission_when_literal_spreads_hold_responsive_choices_keeps_selected_levels(
    #[case] rules: &str,
    #[case] reads: usize,
    #[values(false, true)] single_css: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: literal spreads contain an overwritten selector and a lazy responsive winner.
    let _state = crate::test_state::TestStateGuard::new();
    let original = format!(
        "const trace=[];const props={{on:true,data:{{get color(){{trace.push('color');return 'red';}}}}}};const read=({rules});read(props);props.on=false;read(props);JSON.stringify(trace);"
    );
    let native = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&original))
        .map_err(|error| error.to_string())?
        .as_string()
        .ok_or("expected native trace")?
        .to_std_string_escaped();
    assert_eq!(native, serde_json::to_string(&vec!["color"; reads])?);
    let source =
        format!("import {{styled}} from '@devup-ui/react';const Card=styled.div({rules});");
    let option = crate::ExtractOption {
        single_css,
        import_main_css: false,
        ..Default::default()
    };
    let bucket = crate::css_bucket("spread.tsx", &option);
    let output = crate::extract("spread.tsx", &source, option)?;
    // When: actual extracted renderers select and omit that branch on consecutive renders.
    let actual = crate::styled_source_tests::captured_render::execute(
        ("spread.tsx", bucket.as_deref()),
        &output,
        "const trace=[];const props={on:true,data:{get color(){trace.push('color');return 'red';}}};const first=render({target:Card,props});props.on=false;const second=render({target:Card,props});JSON.stringify([[first,second].map(e=>[0,1,2].map(level=>values(e,'color',level,'&:hover'))),trace]);",
    );
    // Then: losing effects survive, and only selected responsive leaves contribute declarations.
    assert_eq!(
        actual,
        serde_json::json!([
            [[["red"], [], ["blue"]], [[], [], []]],
            vec!["color"; reads]
        ])
    );
    Ok(())
}

#[rstest::rstest]
#[case("p=>p.on?{color:p.color}:null", false, false)]
#[case("p=>p.theme.colors[key]", false, false)]
#[case("p=>p.theme.colors[key]", true, false)]
#[case("p=>p.on?{color:p.theme.colors[key]}:null", false, false)]
#[case("p=>p.on?{color:p.theme.colors[key]}:null", true, true)]
#[case("p=>({color:p.theme.colors[key]})", false, true)]
#[case("p=>({color:p.theme.colors[key]})", true, true)]
#[case("p=>({...p.rules})", true, false)]
#[case("p=>({_hover:{...p.rules}})", true, false)]
#[case("p=>({...{get color(){return p.color;}}})", true, false)]
#[case("p=>({...{[p.key]:p.color}})", true, false)]
#[case("p=>({...{__proto__:p.proto,color:p.color}})", true, false)]
#[case("p=>({...{'__proto__':p.proto,color:p.color}})", true, false)]
#[case("p=>({...{__proto__}})", true, false)]
#[case("p=>({...{_hover:p.on?{...p.rules}:null}})", true, false)]
#[case(
    "p=>({...{_hover:p.on?{get color(){return p.color;}}:null}})",
    true,
    false
)]
#[case("p=>({...{color:p.theme.colors[key],[p.key]:p.color}})", true, false)]
#[case("p=>({...{_hover:p.on&&{...p.rules}}})", true, false)]
#[case("p=>({...{_hover:[{...p.rules}]}})", true, false)]
fn props_admission_when_callback_is_refused_preserves_rules_before_theme_precedence(
    #[case] source: &str,
    #[case] bound: bool,
    #[case] theme: bool,
) {
    // Given: the same real semantic environment on both sides of the admission guard.
    let arena = oxc_allocator::Allocator::default();
    let builder = AstBuilder::new(&arena);
    let parsed = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expected callback")
    };
    let mut style = statement.expression.clone_in_with_semantic_ids(&arena);
    let original = readable_code(&style);
    let expected = if theme {
        let leaf = oxc_parser::Parser::new(
            &arena,
            "p=>p.theme.colors[key]",
            oxc_span::SourceType::mjs(),
        )
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
        let offset = source
            .find("p.theme")
            .unwrap_or_else(|| panic!("expected theme read"));
        (
            u32::try_from(offset).unwrap_or_else(|error| panic!("{error}")),
            unmapped_theme_error("styled", &leaf).1,
        )
    } else {
        (
            style.span().start,
            build_time_error("styled", &original, PROPS_FUNCTION),
        )
    };
    let values = crate::style_values::StyleValues::new(std::rc::Rc::new(scoping));
    let inline_css = rustc_hash::FxHashMap::default();
    let bindings = StyledBindings {
        imports: &|_| None,
        values: &values,
        inline_css: &inline_css,
    };
    let mut evaluations = Evaluations::new(&builder, source.into());
    // When: unsupported rules must refuse before a later theme diagnostic can replace them.
    let Err(error) = lower(&mut evaluations, &mut style, bound.then_some(&bindings)) else {
        panic!("callback must be refused")
    };
    // Then: exact error and location survive; refusal leaves source and captures untouched.
    assert_eq!(error, expected);
    assert_eq!(readable_code(&style), original);
    assert_eq!(evaluations.bindings.len(), 0);
}
