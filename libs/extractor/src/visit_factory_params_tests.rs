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

#[rstest::rstest]
#[case::literal_attribute(false)]
#[case::runtime_attribute(true)]
#[serial_test::serial]
fn captured_native_css_suspension_keeps_static_body_and_runtime_control(
    #[case] runtime_title: bool,
) {
    let _state = crate::test_state::TestStateGuard::new();
    let title = if runtime_title {
        "title={read('title','kept')}"
    } else {
        "title='kept'"
    };
    let source = format!(
        "import {{jsx}} from '@emotion/react';const events=[];function read(name,value){{events.push(name);return value;}}function* App(){{return <div hidden {title} css={{{{color:yield read('yield','pause'),color:'blue'}}}}>text</div>;}}"
    );
    // A real suspension must stay outside the JSX, even when its CSS value is overwritten.
    let arena = Allocator::default();
    let mut parsed = Parser::new(&arena, &source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut visitor = DevupVisitor::new(
        &arena,
        "suspending-attrs.tsx",
        "@devup-ui/react",
        Vec::new(),
        None,
    );
    visitor.takes_css_prop(crate::css_prop::CssProp::Everywhere);
    visitor.visit_program(&mut parsed.program);
    assert!(visitor.errors.is_empty(), "{:?}", visitor.errors);
    assert_eq!(visitor.jsx.sequences, !runtime_title);
    let function = parsed
        .program
        .body
        .iter()
        .find_map(|statement| match statement {
            Statement::FunctionDeclaration(function)
                if function.id.as_ref().is_some_and(|id| id.name == "App") =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("App generator"));
    assert!(function.generator);
    let body = function
        .body
        .as_ref()
        .unwrap_or_else(|| panic!("generator body"));
    let Some(Statement::ReturnStatement(statement)) = body.statements.last() else {
        panic!("generator return");
    };
    let value = statement
        .argument
        .as_ref()
        .unwrap_or_else(|| panic!("return value"));
    let value = crate::utils::unwrap_syntax_only(value);
    if runtime_title {
        assert!(matches!(value, Expression::CallExpression(_)));
    } else {
        let Expression::SequenceExpression(sequence) = value else {
            panic!("suspension followed by the static native body");
        };
        assert_eq!(sequence.expressions.len(), 2);
        assert!(matches!(
            sequence.expressions[0],
            Expression::YieldExpression(_)
        ));
        let Expression::JSXElement(element) = &sequence.expressions[1] else {
            panic!("static native body");
        };
        assert!(matches!(&element.opening_element.name,
            JSXElementName::Identifier(name) if name.name == "div"));
        assert!(element.closing_element.as_ref().is_some_and(|closing|
            matches!(&closing.name, JSXElementName::Identifier(name) if name.name == "div")));
        assert!(element.opening_element.type_arguments.is_none());
        assert_eq!(element.children.len(), 1);
        assert!(matches!(&element.children[0], JSXChild::Text(text) if text.value == "text"));
        assert_eq!(element.opening_element.attributes.len(), 3);
        assert!(element.opening_element.attributes.iter().all(
            |attribute| matches!(attribute, JSXAttributeItem::Attribute(attribute)
                if matches!(attribute.value, None | Some(JSXAttributeValue::StringLiteral(_))))
        ));
    }
    // The public pipeline must retain one yield and the original attribute evaluation order.
    let output = crate::extract(
        "suspending-attrs.tsx",
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
        ("suspending-attrs.tsx", None),
        &output,
        "const iterator=App();const first=iterator.next();const before=events.slice();const second=iterator.next('discarded');const element=second.value;const third=iterator.next();JSON.stringify([first.value,first.done,before,second.done,third.done,events,element.target,element.props.title,element.props.hidden,element.props.children,values(element,'color'),Object.hasOwn(element.props,'css')]);",
    );
    let events = if runtime_title {
        vec!["title", "yield"]
    } else {
        vec!["yield"]
    };
    assert_eq!(
        actual,
        serde_json::json!([
            "pause",
            false,
            events,
            true,
            true,
            events,
            "div",
            "kept",
            true,
            "text",
            ["blue"],
            false
        ])
    );
}
