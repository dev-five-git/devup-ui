use super::*;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[rstest::rstest]
#[case::factory_fallback(true)]
#[case::authored_leaf(false)]
#[serial_test::serial]
fn factory_params_use_the_call_location_when_the_leaf_has_no_offset(#[case] clear_offset: bool) {
    // Given a factory call with either a generated zero-span or an authored empty params list.
    let _state = crate::test_state::TestStateGuard::new();
    let arena = Allocator::default();
    let source = "import {Box} from '@devup-ui/react';import {jsx} from 'react/jsx-runtime';\njsx(Box,{_nthChild:{params:[],color:'red'}});";
    let mut parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
    let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.last_mut() else {
        panic!("expected factory statement");
    };
    let Expression::CallExpression(call) = &mut statement.expression else {
        panic!("expected factory call");
    };
    let call_offset = call.span.start;
    let Expression::ObjectExpression(props) = call.arguments[1].to_expression_mut() else {
        panic!("expected props");
    };
    let ObjectPropertyKind::ObjectProperty(selector) = &mut props.properties[0] else {
        panic!("expected selector");
    };
    let Expression::ObjectExpression(rules) = &mut selector.value else {
        panic!("expected selector rules");
    };
    let ObjectPropertyKind::ObjectProperty(params) = &mut rules.properties[0] else {
        panic!("expected params");
    };
    let Expression::ArrayExpression(array) = &mut params.value else {
        panic!("expected params array");
    };
    let leaf_offset = array.span.start;
    if clear_offset {
        array.span = oxc_span::SPAN;
    }
    let mut visitor = DevupVisitor::new(&arena, "factory.tsx", "@devup-ui/react", Vec::new(), None);
    // When the actual factory visitor extracts its structural params.
    visitor.visit_program(&mut parsed.program);
    // Then zero-span errors use the call, while authored leaves keep their own location.
    assert_eq!(
        visitor.errors,
        vec![(
            if clear_offset {
                call_offset
            } else {
                leaf_offset
            },
            element_error(
                "Box",
                "[]",
                crate::extractor::selector_params::SELECTOR_PARAMS
            ),
        )]
    );
}

#[rstest::rstest]
#[case::devup_base("const Base = styled(Box)({color:'red'});", true, false)]
#[case::native_base("const Base = styled.div({color:'red'});", false, false)]
#[case::ordinary_base("function Base(props) { return null; }", false, true)]
#[serial_test::serial]
fn styled_extensions_inherit_only_a_style_capable_base(
    #[case] base: &str,
    #[case] style_capable: bool,
    #[case] keeps_padding_prop: bool,
) {
    let _state = crate::test_state::TestStateGuard::new();
    let arena = Allocator::default();
    let source = format!(
        "import {{Box,styled}} from '@devup-ui/react';{base}const Choice=styled(Base)({{opacity:0.5}});const view=<Choice p={{2}} title='kept'/>;"
    );
    let mut parsed = Parser::new(&arena, &source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let mut visitor =
        DevupVisitor::new(&arena, "extension.tsx", "@devup-ui/react", Vec::new(), None);

    visitor.visit_program(&mut parsed.program);

    assert!(visitor.errors.is_empty(), "{:?}", visitor.errors);
    let symbol = visitor
        .style_values
        .root_symbol("Choice")
        .unwrap_or_else(|| panic!("Choice binding"));
    let definition = visitor
        .definition(symbol)
        .unwrap_or_else(|| panic!("Choice definition"));
    assert_eq!(definition.takes_styles, style_capable);
    assert!(definition.takes("title"));
    // Native tags filter invalid DOM props; ordinary components forward them.
    if !style_capable {
        assert_eq!(definition.takes("p"), keeps_padding_prop);
    }
    let padding = std::mem::take(&mut visitor.styles)
        .into_styles("extension.tsx")
        .iter()
        .filter(|style| {
            matches!(style, crate::ExtractStyleValue::Static(value) if value.property() == "padding")
        })
        .count();
    assert_eq!(padding, usize::from(style_capable));
    let code = oxc_codegen::Codegen::new().build(&parsed.program).code;
    assert!(code.contains("title=\"kept\""), "{code}");
    assert_eq!(code.contains("p={2}"), keeps_padding_prop, "{code}");
}

#[rstest::rstest]
#[case::literal_attribute(false)]
#[case::runtime_attribute(true)]
#[serial_test::serial]
fn captured_native_css_keeps_literal_and_runtime_attributes(#[case] runtime_title: bool) {
    let _state = crate::test_state::TestStateGuard::new();
    let title = if runtime_title {
        "title={read('title','kept')}"
    } else {
        "title='kept'"
    };
    let source = format!(
        "import {{jsx}} from '@emotion/react';const events=[];function read(name,value){{events.push(name);return value;}}const App=()=> <div hidden {title} css={{{{color:read('color','red'),color:'blue'}}}}>text</div>;"
    );
    let output = crate::extract(
        "static-attrs.tsx",
        &source,
        crate::ExtractOption {
            single_css: true,
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".into(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let actual = crate::styled_source_tests::captured_render::execute(
        ("static-attrs.tsx", None),
        &output,
        "const element=App();JSON.stringify([events,element.target,element.props.title,element.props.hidden,element.props.children,values(element,'color'),Object.hasOwn(element.props,'css')]);",
    );
    let events = if runtime_title {
        vec!["title", "color"]
    } else {
        vec!["color"]
    };
    assert_eq!(
        actual,
        serde_json::json!([events, "div", "kept", true, "text", ["blue"], false])
    );
}
