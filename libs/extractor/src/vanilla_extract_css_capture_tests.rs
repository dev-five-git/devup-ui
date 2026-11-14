use super::{Values, validate};
use crate::{ExtractOption, extract_without_source_map};
use crate::{own_css_plan::Plan, vanilla_extract::Collector};
use boa_engine::{Context, Source};
use rustc_hash::{FxHashMap, FxHashSet};
use serial_test::serial;
use std::{cell::RefCell, rc::Rc};

#[test]
fn capture_keeps_numeric_data_keys_and_scalar_declarations() -> Result<(), String> {
    // Given: a numeric key belongs to ignored props, not a CSS declaration.
    let names = FxHashMap::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut values = Values {
        names: &names,
        collector: &collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: Plan::default(),
    };
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes("({props:{0:'data'},color:'red'})"))
        .map_err(|error| error.to_string())?;
    // When
    let root = values
        .capture(&value, &mut context)
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(
        values.plan.code(root),
        r#"{"props":{"0":"data"},"color":"red"}"#
    );
    assert_eq!(validate(&values.plan, root), vec![]);
    Ok(())
}

#[test]
fn capture_rejects_nested_responsive_arrays_before_lowering() -> Result<(), String> {
    // Given
    let names = FxHashMap::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut values = Values {
        names: &names,
        collector: &collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: Plan::default(),
    };
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes("({p:[[1,2]]})"))
        .map_err(|error| error.to_string())?;
    // When
    let error = values
        .capture(&value, &mut context)
        .err()
        .ok_or("nested array accepted")?;
    // Then
    assert!(error.to_string().contains(crate::utils::RESPONSIVE_ARRAY));
    Ok(())
}

#[rstest::rstest]
#[case("({_hover:[{p:[1,2]}]})", "_hover -> 0 -> p")]
#[case(concat!("({_hover:[{_hover:[{", "p:2}]}]})"), "_hover -> 0 -> _hover")]
fn capture_validation_rejects_responsive_arrays_across_entry_objects(
    #[case] source: &str,
    #[case] path: &str,
) -> Result<(), String> {
    // Given: real capture resets array context at each property of an entry object.
    let names = FxHashMap::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut values = Values {
        names: &names,
        collector: &collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: Plan::default(),
    };
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes(source))
        .map_err(|error| error.to_string())?;
    let root = values
        .capture(&value, &mut context)
        .map_err(|error| error.to_string())?;
    // When
    let errors = validate(&values.plan, root);
    // Then
    assert_eq!(errors, vec![(path.into(), crate::utils::RESPONSIVE_ARRAY)]);
    Ok(())
}

#[rstest::rstest]
#[case(("_hover", "'not-a-list'"))]
#[case(("_hover", "5"))]
#[case(("_hover", "true"))]
#[case(("_hover", "{}"))]
#[case(("_hover", "()=>0"))]
#[case(("_hover", "Infinity"))]
#[case(("_hover", "1n"))]
#[case(("_is", "['a',()=>0]"))]
#[case(("_is", "null"))]
#[case(("_is", "undefined"))]
#[case(("_is", "[]"))]
#[case(("_is", "['']"))]
#[case(("_is", "['a',{}]"))]
#[case(("_is", "['a',[]]"))]
#[case(("_is", "['a',undefined]"))]
#[case(("_is", "['a',null]"))]
#[case(("_is", "['a',1n]"))]
#[case(("_is", "['a',Infinity]"))]
#[case(("_is", "['a',Object.defineProperty({},'payload',{enumerable:true,get(){throw Error('descendant')}})]"))]
#[case(("_is", "['a',Object.defineProperty([],0,{enumerable:true,get(){throw Error('descendant')}})]"))]
#[case(("_is", "['a',(()=>{const entry={};entry.self=entry;return entry})()]"))]
#[case(("_is", "['a',(()=>{const entry=[];entry.push(entry);return entry})()]"))]
#[serial]
fn bridge_computed_invalid_params_are_terminal_at_the_call(
    #[case] fixture: (&str, &str),
    #[values(false, true)] caught: bool,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: neither a reference nor a computed call has authored leaf provenance.
    let _state = crate::test_state::TestStateGuard::new();
    let (selector, value) = fixture;
    let expression = if caught { "computed()" } else { "value" };
    let body = format!("return css({{props:data,{selector}:{{params:{expression},p:2}}}})");
    let helper = if caught {
        format!("function make(){{try{{{body}}}catch{{return css({{p:9}})}}}}")
    } else {
        format!("function make(){{{body}}}")
    };
    let source = format!(
        "import{{css}}from'@devup-ui/react';\nlet data=0;for(let i=0;i<16;i++)data={{payload:data}};const value={value};function computed(){{return value}}\n{helper}\nexport const card=make();"
    );
    let column = helper
        .find("css({props")
        .ok_or("missing real styling call")?
        + 1;
    // When
    let error = extract_without_source_map(
        "/computed-params.css.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .err()
    .ok_or("computed invalid params accepted")?
    .to_string();
    // Then: a surrounding catch cannot consume the sticky metadata refusal.
    assert!(
        error.starts_with(&format!("/computed-params.css.ts:3:{column}:")),
        "{error}"
    );
    assert!(
        error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
        "{error}"
    );
    Ok(())
}

fn compile_params(body: &str) -> Result<crate::ExtractOutput, String> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    extract_without_source_map(
        "/capture-params.css.ts",
        &format!("import{{css}}from'@devup-ui/react';{body}"),
        ExtractOption::default(),
    )
    .map_err(|error| error.to_string())
}

#[rstest::rstest]
#[case("_groupNthChild", "[]", "[]")]
#[case("_groupNthChild", "['']", "''")]
#[case("selectors:{\"&:nth-child[title='옳]:not']\"", "[]", "[]")]
#[case("selectors:{\"&:nth-child[title='옳]:not']\"", "['']", "''")]
#[serial]
fn bridge_nested_targets_keep_authored_empty_locations(
    #[case] target: &str,
    #[case] params: &str,
    #[case] marker: &str,
) -> Result<(), String> {
    // Given
    let closing = if target.starts_with("selectors:") {
        "}"
    } else {
        ""
    };
    let body = format!(
        "let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card=css({{props:data,_hover:{{{target}:{{params:{params},p:2}}{closing}}}}});"
    );
    let source = format!("import{{css}}from'@devup-ui/react';{body}");
    let offset = source
        .find(marker)
        .ok_or("missing authored empty witness")?;
    let location = crate::locate("/capture-params.css.ts", &source, offset);
    // When
    let error = compile_params(&body)
        .err()
        .ok_or("nested required empty params accepted")?;
    // Then
    assert!(error.starts_with(&format!("{location}:")), "{error}");
    assert!(
        error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
        "{error}"
    );
    Ok(())
}

#[test]
#[serial]
fn bridge_refuses_missing_params_bindings() -> Result<(), String> {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let body = concat!(
        "let data=0;for(let i=0;i<16;i++)data={",
        "payload:data};export const card=css({props:data,_hover:{params:MISSING,p:2}});"
    );
    // When
    let error = compile_params(body)
        .err()
        .ok_or("MISSING normalized to absent params")?;
    // Then
    assert!(error.starts_with("/capture-params.css.ts:"), "{error}");
    assert!(error.contains("MISSING"), "{error}");
    Ok(())
}

#[rstest::rstest]
#[case("_hover", "[]", ":hover:hover")]
#[case("_nthChild", "['2n']", ":nth-child(2n):hover")]
#[serial]
fn bridge_deep_optional_empty_params_match_flat_selector(
    #[case] outer: &str,
    #[case] params: &str,
    #[case] repeated: &str,
) -> Result<(), String> {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let selector = format!("&{}", repeated.repeat(4096));
    let expected = compile_params(&format!(
        "export const card=css({{selectors:{{'{selector}':{{p:2}}}}}});"
    ))?;
    let body = format!(
        "let rules={{p:2}};for(let i=0;i<4096;i++)rules={{{outer}:{{params:{params},_hover:{{params:[],...rules}}}}}};export const card=css(rules);"
    );
    // When
    let actual = compile_params(&body)?;
    // Then
    assert_eq!(actual.styles, expected.styles);
    assert_eq!(actual.code, expected.code);
    Ok(())
}

#[rstest::rstest]
#[case(
    "({params:undefined,props:undefined})",
    r#"{"params":void 0,"props":undefined}"#,
    true
)]
#[case(
    "({props:{params:undefined,array:[,]}})",
    r#"{"props":{"params":undefined,"array":[undefined]}}"#,
    true
)]
#[case(
    "({props:{params:new Proxy([,],{has(){throw Error('unexpected has')}})}})",
    r#"{"props":{"params":[undefined]}}"#,
    true
)]
#[case(
    "({_is:{params:Object.setPrototypeOf(['a',,'b'],{1:'x'})}})",
    r#"{"_is":{"params":["a","x","b"]}}"#,
    true
)]
#[case(
    concat!("({_is:{params:Object.setPrototypeOf(['a',,'b'],{", "1:undefined})}})"),
    r#"{"_is":{"params":["a",undefined,"b"]}}"#,
    false
)]
#[case(
    "({_is:{params:Object.setPrototypeOf(['a',,'b'],{})}})",
    r#"{"_is":{"params":["a",,"b"]}}"#,
    true
)]
#[case(
    "({_is:{params:new Proxy(['a'],{has(){throw Error('unexpected has')}})}})",
    r#"{"_is":{"params":["a"]}}"#,
    true
)]
#[case(
    "({_is:{params:['a',,'b']}})",
    r#"{"_is":{"params":["a",,"b"]}}"#,
    true
)]
#[case("({_is:{params:['a',,]}})", r#"{"_is":{"params":["a",,]}}"#, true)]
#[case(
    "({_is:{params:['a',undefined,'b']}})",
    r#"{"_is":{"params":["a",undefined,"b"]}}"#,
    false
)]
#[case(
    "(Array.isArray=()=>false,({_is:{params:new Proxy(['a'],{})}}))",
    r#"{"_is":{"params":["a"]}}"#,
    true
)]
#[case(
    "({_is:{params:new Proxy(new Proxy(['a'],{}),{})}})",
    r#"{"_is":{"params":["a"]}}"#,
    true
)]
fn capture_preserves_sparse_presence_and_only_normalizes_params_absence(
    #[case] source: &str,
    #[case] code: &str,
    #[case] accepted: bool,
) -> Result<(), String> {
    // Given
    let names = FxHashMap::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut values = Values {
        names: &names,
        collector: &collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: Plan::default(),
    };
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes(source))
        .map_err(|error| error.to_string())?;
    // When
    let root = values
        .capture(&value, &mut context)
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(values.plan.code(root), code);
    let expected = if accepted {
        vec![]
    } else {
        vec![(
            "_is -> params".to_string(),
            crate::extractor::selector_params::SELECTOR_PARAMS,
        )]
    };
    assert_eq!(validate(&values.plan, root), expected);
    Ok(())
}

#[rstest::rstest]
#[serial]
fn revoked_proxy_array_is_a_located_build_error(
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    let _state = crate::test_state::TestStateGuard::new();
    let body = "const handle=Proxy.revocable(['a'],{});handle.revoke();export const card=css({_is:{params:handle.proxy,p:2}});";
    let source = format!("import{{css}}from'@devup-ui/react';\n{body}");
    let column = body.find("css(").ok_or("missing styling call")? + 1;
    let error = extract_without_source_map(
        "/revoked-proxy.css.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .err()
    .ok_or("revoked Proxy accepted")?
    .to_string();
    assert!(
        error.starts_with(&format!("/revoked-proxy.css.ts:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("cannot use `"), "{error}");
    Ok(())
}

#[rstest::rstest]
#[case(crate::own_css_plan::Node::Object(Vec::new()))]
#[case(crate::own_css_plan::Node::Array(Vec::new()))]
fn bridge_lowering_refuses_containers_without_scheduling_their_declarations(
    #[case] container: crate::own_css_plan::Node,
) -> Result<(), String> {
    // Given: exercise the Plan adapter independently of the capture-time refusal.
    use crate::own_css_plan::{Node, Property};
    let plan = Plan {
        nodes: vec![
            Node::Object(vec![Property {
                name: "_nthChild".into(),
                slot: 0,
                value: 1,
            }]),
            Node::Object(vec![
                Property {
                    name: "params".into(),
                    slot: 1,
                    value: 2,
                },
                Property {
                    name: "p".into(),
                    slot: 2,
                    value: 5,
                },
            ]),
            Node::Array(vec![3, 4]),
            Node::Scalar(3),
            container,
            Node::Scalar(4),
        ],
        slots: ["'_nthChild'", "'params'", "'p'", "'2n'", "2"]
            .map(String::from)
            .into(),
        roots: vec![0],
        ..Plan::default()
    };
    let arena = oxc_allocator::Allocator::default();
    let ast = oxc_ast::builder::AstBuilder::new(&arena);
    let slots = plan
        .slots
        .iter()
        .map(|code| {
            oxc_parser::Parser::new(&arena, code, oxc_span::SourceType::mjs())
                .parse_expression()
                .map_err(|error| format!("{error:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // When
    let result = crate::own_css_lower::RuleCompiler {
        ast: &ast,
        plan: &plan,
        slots: &slots,
    }
    .lower(0);
    // Then
    assert_eq!(result.styles.len(), 1);
    let mut errors = Vec::new();
    crate::utils::unreadable_styles(&result.styles, false, &mut errors);
    assert_eq!(
        errors,
        vec![(
            0,
            "params".to_string(),
            Some(crate::extractor::selector_params::SELECTOR_PARAMS)
        )]
    );
    Ok(())
}

#[rstest::rstest]
#[case(
    "Object.defineProperty({},'payload',{enumerable:true,get(){reads++;throw Error('descendant')}})"
)]
#[case("Object.defineProperty([],0,{enumerable:true,get(){reads++;throw Error('descendant')}})")]
#[case("(()=>{const entry={};entry.self=entry;return entry})()")]
#[case("(()=>{const entry=[];entry.push(entry);return entry})()")]
fn capture_refuses_params_containers_before_descendants_or_later_indices(
    #[case] entry: &str,
) -> Result<(), String> {
    // Given: both the rejected entry and its following index can expose a read.
    let names = FxHashMap::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    let location = "/early-params.css.ts:3:7";
    let mut values = Values {
        names: &names,
        collector: &collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: Plan {
            origin: Some((location.into(), "css".into())),
            ..Plan::default()
        },
    };
    let mut context = Context::default();
    let source = format!(
        "let reads=0;const params=[{entry}];Object.defineProperty(params,1,{{get(){{reads++;throw Error('later')}}}});({{_is:{{params,p:2}}}})"
    );
    let value = context
        .eval(Source::from_bytes(&source))
        .map_err(|error| error.to_string())?;
    // When
    let result = values.capture(&value, &mut context);
    // Then: the params refusal wins without entering either container or the next index.
    assert!(result.is_err());
    let reads = context
        .eval(Source::from_bytes("reads"))
        .map_err(|error| error.to_string())?;
    assert_eq!(reads.as_number(), Some(0.0));
    assert_eq!(
        collector.borrow().declaration_errors,
        vec![format!(
            "{location}: {}",
            crate::utils::build_time_error(
                "css",
                "params",
                crate::extractor::selector_params::SELECTOR_PARAMS
            )
        )]
    );
    Ok(())
}

#[rstest::rstest]
#[case((":is([role=group],[data-group]):hover &", ":is([role=group],[data-group]):hover ", ""))]
#[case(("&[title='옳]:not']:host", "", "[title='옳]:not']:host"))]
#[serial]
fn bridge_complex_empty_contexts_match_independent_flat_selectors(
    #[case] shape: (&str, &str, &str),
    #[values(false, true)] list: bool,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: distinct grouped or quoted prefixes, with a separately branched list parent.
    let _state = crate::test_state::TestStateGuard::new();
    let (template, before, after) = shape;
    let before = before.repeat(4096);
    let after = after.repeat(4096);
    let selectors = if list {
        format!("{before}&:hover{after},{before}&:host{after}")
    } else {
        format!("{before}&{after}")
    };
    let flat = format!(
        "export const card=css({{selectors:{{{}:{{p:2}}}}}});",
        crate::vanilla_extract::json_string(&selectors)
    );
    let compile = |body: &str| {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        extract_without_source_map(
            "/complex-params.css.ts",
            &format!("import{{css}}from'@devup-ui/react';{body}"),
            ExtractOption {
                single_css,
                ..ExtractOption::default()
            },
        )
        .map_err(|error| error.to_string())
    };
    let expected = compile(&flat)?;
    let wrap = if list {
        "rules={selectors:{'&:hover,&:host':rules}};"
    } else {
        ""
    };
    let source = format!(
        "let rules={{p:2}};const template={};for(let i=0;i<4096;i++)rules={{selectors:{{[template]:{{params:[],...rules}}}}}};{wrap}export const card=css(rules);",
        crate::vanilla_extract::json_string(template)
    );
    // When
    let actual = compile(&source)?;
    // Then
    assert_eq!(actual.styles, expected.styles);
    assert_eq!(actual.code, expected.code);
    Ok(())
}

#[rstest::rstest]
#[case("{1:'x'}", Some("&:is(a,x,b)"))]
#[case(concat!("{", "1:undefined}"), None)]
#[case("{}", Some("&:is(a,b)"))]
#[serial]
fn bridge_inherited_params_presence_uses_native_reads_and_call_provenance(
    #[case] prototype: &str,
    #[case] expected: Option<&str>,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: only the inherited index's presence or value changes.
    let _state = crate::test_state::TestStateGuard::new();
    let rule = "export const card=css({props:data,_is:{params:entries,p:2}});";
    let source = format!(
        "import{{css}}from'@devup-ui/react';\nlet data=0;for(let i=0;i<16;i++)data={{payload:data}};const entries=Object.setPrototypeOf(['a',,'b'],{prototype});\n{rule}"
    );
    // When
    let result = extract_without_source_map(
        "/presence-params.css.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string());
    // Then
    if let Some(expected) = expected {
        let output = result?;
        let selectors: Vec<_> = output
            .styles
            .iter()
            .filter_map(|style| match style {
                crate::ExtractStyleValue::Static(style) => {
                    style.selector.as_ref().map(ToString::to_string)
                }
                _ => None,
            })
            .collect();
        assert_eq!(selectors, vec![expected]);
    } else {
        let error = result.err().ok_or("inherited present undefined accepted")?;
        let column = rule.find("css(").ok_or("missing styling call")? + 1;
        assert!(
            error.starts_with(&format!("/presence-params.css.ts:3:{column}:")),
            "{error}"
        );
        assert!(
            error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
            "{error}"
        );
    }
    Ok(())
}

#[rstest::rstest]
#[case(&["&:nth-child", "&:hover"], true)]
#[case(&["&:nth-child", "=2n", "&:hover"], false)]
#[case(&["&:nth-child:nth-of-type", "=2n"], true)]
#[case(&["&:nth-child:nth-of-type", "=2n", "=odd"], false)]
#[case(&["&:nth-child", ":is(&)"], false)]
#[case(&["&:hover", ":is([role=group],[data-group]):nth-child &"], true)]
#[case(&["&:nth-child", "&[title='옳]:not']:host"], false)]
#[case(&["&:nth", "&-child > a"], true)]
#[case(&["&:nth-child,&:host", "&:hover"], true)]
fn params_target_summary_matches_canonical_nesting(
    #[case] operations: &[&str],
    #[case] required: bool,
) {
    let mut contexts = crate::own_css_context::Contexts::default();
    let mut path = None;
    for operation in operations {
        let frame = operation.strip_prefix('=').map_or_else(
            || crate::own_css_context::Frame::Selector((*operation).to_string()),
            |params| crate::own_css_context::Frame::Params(params.to_string()),
        );
        path = Some(contexts.push(path, frame));
    }
    assert_eq!(contexts.requires_argument(path), required);
}

#[test]
fn fresh_target_restores_authored_requiredness_after_unknown_params() {
    let mut contexts = crate::own_css_context::Contexts::default();
    let outer = contexts.push(
        None,
        crate::own_css_context::Frame::Selector("&:nth-child".into()),
    );
    let unknown = contexts.uncertain(Some(outer));
    assert!(!contexts.requires_argument(Some(unknown)));
    let child = contexts.push(
        Some(unknown),
        crate::own_css_context::Frame::Selector("&:nth-child".into()),
    );
    assert!(contexts.requires_argument(Some(child)));
}

#[rstest::rstest]
#[case("['color:', ';']", false)]
#[case("['', '']", true)]
fn evaluated_template_placement_owns_params_validation(
    #[case] raw: &str,
    #[case] statement: bool,
) -> Result<(), String> {
    let collector = Rc::new(RefCell::new(Collector {
        origin: Some(("/placement.css.ts:2:5".into(), "css".into())),
        ..Collector::default()
    }));
    let mut context = Context::default();
    let parts = context
        .eval(Source::from_bytes(&format!(
            "Object.assign({raw},{{raw:{raw}}})"
        )))
        .map_err(|error| error.to_string())?;
    let value = context
        .eval(Source::from_bytes(concat!(
            "let data=0;for(let i=0;i<16;i++)data={",
            "payload:data};({props:data,_is:{params:[],p:2}})"
        )))
        .map_err(|error| error.to_string())?;
    super::super::super::call(&collector, &[parts, value], &mut context)
        .map_err(|error| error.to_string())?;
    let collector = collector.borrow();
    assert_eq!(collector.styles.styles.len(), 1);
    if statement {
        assert!(
            collector
                .declaration_errors
                .iter()
                .any(|error| error.contains(crate::extractor::selector_params::SELECTOR_PARAMS))
        );
    } else {
        assert_eq!(collector.declaration_errors, Vec::<String>::new());
    }
    Ok(())
}

#[test]
fn capture_preserves_root_and_nested_array_order_with_once_only_getters() -> Result<(), String> {
    // Given: native array getters expose both repeated reads and traversal order.
    let names = FxHashMap::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut values = Values {
        names: &names,
        collector: &collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: Plan::default(),
    };
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes(concat!(
            "let reads=[];const nested=Object.defineProperties([],{",
            "0:{get(){reads.push('nested:0');return 2}},",
            "1:{get(){reads.push('nested:1');return 'last'}}});",
            "Object.defineProperties([],{",
            "0:{get(){reads.push('root:0');return 'first'}},",
            "1:{get(){reads.push('root:1');return nested}},",
            "2:{get(){reads.push('root:2');return null}}})"
        )))
        .map_err(|error| error.to_string())?;
    // When
    let root = values
        .capture(&value, &mut context)
        .map_err(|error| error.to_string())?;
    // Then: the array graph and values survive without duplicate getter evaluation.
    assert_eq!(values.plan.code(root), r#"["first",[2,"last"],null]"#);
    let reads = context
        .eval(Source::from_bytes("reads.join(',')"))
        .map_err(|error| error.to_string())?;
    assert_eq!(
        reads.as_string().map(|text| text.to_std_string_escaped()),
        Some("root:0,root:1,root:2,nested:0,nested:1".into())
    );
    Ok(())
}

#[rstest::rstest]
#[case("{}", false)]
#[case("'not-a-list'", false)]
#[case("['2n']", true)]
#[serial]
fn bridge_rejects_invalid_params_roots_inside_data_named_layers(
    #[case] value: &str,
    #[case] accepted: bool,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: props is a layer name, and its captured descendants retain their real node shapes.
    let _state = crate::test_state::TestStateGuard::new();
    let rule = "export const card=css(rules);";
    let source = format!(
        "import{{css}}from'@devup-ui/react';\nlet data=0;for(let i=0;i<16;i++)data={{payload:data}};const rules={{props:data,'@layer':{{props:{{_nthChild:{{params:{value},p:2}}}}}}}};\n{rule}"
    );
    let column = rule.find("css(").ok_or("missing real css call")? + 1;
    // When
    let result = extract_without_source_map(
        "/layer-params.css.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then
    if accepted {
        let output = result.map_err(|error| error.to_string())?;
        assert_eq!(output.styles.len(), 2);
        assert!(
            output.styles.iter().any(|entry| matches!(entry, crate::ExtractStyleValue::Static(style)
                if style.property() == "padding" && style.value() == "8px"
                    && style.layer.as_deref() == Some("props")
                    && style.selector.as_ref().is_some_and(|selector| selector.to_string() == "&:nth-child(2n)")))
        );
        assert!(output.styles.iter().any(|entry| matches!(entry,
            crate::ExtractStyleValue::LayerOrder(order)
                if order.file == "/layer-params.css.ts" && order.layers == vec!["props".to_string()]
        )));
    } else {
        let error = result
            .err()
            .ok_or("invalid params root scheduled a declaration")?
            .to_string();
        assert!(
            error.starts_with(&format!("/layer-params.css.ts:3:{column}:")),
            "{error}"
        );
        assert!(
            error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
            "{error}"
        );
    }
    Ok(())
}
