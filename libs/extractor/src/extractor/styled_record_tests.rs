use boa_engine::{Context, Source};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast::{ast::Statement, builder::AstBuilder};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::{styled_record::prepare, styled_record_evaluation::bind_render};
use crate::utils::readable_code;

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
#[case::distinct("{ ...(on ? { color: 'red' } : {}), p: 1 }")]
#[case::computed_unaffected("{ ...(on ? { ['color']: 'red' } : {}), p: 1 }")]
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

/// Evaluate the emitted render scope, lowering only its empty native JSX element.
fn evaluate_native_render(code: &str, observation: &str) -> String {
    use oxc_ast::ast::{
        ImportDeclaration, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElement,
        JSXElementName,
    };
    use oxc_ast_visit::Visit;
    use oxc_span::Span;

    #[derive(Default)]
    struct NativeElements(Vec<(Span, String)>);

    impl<'a> Visit<'a> for NativeElements {
        fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
            self.0.push((import.span, String::new()));
        }

        fn visit_jsx_element(&mut self, element: &JSXElement<'a>) {
            assert_eq!(element.children.len(), 0);
            let JSXElementName::Identifier(tag) = &element.opening_element.name else {
                panic!("fixture must render a native element");
            };
            assert_eq!(tag.name, "div");
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
                format!("({{type:'div',props:{{{}}}}})", props.join(",")),
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
fn template_key_precedence_when_public_component_renders(
    #[values(false, true)] single_css: bool,
    #[values(false, true)] on: bool,
) {
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react'; const trace=[]; const on={on}; const Card=styled('div',{{...((trace.push('spread'),on)?{{[`color`]:(trace.push('leaf'),'red')}}:{{color:'purple'}}),[`color`]:'orange'}}); const rendered=Card({{id:'card'}});"
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
#[serial_test::serial]
#[case::padding(("p", "+2", "padding", "8px"))]
#[case::margin(("m", "-2", "margin", "-8px"))]
#[case::quoted_padding(("p", "+'2'", "padding", "8px"))]
#[case::template_margin(("m", "-`2`", "margin", "-8px"))]
#[case::interpolated_padding(("p", "`${2}`", "padding", "8px"))]
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
        "import {{styled}} from '@devup-ui/react'; function make(on,__devupRecord0){{const Card=styled.div(__devupRecord0,{{...(on?{{color:'red'}}:{{}}),color:'orange'}});return Card({{id:'card',className:'caller-class'}});}} const rendered=make({on},'authored-class');"
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
