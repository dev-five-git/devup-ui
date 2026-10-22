use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use serial_test::serial;

fn compile(source: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    extract_without_source_map("/deep-object.css.ts", source, ExtractOption::default())
        .map_err(|error| error.to_string())
}

#[test]
#[serial]
fn deep_object_mixed_wrappers_keep_every_context_when_heap_built() -> Result<(), String> {
    // Given: shallow source with alternating non-foldable wrappers and selector kinds.
    let source = "import{css}from'@devup-ui/react';let rules={p:[2,null,3]};for(let i=0;i<12000;i++){rules={_hover:{'@supports (display: grid)':{'@container (min-width: 1px)':{selectors:{'&:focus':rules}}}}}}export const card=css(rules);";
    // When
    let output = compile(source)?;
    // Then
    assert_eq!(output.styles.len(), 2);
    for value in &output.styles {
        let ExtractStyleValue::Static(style) = value else {
            return Err("mixed rules must stay static".into());
        };
        let selector = style.selector().ok_or("context must survive")?;
        let css::style_selector::StyleSelector::At {
            outer, selector, ..
        } = selector
        else {
            return Err("at-rule wrappers must survive".into());
        };
        assert_eq!(outer.len(), 23_999);
        let selector = selector.as_deref().ok_or("pseudo selectors must survive")?;
        assert_eq!(selector.matches(":hover").count(), 12_000);
        assert_eq!(selector.matches(":focus").count(), 12_000);
        assert_eq!(style.property(), "padding");
        assert!(matches!(style.value(), "8px" | "12px"));
    }
    assert_eq!(output.code.matches("rules").count(), 0);
    Ok(())
}

#[test]
#[serial]
fn deep_object_ignored_data_is_captured_without_nested_generated_syntax() -> Result<(), String> {
    // Given
    let expected = compile(concat!(
        "import{",
        "css}from'@devup-ui/react';export const card=css({p:2});"
    ))?;
    let source = "import{css}from'@devup-ui/react';let data=1;for(let i=0;i<100000;i++){data={payload:data}}export const card=css({props:data,p:2});";
    // When
    let output = compile(source)?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[rstest::rstest]
#[case("css({p:[1,2],_hover:{p:[1,2],color:'red'}},{p:[3],_hover:{p:[3],w:2}})")]
#[case(
    "css({selectors:{'&:hover, & > a':{_focus:{p:[2,null,3]}}},_media:{'(min-width:1px)':{color:'red'}}})"
)]
#[case(concat!("css({", "'@layer':{base:{_supports:{'(display:grid)':{p:2}}}},_print:{color:'blue'}})"))]
#[case(concat!("css({", "_nthChild:{params:['2n'],p:2,_hover:{w:3}}})"))]
#[case("css({_hover:[{p:1,color:'red'},null,{p:2}],opacity:0.5,styleOrder:2})")]
#[case("css({p:'$space',color:'$brand',_hover:{w:'$space'}})")]
#[case("css(base,{p:[3],selectors:{[`${base} &`]:{animation:`${fade} 1s`,color:'blue'}}})")]
#[serial]
fn deep_object_shallow_contexts_match_ordinary_compilation(
    #[case] expression: &str,
) -> Result<(), String> {
    // Given: independent ordinary extraction, not the evaluator's own output as oracle.
    let body = format!(
        "import{{css,keyframes}}from'@devup-ui/react';const base=css({{p:[1,2]}});const fade=keyframes({{from:{{opacity:0}},to:{{opacity:1}}}});export const card={expression};"
    );
    assert!(!crate::utils::is_vanilla_extract_file("/deep-object.tsx"));
    reset_class_map();
    reset_file_map();
    let mut expected =
        extract_without_source_map("/deep-object.tsx", &body, ExtractOption::default())
            .map_err(|error| error.to_string())?;
    assert_eq!(
        expected
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::LayerOrder(_))),
        expression.contains("'@layer'"),
    );
    expected.styles = expected
        .styles
        .into_iter()
        .map(|mut style| {
            if let ExtractStyleValue::LayerOrder(order) = &mut style {
                assert_eq!(order.file, "/deep-object.tsx");
                order.file = "/deep-object.css.ts".to_string();
            }
            style
        })
        .collect();
    // When
    let computed = format!("{body}(()=>1)();");
    assert_eq!(
        crate::stylesheet_policy::plan(
            &computed,
            "/deep-object.css.ts",
            &ExtractOption::default(),
            None,
            &|_| false,
        ),
        crate::stylesheet_policy::Plan::Run,
    );
    let output = compile(&computed)?;
    // Then
    let bindings = |code: &str| {
        code.lines()
            .filter(|line| line.starts_with("const ") || line.starts_with("export const "))
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    assert_eq!(bindings(&output.code), bindings(&expected.code));
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[rstest::rstest]
#[case(concat!("let value={", "};value.self=value;css({props:value});"))]
#[case("let value={};const a=[value];value.a=a;css(value);")]
#[case(
    "let value=()=>1;for(let i=0;i<100000;i++){value={payload:value}}try{css({props:value})}catch(e){}"
)]
#[serial]
fn deep_object_invalid_graphs_remain_terminal(#[case] body: &str) -> Result<(), String> {
    // Given
    let source =
        format!("import{{css}}from'@devup-ui/react';\n{body}export const card=css({{p:2}});");
    // When
    let error = compile(&source).err().ok_or("invalid graph must reject")?;
    // Then
    assert!(error.starts_with("/deep-object.css.ts:2:"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn deep_object_shared_occurrences_keep_lazy_objects_and_eager_arrays() -> Result<(), String> {
    // Given: array reads must finish before nested object getters; shared nodes revisit.
    let source = "import{css}from'@devup-ui/react';let trace='';const shared={get p(){trace+='p';return 2}};const parts=[shared,shared];Object.defineProperty(parts,0,{get(){trace+='0';return shared}});Object.defineProperty(parts,1,{get(){trace+='1';return shared}});const rules={get _hover(){trace+='h';return shared},get _focus(){trace+='f';return shared}};css(parts);export const card=css(rules);if(trace!=='01pphpfp')throw Error(trace);";
    // When
    let output = compile(source)?;
    // Then
    assert_eq!(output.styles.len(), 3);
    Ok(())
}
