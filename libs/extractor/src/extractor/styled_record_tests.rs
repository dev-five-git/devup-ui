use boa_engine::{Context, Source};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast::{ast::Statement, builder::AstBuilder};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::{styled_record::prepare, styled_record_evaluation::bind_render};
use crate::utils::readable_code;

#[rstest]
#[case::prepared_base("const Ext=styled(Base)({'p':1});", "", "4px")]
#[case::two_levels(
    "const Mid=styled(Base)({'p':1});const Ext=styled(Mid)({'p':2});",
    "",
    "8px"
)]
#[case::tagged_extension("const Ext=styled(Base)`padding:4px`;", "", "4px")]
#[case::retargeted_extension(
    "const Mid=Base.withComponent('section');const Ext=styled(Mid)({'p':1});",
    "",
    "4px"
)]
#[case::capture_collisions(
    "const Mid=styled(Base)({backgroundColor:'white',...((trace.push('own'),own)?{backgroundColor:'black'}:{})});const Ext=styled(Mid)(__devupRecord0,{p:1,...((trace.push('authored'),__devupRecord1)?{p:2}:{})});",
    ",own,__devupRecord0,__devupRecord1",
    "8px"
)]
#[serial_test::serial]
fn inherited_captures_when_native_extension_renders(
    #[case] extension: &str,
    #[case] parameters: &str,
    #[case] padding: &str,
    #[values(false, true)] on: bool,
    #[values(false, true)] authored: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: independent inherited, own and authored controllers.
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react';const trace=[];function make(on{parameters}){{const Base=styled.div({{color:'red',...((trace.push('base'),on)?{{color:'blue'}}:{{}})}});{extension}return Ext.render({{id:'extension',className:'caller'}},null);}}const rendered=make({on},{},'authored',{authored});",
        !on
    );
    let output = match crate::extract(
        "test.tsx",
        &code,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    ) {
        Ok(output) => output,
        Err(error) => panic!("extension fixture must extract: {error}"),
    };
    let mut declarations = Vec::new();
    for style in &output.styles {
        let crate::ExtractStyleValue::Static(value) = style else {
            panic!("fixture declarations must be static");
        };
        let Some(crate::extract_style::style_property::StyleProperty::ClassName(name)) =
            style.extract(if single_css { None } else { Some("test.tsx") })
        else {
            panic!("fixture declaration must have a class");
        };
        declarations.push(format!(
            "['{name}','{}','{}']",
            value.property(),
            value.value()
        ));
    }
    // When: execute the emitted native render, not a generated-name assertion.
    let rendered = evaluate_native_render(
        &output.code,
        &format!(
            "const names=rendered.props.className.split(' ');const styles=[{}].filter(([name])=>names.includes(name)).map(([,prop,value])=>[prop,value]).sort();JSON.stringify([rendered.type,rendered.props.id,styles,names.filter(name=>name==='authored'||name==='caller'),trace]);",
            declarations.join(",")
        ),
    );
    // Then: only the selected declarations survive, with ordered single reads.
    let color = if on { "blue" } else { "red" };
    let target = if extension.contains("withComponent") {
        "section"
    } else {
        "div"
    };
    let expected = if parameters.is_empty() {
        format!(
            r#"["{target}","extension",[["color","{color}"],["padding","{padding}"]],["caller"],["base"]]"#
        )
    } else {
        let background = if on { "white" } else { "black" };
        let padding = if authored { padding } else { "4px" };
        format!(
            r#"["div","extension",[["background-color","{background}"],["color","{color}"],["padding","{padding}"]],["authored","caller"],["base","own","authored"]]"#
        )
    };
    assert_eq!(rendered, expected);
}

#[rstest]
#[serial_test::serial]
fn inherited_captures_when_dynamic_native_base_renders(
    #[values(false, true)] on: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: a lowercase runtime target binding and an inherited captured guard.
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react';const trace=[];function make(on,target){{const Base=styled(target)({{color:'red',...((trace.push('base'),on)?{{color:'blue'}}:{{}})}});const Ext=styled(Base)({{p:1}});return Ext.render({{id:'extension',className:'caller'}},null);}}const rendered=make({on},'div');"
    );
    let output = match crate::extract(
        "test.tsx",
        &code,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    ) {
        Ok(output) => output,
        Err(error) => panic!("dynamic base extension must extract: {error}"),
    };
    let mut declarations = Vec::new();
    for style in &output.styles {
        let crate::ExtractStyleValue::Static(value) = style else {
            panic!("fixture declarations must be static");
        };
        let Some(crate::extract_style::style_property::StyleProperty::ClassName(name)) =
            style.extract(if single_css { None } else { Some("test.tsx") })
        else {
            panic!("fixture declaration must have a class");
        };
        declarations.push(format!(
            "['{name}','{}','{}']",
            value.property(),
            value.value()
        ));
    }
    // When: execute the emitted extension with its actual runtime tag value.
    let rendered = evaluate_native_render(
        &output.code,
        &format!(
            "const names=rendered.props.className.split(' ');const styles=[{}].filter(([name])=>names.includes(name)).map(([,prop,value])=>[prop,value]).sort();JSON.stringify([rendered.type,rendered.props.id,styles,names.filter(name=>name==='caller'),trace]);",
            declarations.join(",")
        ),
    );
    // Then: the tag, selected color, extension padding and single read survive.
    let color = if on { "blue" } else { "red" };
    assert_eq!(
        rendered,
        format!(
            r#"["div","extension",[["color","{color}"],["padding","4px"]],["caller"],["base"]]"#
        )
    );
}

#[rstest]
#[serial_test::serial]
fn inherited_dependent_reads_when_attrs_and_later_styles_compose(
    #[values(false, true)] outer: bool,
    #[values(false, true)] inner: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: nested captured guards, a dynamic leaf and an authored attrs name.
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react';const trace=[];function make(outer,inner,__devupRecord0){{const Base=styled.div.attrs({{title:__devupRecord0,id:'base'}})({{color:'red',backgroundColor:'white',...((trace.push('outer'),outer)?{{...((trace.push('inner'),inner)?{{backgroundColor:(trace.push('leaf'),'gold')}}:{{}})}}:{{}})}});const Ext=styled(Base).attrs({{id:'extension'}})({{color:'orange'}});return Ext.render({{}},null);}}const rendered=make({outer},{inner},'authored-title');"
    );
    let output = match crate::extract(
        "test.tsx",
        &code,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    ) {
        Ok(output) => output,
        Err(error) => panic!("dependent extension must extract: {error}"),
    };
    let mut colors = Vec::new();
    for style in &output.styles {
        if let crate::ExtractStyleValue::Static(value) = style
            && value.property() == "color"
            && let Some(crate::extract_style::style_property::StyleProperty::ClassName(name)) =
                style.extract(if single_css { None } else { Some("test.tsx") })
        {
            colors.push(format!("['{name}','{}']", value.value()));
        }
    }
    // When: render the flattened extension through the existing native harness.
    let rendered = evaluate_native_render(
        &output.code,
        &format!(
            "const names=rendered.props.className.split(' ');JSON.stringify([rendered.props.id,rendered.props.title,[{}].filter(([name])=>names.includes(name)).map(([,value])=>value),Object.values(rendered.props.style||{{}}).filter(value=>value!=null),trace]);",
            colors.join(",")
        ),
    );
    // Then: guarded reads stay single and ordered, attrs and later color win.
    let expected = match (outer, inner) {
        (true, true) => {
            r#"["extension","authored-title",["orange"],["gold"],["outer","inner","leaf"]]"#
        }
        (true, false) => r#"["extension","authored-title",["orange"],[],["outer","inner"]]"#,
        (false, _) => r#"["extension","authored-title",["orange"],[],["outer"]]"#,
    };
    assert_eq!(rendered, expected);
}

fn evaluate_record(rules: &str, setup: &str) -> String {
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = format!("({rules});");
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("record fixture must be an expression");
    };
    let prepared = match prepare(&builder, &statement.expression, &statement.expression) {
        Some(prepared) => prepared,
        None => panic!("record fixture must require preparation"),
    };
    let render = bind_render(&builder, prepared.bindings, prepared.rules);
    let script = format!(
        "const trace = []; {setup}; const result = {}; JSON.stringify([Object.keys(result), result, trace]);",
        readable_code(&render)
    );
    let mut context = Context::default();
    let result = match context.eval(Source::from_bytes(script.as_bytes())) {
        Ok(result) => result,
        Err(error) => panic!("record evaluation failed: {error}"),
    };
    match result.as_string() {
        Some(result) => result.to_std_string_escaped(),
        None => panic!("record evaluation must return a JSON string"),
    }
}

#[rstest]
#[case::later_explicit(
    "{ ...(a ? { color: 'red' } : { color: 'purple' }), color: 'orange' }",
    "const a = true",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::missing_true(
    "{ color: 'orange', ...(a ? {} : { color: 'purple' }) }",
    "const a = true",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::missing_false(
    "{ color: 'orange', ...(a ? { color: 'red' } : {}) }",
    "const a = false",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::supplied_undefined(
    "{ color: 'orange', ...(a ? { color: undefined } : {}) }",
    "const a = true",
    r#"[["color"],{},[]]"#
)]
#[case::later_spread(
    "{ ...(a ? { color: 'red' } : {}), color: 'orange', ...(b ? { color: 'blue' } : {}) }",
    "const a = true, b = true",
    r#"[["color"],{"color":"blue"},[]]"#
)]
#[case::later_spread_missing(
    "{ ...(a ? { color: 'red' } : {}), color: 'orange', ...(b ? { color: 'blue' } : {}) }",
    "const a = false, b = false",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::nested_missing(
    "{ color: 'orange', ...(a ? { ...(b ? { color: 'blue' } : {}) } : {}) }",
    "const a = true, b = false",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::nested_supplied(
    "{ color: 'orange', ...(a ? { ...(b ? { color: 'blue' } : {}) } : {}) }",
    "const a = true, b = true",
    r#"[["color"],{"color":"blue"},[]]"#
)]
#[case::logical_present(
    "{ color: 'orange', ...(a && { color: 'red' }) }",
    "const a = true",
    r#"[["color"],{"color":"red"},[]]"#
)]
#[case::logical_missing(
    "{ color: 'orange', ...(a && { color: 'red' }) }",
    "const a = false",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::null_branch(
    "{ color: 'orange', ...(a ? null : { color: 'red' }) }",
    "const a = true",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::undefined_branch(
    "{ color: 'orange', ...(a ? { color: 'red' } : undefined) }",
    "const a = false",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::false_branch(
    "{ color: 'orange', ...(a ? false : { color: 'red' }) }",
    "const a = true",
    r#"[["color"],{"color":"orange"},[]]"#
)]
#[case::shallow_selector(
    "{ _hover: { color: 'red', backgroundColor: 'yellow' }, ...(a ? { _hover: { color: 'blue' } } : {}) }",
    "const a = true",
    r#"[["_hover"],{"_hover":{"color":"blue"}},[]]"#
)]
#[case::selector_fallback(
    "{ _hover: { color: 'red', backgroundColor: 'yellow' }, ...(a ? { _hover: { color: 'blue' } } : {}) }",
    "const a = false",
    r#"[["_hover"],{"_hover":{"color":"red","backgroundColor":"yellow"}},[]]"#
)]
#[case::responsive_replace(
    "{ color: ['red', null, 'green'], ...(a ? { color: ['blue'] } : {}) }",
    "const a = true",
    r#"[["color"],{"color":["blue"]},[]]"#
)]
#[case::responsive_fallback(
    "{ color: ['red', null, 'green'], ...(a ? { color: ['blue'] } : {}) }",
    "const a = false",
    r#"[["color"],{"color":["red",null,"green"]},[]]"#
)]
#[case::wrapped(
    "{ color: 'orange', ...((a ? { color: 'red' } : {}) satisfies object) }",
    "const a = true",
    r#"[["color"],{"color":"red"},[]]"#
)]
#[case::nested_selector(
    "{ _hover: { color: 'orange', ...(a ? { color: 'red' } : {}) } }",
    "const a = false",
    r#"[["_hover"],{"_hover":{"color":"orange"}},[]]"#
)]
#[case::literal_spread(
    "{ ...{ color: 'orange', p: 1 }, ...(a ? { color: 'red' } : {}), ...{ color: 'blue' } }",
    "const a = true",
    r#"[["p","color"],{"p":1,"color":"blue"},[]]"#
)]
#[case::selector_text(
    "{ _hover: { color: 'red' }, ...(a ? { _hover: `color:blue` } : {}) }",
    "const a = true",
    r#"[["_hover"],{"_hover":"color:blue"},[]]"#
)]
#[case::literal_template_key(
    "{ ...(a ? { color: 'red' } : {}), [`co${'lor'}`]: 'orange' }",
    "const a = true",
    r#"[["color"],{"color":"orange"},[]]"#
)]
fn record_precedence_when_branch_presence_varies(
    #[case] rules: &str,
    #[case] setup: &str,
    #[case] expected: &str,
) {
    let result = evaluate_record(rules, setup);
    assert_eq!(result, expected);
}

#[rstest]
#[case::overwritten(
    "{ color: (trace.push('before'), true) ? 'purple' : 'navy', ...((trace.push('spread'), true) ? { color: 'red' } : {}), color: (trace.push('after'), 'orange') }",
    "",
    r#"[["color"],{"color":"orange"},["before","spread","after"]]"#
)]
#[case::literal_computed_overwritten(
    "{ ...((trace.push('spread'), true) ? { ['color']: (trace.push('leaf'), 'red') } : {}), color: (trace.push('after'), 'orange') }",
    "",
    r#"[["color"],{"color":"orange"},["spread","leaf","after"]]"#
)]
#[case::literal_computed_unselected(
    "{ ...((trace.push('spread'), false) ? { ['color']: (trace.push('leaf'), 'red') } : {}), color: (trace.push('after'), 'orange') }",
    "",
    r#"[["color"],{"color":"orange"},["spread","after"]]"#
)]
#[case::literal_computed_selected(
    "{ color: 'orange', ...((trace.push('spread'), true) ? { ['color']: 'red' } : {}) }",
    "",
    r#"[["color"],{"color":"red"},["spread"]]"#
)]
#[case::template_computed_overwritten(
    "{ ...((trace.push('spread'), true) ? { [`color`]: (trace.push('leaf'), 'red') } : {}), [`color`]: (trace.push('after'), 'orange') }",
    "",
    r#"[["color"],{"color":"orange"},["spread","leaf","after"]]"#
)]
#[case::template_computed_unselected(
    "{ ...((trace.push('spread'), false) ? { [`color`]: (trace.push('leaf'), 'red') } : {}), color: (trace.push('after'), 'orange') }",
    "",
    r#"[["color"],{"color":"orange"},["spread","after"]]"#
)]
#[case::template_computed_selected(
    "{ color: 'orange', ...((trace.push('spread'), true) ? { [`color`]: 'red' } : {}) }",
    "",
    r#"[["color"],{"color":"red"},["spread"]]"#
)]
#[case::branch_guard(
    "{ color: 'orange', ...((trace.push('outer'), false) ? { ...((trace.push('inner'), true) ? { color: (trace.push('leaf'), 'red') } : {}) } : {}) }",
    "",
    r#"[["color"],{"color":"orange"},["outer"]]"#
)]
#[case::selected_nested(
    "{ color: 'orange', ...((trace.push('outer'), true) ? { ...((trace.push('inner'), true) ? { color: (trace.push('leaf'), 'red') } : {}) } : {}) }",
    "",
    r#"[["color"],{"color":"red"},["outer","inner","leaf"]]"#
)]
#[case::getter_occurrences(
    "{ color: 'orange', ...(state.on ? { color: 'red', bg: 'white' } : {}), ...(state.on ? { color: 'blue' } : {}) }",
    "const state = { get on() { trace.push('get'); return true } }",
    r#"[["bg","color"],{"bg":"white","color":"blue"},["get","get"]]"#
)]
#[case::captured_identifier(
    "{ color: 'orange', ...(on ? { color: 'red' } : {}), ...((on = false) ? { color: 'blue' } : {}) }",
    "let on = true",
    r#"[["color"],{"color":"red"},[]]"#
)]
#[case::name_collision(
    "{ color: 'orange', ...(__devupRecord0 ? { color: __devupRecord1 } : {}) }",
    "const __devupRecord0 = true, __devupRecord1 = 'red'",
    r#"[["color"],{"color":"red"},[]]"#
)]
#[case::equal_branches(
    "{ color: (trace.push('before'), true) ? 'red' : 'red', ...((trace.push('spread'), true) ? { color: 'red' } : {}), color: 'orange' }",
    "",
    r#"[["color"],{"color":"orange"},["before","spread"]]"#
)]
#[case::value_logical(
    "{ color: 'orange', ...(on ? { color: 'red' } : {}), p: (trace.push('left'), true) && (trace.push('right'), 2) }",
    "const on = false",
    r#"[["color","p"],{"color":"orange","p":2},["left","right"]]"#
)]
#[case::value_or(
    "{ color: 'orange', ...(on ? { color: 'red' } : {}), p: (trace.push('left'), false) || (trace.push('right'), 2) }",
    "const on = false",
    r#"[["color","p"],{"color":"orange","p":2},["left","right"]]"#
)]
#[case::value_nullish(
    "{ color: 'orange', ...(on ? { color: 'red' } : {}), p: (trace.push('left'), null) ?? (trace.push('right'), 2) }",
    "const on = false",
    r#"[["color","p"],{"color":"orange","p":2},["left","right"]]"#
)]
#[case::logical_skipped(
    "{ color: 'orange', ...(on ? { color: 'red' } : {}), p: (trace.push('left'), false) && (trace.push('right'), 2) }",
    "const on = false",
    r#"[["color","p"],{"color":"orange","p":false},["left"]]"#
)]
#[case::branch_alternate(
    "{ color: 'orange', ...((trace.push('outer'), false) ? {} : { color: (trace.push('leaf'), 'blue') }) }",
    "",
    r#"[["color"],{"color":"blue"},["outer","leaf"]]"#
)]
fn record_reads_when_relocated_or_overwritten(
    #[case] rules: &str,
    #[case] setup: &str,
    #[case] expected: &str,
) {
    let result = evaluate_record(rules, setup);
    assert_eq!(result, expected);
}

#[rstest]
#[case::plain("{ color: 'red' }")]
#[case::unconditional("{ ...{ color: 'red' }, color: 'orange' }")]
#[case::unknown_spread("{ color: 'orange', ...(on ? { color: 'red' } : {}), ...unknown }")]
#[case::unknown_branch("{ color: 'orange', ...(on ? unknown : { color: 'red' }) }")]
#[case::computed_key("{ color: 'orange', ...(on ? { [key]: 'red' } : {}) }")]
#[case::computed_effect("{ color: 'orange', ...(on ? { [(read(), 'color')]: 'red' } : {}) }")]
#[case::computed_template_unknown("{ color: 'orange', ...(on ? { [`${key}`]: 'red' } : {}) }")]
#[case::computed_template_effect("{ color: 'orange', ...(on ? { [`${read()}`]: 'red' } : {}) }")]
#[case::computed_getter("{ color: 'orange', ...(on ? { get ['color']() { return 'red' } } : {}) }")]
#[case::computed_method("{ color: 'orange', ...(on ? { ['color']() { return 'red' } } : {}) }")]
#[case::getter("{ color: 'orange', ...(on ? { get color() { return 'red' } } : {}) }")]
#[case::method("{ color: 'orange', ...(on ? { color() { return 'red' } } : {}) }")]
#[case::array_spread("{ color: 'orange', ...(on ? { color: [...values] } : {}) }")]
fn original_record_when_unaffected_or_unreadable(#[case] rules: &str) {
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = format!("({rules});");
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("record fixture must be an expression");
    };
    let original = readable_code(&statement.expression);
    assert!(prepare(&builder, &statement.expression, &statement.expression).is_none());
    assert_eq!(readable_code(&statement.expression), original);
}

#[rstest]
#[case::distinct("{ ...(on ? { color: 'red' } : {}), p: 1 }")]
#[case::computed_unaffected("{ ...(on ? { ['color']: 'red' } : {}), p: 1 }")]
fn retained_guard_when_record_has_distinct_keys(
    #[case] rules: &str,
    #[values(false, true)] enabled: bool,
) {
    // Given: the unchanged distinct-key inputs with a getter-backed guard.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = format!("({rules});");
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("record fixture must be an expression");
    };
    let original = readable_code(&statement.expression);
    let prepared = prepare(&builder, &statement.expression, &statement.expression)
        .unwrap_or_else(|| panic!("distinct keys must retain guard captures"));
    assert_eq!(prepared.bindings.len(), 1);
    assert_eq!(readable_code(&statement.expression), original);
    let setup = format!(
        "Object.defineProperty(globalThis, 'on', {{ get() {{ trace.push('guard'); return {enabled}; }} }})"
    );
    // When: evaluate the prepared record with either guard outcome.
    let result = evaluate_record(rules, &setup);
    // Then: the guard is read once and retained union-key order/value is stable.
    assert_eq!(
        result,
        if enabled {
            r#"[["color","p"],{"color":"red","p":1},["guard"]]"#
        } else {
            r#"[["color","p"],{"p":1},["guard"]]"#
        }
    );
}

/// Evaluate the emitted render scope, lowering its empty native or bound JSX element.
fn evaluate_native_render(code: &str, observation: &str) -> String {
    use oxc_ast::ast::{
        ImportDeclaration, ImportDeclarationSpecifier, JSXAttributeItem, JSXAttributeName,
        JSXAttributeValue, JSXElement, JSXElementName,
    };
    use oxc_ast_visit::Visit;
    use oxc_span::Span;

    #[derive(Default)]
    struct NativeElements(Vec<(Span, String)>);

    impl<'a> Visit<'a> for NativeElements {
        fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
            let replacement = import
                .specifiers
                .iter()
                .flatten()
                .filter_map(|specifier| match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(named)
                        if import.source.value == "react"
                            && named.imported.to_string() == "forwardRef" =>
                    {
                        Some(format!("const {}=render=>({{render}});", named.local.name))
                    }
                    _ => None,
                })
                .collect::<String>();
            self.0.push((import.span, replacement));
        }

        fn visit_jsx_element(&mut self, element: &JSXElement<'a>) {
            assert_eq!(element.children.len(), 0);
            let tag = match &element.opening_element.name {
                JSXElementName::Identifier(tag) => {
                    assert_eq!(tag.name, "div");
                    "'div'"
                }
                JSXElementName::IdentifierReference(tag) => {
                    assert!(["DevupStyled", "DevupAs"].contains(&tag.name.as_str()));
                    tag.name.as_str()
                }
                _ => panic!("fixture must render a native element"),
            };
            let props: Vec<_> = element
                .opening_element
                .attributes
                .iter()
                .map(|attribute| match attribute {
                    JSXAttributeItem::SpreadAttribute(spread) => {
                        format!("...{}", readable_code(&spread.argument))
                    }
                    JSXAttributeItem::Attribute(attribute) => {
                        let JSXAttributeName::Identifier(name) = &attribute.name else {
                            panic!("fixture must have identifier attributes");
                        };
                        let Some(JSXAttributeValue::ExpressionContainer(value)) = &attribute.value
                        else {
                            panic!("fixture must have expression attributes");
                        };
                        let value = match value.expression.as_expression() {
                            Some(value) => value,
                            None => panic!("fixture attribute must contain an expression"),
                        };
                        format!("{}:({})", name.name, readable_code(value))
                    }
                })
                .collect();
            self.0.push((
                element.span,
                format!("({{type:{tag},props:{{{}}}}})", props.join(",")),
            ));
        }
    }

    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut elements = NativeElements::default();
    elements.visit_program(&parsed.program);
    elements
        .0
        .sort_unstable_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut script = code.to_string();
    for (span, replacement) in elements.0 {
        let (start, end) = match (usize::try_from(span.start), usize::try_from(span.end)) {
            (Ok(start), Ok(end)) => (start, end),
            _ => panic!("fixture spans must fit native indices"),
        };
        script.replace_range(start..end, &replacement);
    }
    script.push_str(observation);
    let mut context = Context::default();
    let result = match context.eval(Source::from_bytes(script.as_bytes())) {
        Ok(result) => result,
        Err(error) => panic!("native render evaluation failed: {error}"),
    };
    match result.as_string() {
        Some(result) => result.to_std_string_escaped(),
        None => panic!("native observation must return JSON"),
    }
}

#[rstest]
#[serial_test::serial]
fn captures_and_filter_survive_when_prepared_base_retargets_then_extends(
    #[values("test.tsx", "test.ts")] filename: &str,
    #[values(false, true)] on: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: conditional captures, prop-reading attrs and an inherited filter.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{styled}} from '@devup-ui/react';const trace=[];const on={on};const Base=styled.div.attrs(p=>({{'data-tone':p.tone}})).withConfig({{shouldForwardProp:p=>p!=='tone'&&p!=='blocked'}})({{color:'red',...((trace.push('base'),on)?{{color:'blue'}}:{{}})}});const Mid=Base.withComponent('section');const Card=styled(Mid)({{p:1}});"
    );
    let output = match crate::extract(
        filename,
        &source,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    ) {
        Ok(output) => output,
        Err(error) => panic!("prepared filter extraction failed: {error}"),
    };
    let color_classes: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(value)
                if value.property() == "color"
                    && value.value() == if on { "blue" } else { "red" } =>
            {
                match style.extract(if single_css { None } else { Some(filename) }) {
                    Some(crate::extract_style::style_property::StyleProperty::ClassName(name)) => {
                        Some(name)
                    }
                    _ => panic!("selected color must have a class"),
                }
            }
            _ => None,
        })
        .collect();
    assert_ne!(color_classes.len(), 0);
    let winning = serde_json::to_string(&color_classes)
        .unwrap_or_else(|error| panic!("class fixture serialization failed: {error}"));
    // When: the emitted extension renders with distinct input, selected target and forwarded target.
    let observation = format!(
        "const __devupJsx0=(type,props)=>({{type,props}});const rendered=Card.render({{tone:'olive',blocked:9,id:'card',ref:99,children:'child',className:'caller',as:'article',forwardedAs:'nav'}},7);JSON.stringify([rendered.type,rendered.props.as,rendered.props.id,rendered.props.ref,rendered.props.children,rendered.props.className.includes('caller'),{winning}.some(c=>rendered.props.className.split(' ').includes(c)),rendered.props['data-tone'],'tone' in rendered.props,'blocked' in rendered.props,trace]);"
    );
    let result = evaluate_native_render(&output.code, &observation);
    // Then: captures run once, attrs reads survive, filtered props disappear and identity/ordinary props remain.
    assert_eq!(
        result,
        r#"["article","nav","card",7,"child",true,true,"olive",false,false,["base"]]"#
    );
}

#[rstest]
#[serial_test::serial]
fn template_key_precedence_when_public_component_renders(
    #[values(false, true)] single_css: bool,
    #[values(false, true)] on: bool,
) {
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react'; const trace=[]; const on={on}; const Card=styled('div',{{...((trace.push('spread'),on)?{{[`color`]:(trace.push('leaf'),'red')}}:{{color:'purple'}}),[`color`]:'orange'}}); const rendered=Card.render({{id:'card'}},null);"
    );
    let output = match crate::extract(
        "test.tsx",
        &code,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    ) {
        Ok(output) => output,
        Err(error) => panic!("static template key extraction failed: {error}"),
    };
    let values: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::ExtractStyleValue::Static(style) => (style.property(), style.value()),
            _ => panic!("winning template key must remain static"),
        })
        .collect();
    assert_eq!(values, vec![("color", "orange")]);
    let rendered = evaluate_native_render(
        &output.code,
        "JSON.stringify([rendered.type,rendered.props.id,trace]);",
    );
    assert_eq!(
        rendered,
        if on {
            r#"["div","card",["spread","leaf"]]"#
        } else {
            r#"["div","card",["spread"]]"#
        }
    );
}

#[rstest]
#[case::padding(("p", "+2", "padding", "8px"))]
#[case::margin(("m", "-2", "margin", "-8px"))]
#[case::quoted_padding(("p", "+'2'", "padding", "8px"))]
#[case::template_margin(("m", "-`2`", "margin", "-8px"))]
#[case::interpolated_padding(("p", "`${2}`", "padding", "8px"))]
#[serial_test::serial]
fn signed_shorthand_when_public_record_is_prepared(
    #[case] shorthand: (&str, &str, &str, &str),
    #[values(false, true)] single_css: bool,
) {
    let (property, literal, css_property, expected) = shorthand;
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react'; const Card=styled.div({{...(on?{{color:'red'}}:{{}}),color:'orange',{property}:{literal}}});"
    );
    let output = match crate::extract(
        "test.tsx",
        &code,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    ) {
        Ok(output) => output,
        Err(error) => panic!("signed shorthand extraction failed: {error}"),
    };
    let mut values: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::ExtractStyleValue::Static(style) => (style.property(), style.value()),
            _ => panic!("literal shorthand must not become a dynamic variable"),
        })
        .collect();
    values.sort_unstable();
    let mut expected = vec![("color", "orange"), (css_property, expected)];
    expected.sort_unstable();
    assert_eq!(values, expected);
}

#[rstest]
#[serial_test::serial]
fn authored_composed_class_when_public_component_renders(
    #[values(false, true)] single_css: bool,
    #[values(false, true)] on: bool,
) {
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react'; function make(on,__devupRecord0){{const Card=styled.div(__devupRecord0,{{...(on?{{color:'red'}}:{{}}),color:'orange'}});return Card.render({{id:'card',className:'caller-class'}},null);}} const rendered=make({on},'authored-class');"
    );
    let output = match crate::extract(
        "test.tsx",
        &code,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    ) {
        Ok(output) => output,
        Err(error) => panic!("composed class extraction failed: {error}"),
    };
    let rendered = evaluate_native_render(
        &output.code,
        "JSON.stringify([rendered.type,rendered.props.id,rendered.props.className.split(' ').filter(name=>name==='authored-class'||name==='caller-class')]);",
    );
    assert_eq!(
        rendered,
        r#"["div","card",["authored-class","caller-class"]]"#
    );
}
