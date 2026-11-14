use super::*;
use rstest::rstest;

#[rstest]
#[case("rest")]
#[case("className")]
fn local_shape_when_callback_capture_is_unresolved_reports_binding_error(
    #[case] name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: these names collide with locals introduced by generated styled scopes.
    let allocator = Allocator::default();
    let source = format!(
        "import {{styled}} from '@devup-ui/react';\nfunction f() {{ const inner = {{color: () => {name}}}; return styled.div(inner); }}"
    );
    let mut parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let offset = u32::try_from(source.find("inner =").ok_or("missing binding")?)?;
    // When
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut parsed.program,
        "capture.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    // Then: the rejected shape stays a binding read, never a relocated callback.
    assert_eq!(
        inlined.errors,
        vec![(offset, crate::utils::local_style_error("inner"))]
    );
    assert_eq!(
        crate::located_errors("capture.tsx", &source, &[], inlined.errors),
        format!(
            "capture.tsx:2:22: {}",
            crate::utils::local_style_error("inner")
        )
    );
    let generated = oxc_codegen::Codegen::new().build(&parsed.program).code;
    assert!(generated.contains("styled.div(inner)"), "{generated}");
    Ok(())
}

#[test]
fn local_shape_when_callback_capture_is_resolved_retains_lexical_read() {
    // Given
    let allocator = Allocator::default();
    let source = "import {styled} from '@devup-ui/react'; function f(p) { const rest = p.color; const inner = {color: () => rest}; return styled.div(inner); }";
    let mut parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    // When
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut parsed.program,
        "capture.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    // Then
    assert_eq!(inlined.errors, vec![]);
    let generated = oxc_codegen::Codegen::new().build(&parsed.program).code;
    assert!(
        generated.contains("color: () => __devup_rest"),
        "{generated}"
    );
}

#[rstest]
#[case(true)]
#[case(false)]
fn renamed_shorthand_when_capture_alias_changes_preserves_public_key_and_value(
    #[case] rename: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: only rest is renamed; the unrelated shorthand is a control.
    let allocator = Allocator::default();
    let mut parsed = Parser::new(
        &allocator,
        "function f() { const rest = 'purple'; const unchanged = 'green'; return {rest, unchanged}; }",
        SourceType::tsx(),
    ).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let symbol = scoping
        .symbol_ids()
        .find(|symbol| scoping.symbol_name(*symbol) == "rest")
        .ok_or("missing captured binding")?;
    let aliases = if rename {
        FxHashMap::from_iter([(symbol, "__devup_rest".to_string())])
    } else {
        FxHashMap::default()
    };
    // When
    local_capture_reads::Rename {
        builder: &AstBuilder::new(&allocator),
        scoping: &scoping,
        aliases: &aliases,
    }
    .visit_program(&mut parsed.program);
    let generated = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let script = format!("{generated}\nJSON.stringify(f());");
    let value = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .map_err(|error| error.to_string())?;
    let json = value
        .as_string()
        .ok_or("expected JSON result")?
        .to_std_string_escaped();
    // Then: a shorthand accidentally kept after renaming would change the key.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json)?,
        serde_json::json!({"rest": "purple", "unchanged": "green"})
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn local_shape_when_emotion_styles_read_runtime_leaves_preserves_pixel_context() {
    // Given
    let source = "import styled from '@emotion/styled';function make(input){const shape={padding:2,color:input.color};return styled.div(shape);}const Choice=make({color:'red'});";
    let option = ExtractOption {
        single_css: true,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/styled".into(),
            crate::ImportAlias::DefaultToNamed("styled".into()),
        )]),
        ..Default::default()
    };
    let output =
        crate::extract("local-shape.tsx", source, option).unwrap_or_else(|error| panic!("{error}"));
    // When
    let observed = crate::styled_source_tests::captured_render::execute(
        ("local-shape.tsx", None),
        &output,
        "const element=render({target:Choice,props:{}});JSON.stringify([values(element,'color'),values(element,'padding')]);",
    );
    // Then
    assert_eq!(observed, serde_json::json!([["red"], ["2px"]]));
}

#[rstest]
#[case("{}", "blue")]
#[case("{color:'green'}", "red")]
#[serial_test::serial]
fn imported_class_when_used_as_a_controller_selects_by_declarations_not_metadata(
    #[case] rules: &str,
    #[case] expected: &str,
) {
    // Given
    let module = format!("import {{css}} from '@devup-ui/react';export const mixin=css({rules});");
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./base").then(|| crate::ResolvedModule {
            path: "/base.tsx".into(),
            code: module.clone(),
        })
    };
    let source = "import {styled} from '@devup-ui/react';import {mixin} from './base';const Choice=styled.div(mixin?{color:'red'}:{color:'blue'});";
    // When
    let output = crate::extract_with_modules(
        "/entry.tsx",
        source,
        ExtractOption {
            single_css: true,
            ..Default::default()
        },
        false,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    let colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "color" => Some(style.value()),
            _ => None,
        })
        .collect();
    assert_eq!(colors, [expected]);
}

#[rstest]
#[case("{color:(()=>later)()}", true)]
#[case("{color:(function(){return later})()}", true)]
#[case("{color:()=>later}", false)]
#[case("{color:function(){return later}}", false)]
fn local_shape_tdz_checks_eager_iifes_and_leaves_deferred_callbacks(
    #[case] initializer: &str,
    #[case] expected: bool,
) {
    let allocator = Allocator::default();
    let source = format!("const shape={initializer};const later='red';");
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let mut declarations = LocalConstants::default();
    declarations.visit_program(&parsed.program);
    let oxc_ast::ast::Statement::VariableDeclaration(declaration) = &parsed.program.body[0] else {
        panic!("expected shape declaration")
    };
    let init = declaration.declarations[0]
        .init
        .as_ref()
        .unwrap_or_else(|| panic!("expected shape initializer"));
    let before = crate::utils::readable_code(init);
    assert_eq!(
        local_rule_shapes::has_tdz(init, &scoping, &declarations.bindings),
        expected
    );
    assert_eq!(crate::utils::readable_code(init), before);
}

#[test]
fn selected_jsx_style_spread_keeps_the_authored_winning_value() {
    let allocator = Allocator::default();
    let source = "import {Box} from '@devup-ui/react';const selected=true;const color='red';<Box {...(selected ? {color} : {color:'blue'})}/>;";
    let mut parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut parsed.program,
        "spread.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    assert!(inlined.errors.is_empty(), "{:?}", inlined.errors);
    let Some(oxc_ast::ast::Statement::ExpressionStatement(statement)) = parsed.program.body.last()
    else {
        panic!("expected element expression")
    };
    let Expression::JSXElement(element) = &statement.expression else {
        panic!("expected JSX")
    };
    let oxc_ast::ast::JSXAttributeItem::SpreadAttribute(spread) =
        &element.opening_element.attributes[0]
    else {
        panic!("spread must remain an authored copy")
    };
    let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(&spread.argument)
    else {
        panic!("constant controller must select exactly one object")
    };
    assert_eq!(object.properties.len(), 1);
    let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        panic!("expected own property")
    };
    assert!(
        property
            .key
            .static_name()
            .is_some_and(|name| name == "color")
    );
    assert!(matches!(&property.value, Expression::StringLiteral(value) if value.value == "red"));
    assert!(!property.computed);
    assert!(!property.method);
}
#[rstest]
#[case("title={mark('before','title')}")]
#[case("{...{title:mark('before','title')}}")]
#[serial_test::serial]
fn intrinsic_css_relocation_preserves_evaluation_order_with_spread_control(#[case] before: &str) {
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "import {{jsx}} from '@emotion/react';const events=[];function mark(name,value){{events.push(name);return value;}}const App=()=> <div {before} css={{{{color:mark('color','red')}}}} id={{mark('after','id')}}/>;"
    );
    let output = crate::extract(
        "boundary-order.tsx",
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
        ("boundary-order.tsx", None),
        &output,
        "const element=App();JSON.stringify([events,element.target,element.props.title,element.props.id,values(element,'color'),Object.hasOwn(element.props,'css')]);",
    );
    assert_eq!(
        actual,
        serde_json::json!([
            ["before", "color", "after"],
            "div",
            "title",
            "id",
            ["red"],
            false
        ])
    );
}

#[test]
#[serial_test::serial]
fn overwritten_dynamic_css_retains_boolean_attribute_and_once_only_effect() {
    let _state = crate::test_state::TestStateGuard::new();
    let source = "import {jsx} from '@emotion/react';const events=[];function read(){events.push('read');return 'red';}const App=()=> <div hidden css={{color:read(),color:'blue'}}/>;";
    let output = crate::extract(
        "boolean-static.tsx",
        source,
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
        ("boolean-static.tsx", None),
        &output,
        "const element=App();JSON.stringify([events,element.target,element.props.hidden,values(element,'color'),Object.hasOwn(element.props,'css')]);",
    );
    assert_eq!(
        actual,
        serde_json::json!([["read"], "div", true, ["blue"], false])
    );
}

#[rstest]
#[case("", "undefined", false, false, 0)]
#[case(",null", "null", false, false, 0)]
#[case(",undefined", "undefined", false, false, 0)]
#[case(",{}", "object", false, false, 0)]
#[case(",readProps()", "object", true, false, 1)]
#[case(",{['css']:{color:'red'}}", "object", false, true, 0)]
#[serial_test::serial]
fn compiled_intrinsic_without_eligible_css_preserves_raw_props(
    #[case] arguments: &str,
    #[case] kind: &str,
    #[case] identity: bool,
    #[case] has_css: bool,
    #[case] calls: usize,
) {
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "import {{jsx}} from '@emotion/react';let calls=0;const shared={{id:'kept'}};function readProps(){{calls++;return shared;}}const Control=()=>jsx('div',{{css:{{color:'red'}}}});const App=()=>jsx('div'{arguments});"
    );
    let output = crate::extract(
        "raw-css-controls.ts",
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
        ("raw-css-controls.ts", None),
        &output,
        "const element=App();const control=Control();JSON.stringify([element.target,element.props===null?'null':typeof element.props,element.props===shared,Object.hasOwn(element.props||{},'css'),calls,values(control,'color'),Object.hasOwn(control.props,'css')]);",
    );
    assert_eq!(
        actual,
        serde_json::json!(["div", kind, identity, has_css, calls, ["red"], false])
    );
}

#[test]
fn declaration_collection_keeps_nested_const_ends_and_excludes_non_bindings() {
    let arena = Allocator::default();
    let source = "const outer = () => { const nested = 4; }; let ignored = 2; var legacy = 3; const { destructured } = input; declare const ambient: string;";
    let parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let before = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let mut declarations = LocalConstants::default();

    declarations.visit_program(&parsed.program);

    let at = |needle: &str| {
        u32::try_from(
            source
                .find(needle)
                .unwrap_or_else(|| panic!("missing {needle}")),
        )
        .unwrap_or_else(|error| panic!("fixture offset: {error}"))
    };
    assert_eq!(
        declarations.bindings,
        FxHashMap::from_iter([(at("outer"), at("};") + 1), (at("nested"), at("4;") + 1)])
    );
    assert_eq!(
        oxc_codegen::Codegen::new().build(&parsed.program).code,
        before
    );
}

#[rstest]
#[case("[0, ...[1, 2], 3]", &["0", "1", "2", "3"])]
#[case("[0, ...[], 3]", &["0", "3"])]
#[case("[0, ...[1, ...[2]], 3]", &["0", "1", "2", "3"])]
fn exact_array_spreads_keep_element_order_and_empty_spreads(
    #[case] source: &str,
    #[case] expected: &[&str],
) {
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("array fixture");
    };
    let before = crate::utils::readable_code(&statement.expression);
    let option = ExtractOption::default();
    let mut scope = ModuleScope::new("array.ts", &parsed.program, None);
    let mut modules = Modules {
        resolver: None,
        option: &option,
        exports: FxHashMap::default(),
        loading: vec![],
    };

    let Some(Constant::Array(values)) = scope.evaluate(&mut modules, &statement.expression) else {
        panic!("exact array spread must evaluate");
    };

    let actual = values.iter().map(js_string).collect::<Option<Vec<_>>>();
    assert_eq!(
        actual,
        Some(expected.iter().map(|value| (*value).to_string()).collect())
    );
    assert_eq!(crate::utils::readable_code(&statement.expression), before);
}

#[rstest]
#[case::eager_forward("const shape = { color: later };const later='red';", true)]
#[case::declared_before("const later='red';const shape = { color: later };", false)]
#[case::deferred_callback("const shape = {color:()=>later};const later='red';", false)]
fn referenced_local_shapes_reject_eager_tdz_and_keep_safe_controls(
    #[case] declarations: &str,
    #[case] rejected: bool,
) {
    let arena = Allocator::default();
    let source = format!(
        "import {{styled}} from '@devup-ui/react';function make(){{{declarations}return styled.div(shape);}}"
    );
    let mut parsed = Parser::new(&arena, &source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let before = oxc_codegen::Codegen::new().build(&parsed.program).code;
    {
        // The public pipeline rejects eager TDZ before this visitor runs.
        // Exercise the collector using real semantic IDs and declaration ends.
        let scoping = SemanticBuilder::new()
            .build(&parsed.program)
            .semantic
            .into_scoping();
        let symbol = scoping
            .symbol_ids()
            .find(|symbol| scoping.symbol_name(*symbol) == "shape")
            .unwrap_or_else(|| panic!("shape symbol"));
        assert_eq!(scoping.get_resolved_reference_ids(symbol).len(), 1);
        let mut ends = LocalConstants::default();
        ends.visit_program(&parsed.program);
        let option = ExtractOption::default();
        let mut scope = ModuleScope::new("local-tdz.tsx", &parsed.program, None);
        scope.scoped = Some(&scoping);
        scope.shared_scoping = Some(&scoping);
        let mut modules = Modules {
            resolver: None,
            option: &option,
            exports: FxHashMap::default(),
            loading: vec![],
        };
        let mut found = Vec::new();
        let mut shapes = FxHashMap::default();
        let mut invalid = FxHashSet::default();
        ScopedLocals {
            scope: &mut scope,
            modules: &mut modules,
            scoping: &scoping,
            found: &mut found,
            builder: &AstBuilder::new(&arena),
            shapes: &mut shapes,
            ends: &ends.bindings,
            invalid: &mut invalid,
        }
        .visit_program(&parsed.program);
        let local = found
            .iter()
            .find(|local| local.name == "shape")
            .unwrap_or_else(|| panic!("referenced shape must be collected"));
        assert_eq!(local.binding, scoping.symbol_span(symbol).start);
        assert_eq!(Some(&local.end), ends.bindings.get(&local.binding));
        assert_eq!(invalid.contains(&symbol), rejected);
        if rejected {
            assert!(!shapes.contains_key(&symbol));
            assert!(local.value.is_none());
        } else {
            assert!(shapes.contains_key(&symbol) || local.value.is_some());
        }
        assert_eq!(
            oxc_codegen::Codegen::new().build(&parsed.program).code,
            before
        );
    }
    let inlined = inline_constants(
        &AstBuilder::new(&arena),
        &mut parsed.program,
        "local-tdz.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    if rejected {
        let offset = u32::try_from(
            source
                .find("color: later")
                .unwrap_or_else(|| panic!("eager read"))
                + "color: ".len(),
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(inlined.errors, vec![(offset, "style value cannot use `later` at build time: it is read before its lexical declaration is initialized, so the original JavaScript throws ReferenceError; move the declaration of `later` above this use; containing style shapes: shape".to_string())]);
        assert!(matches!(
            inlined.disposition,
            ErrorDisposition::TerminalInitialization
        ));
        assert_eq!(
            oxc_codegen::Codegen::new().build(&parsed.program).code,
            before
        );
    } else {
        assert!(inlined.errors.is_empty(), "{:?}", inlined.errors);
        assert!(matches!(inlined.disposition, ErrorDisposition::Recoverable));
    }
}

#[rstest]
#[case::copy_only(false)]
#[case::identity_escape(true)]
#[serial_test::serial]
fn native_nested_spreads_inline_only_copy_exclusive_bindings(#[case] escapes: bool) {
    let _state = crate::test_state::TestStateGuard::new();
    let arena = Allocator::default();
    let escape = if escapes {
        "const identity=props;if(identity!==props){throw new Error('changed identity');}"
    } else {
        ""
    };
    let source = format!(
        "import {{styled}} from '@devup-ui/react';function make(){{const props={{title:'kept',color:'red'}};{escape}return <div {{...{{...props}}}}/>;}}"
    );
    let mut parsed = Parser::new(&arena, &source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let inlined = inline_constants(
        &AstBuilder::new(&arena),
        &mut parsed.program,
        "native-copy.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    assert!(inlined.errors.is_empty(), "{:?}", inlined.errors);
    let function = parsed
        .program
        .body
        .iter()
        .find_map(|statement| match statement {
            Statement::FunctionDeclaration(function)
                if function.id.as_ref().is_some_and(|id| id.name == "make") =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("make function"));
    let body = function
        .body
        .as_ref()
        .unwrap_or_else(|| panic!("function body"));
    let Some(oxc_ast::ast::Statement::ReturnStatement(statement)) = body.statements.last() else {
        panic!("return element")
    };
    let Some(Expression::JSXElement(element)) = &statement.argument else {
        panic!("native element")
    };
    assert_eq!(element.opening_element.name.to_string(), "div");
    let oxc_ast::ast::JSXAttributeItem::SpreadAttribute(spread) =
        &element.opening_element.attributes[0]
    else {
        panic!("outer spread")
    };
    assert_eq!(crate::visit::is_unknown_spread(&spread.argument), escapes);
    let Expression::ObjectExpression(outer) = crate::utils::unwrap_syntax_only(&spread.argument)
    else {
        panic!("outer own object")
    };
    assert_eq!(outer.properties.len(), 1);
    let ObjectPropertyKind::SpreadProperty(inner) = &outer.properties[0] else {
        panic!("nested copy")
    };
    if escapes {
        assert!(
            matches!(crate::utils::unwrap_syntax_only(&inner.argument), Expression::Identifier(identifier) if identifier.name == "props")
        );
    } else {
        let Expression::ObjectExpression(props) = crate::utils::unwrap_syntax_only(&inner.argument)
        else {
            panic!("copy expands to own properties")
        };
        let actual: Vec<_> = props
            .properties
            .iter()
            .map(|property| {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    panic!("own property")
                };
                assert!(!property.computed && !property.method);
                let Expression::StringLiteral(value) = &property.value else {
                    panic!("inert string value")
                };
                (
                    property
                        .key
                        .static_name()
                        .unwrap_or_else(|| panic!("static key"))
                        .as_str()
                        .unwrap_or_else(|| panic!("UTF8 static key"))
                        .to_string(),
                    value
                        .value
                        .as_str()
                        .unwrap_or_else(|| panic!("UTF8 value"))
                        .to_string(),
                )
            })
            .collect();
        assert_eq!(
            actual,
            [
                ("title".to_string(), "kept".to_string()),
                ("color".to_string(), "red".to_string())
            ]
        );
    }
    let output = crate::extract("native-copy.tsx", &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.styles.iter().all(|style| !matches!(
        style,
        ExtractStyleValue::Static(_) | ExtractStyleValue::Dynamic(_)
    )));
    let actual = crate::styled_source_tests::captured_render::execute(
        ("native-copy.tsx", None),
        &output,
        "const element=make();JSON.stringify([element.target,element.props.title,element.props.color]);",
    );
    assert_eq!(actual, serde_json::json!(["div", "kept", "red"]));
}
