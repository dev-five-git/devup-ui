use super::*;
use crate::extract_style::{extract_style_value::ExtractStyleValue, style_property::StyleProperty};
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case::object("({a:0.5,b:{nested:`0.${5}`}})", "a", "b")]
#[case::array("[0.5,[`0.${5}`]]", "0", "1")]
#[serial]
fn scalar_lookup_preserves_whole_values_when_candidates_include_containers(
    #[case] map: &str,
    #[case] first: &str,
    #[case] second: &str,
) {
    // Given: the selected value is scalar data, never a set of style rules.
    let source = format!(
        "{IMPORT}const trace=[];let enabled,key;const state={{get key(){{trace.push('key');return {{toString(){{trace.push('coerce');if(key==='throw')throw new Error('stop');return key;}}}};}}}};const Card=styled.div({{opacity:0.3,...((trace.push('guard'),enabled)?{{opacity:{map}[state.key]}}:{{}}),zIndex:(trace.push('later'),7)}});"
    );
    let original = source
        .strip_prefix(IMPORT)
        .unwrap_or_else(|| panic!("fixture must have its import"));
    let scenarios = serde_json::json!([
        [false, second],
        [true, first],
        [true, second],
        [true, "missing"],
        [true, "throw"]
    ])
    .to_string();
    let authored = format!(
        "const styled={{div:rules=>rules}};const results={scenarios}.map(([on,next])=>{{{original}enabled=on;key=next;trace.length=0;try{{const rules={{opacity:0.3,...((trace.push('guard'),enabled)?{{opacity:{map}[state.key]}}:{{}}),zIndex:(trace.push('later'),7)}};return [Object.entries(rules).filter(([,v])=>v!=null).map(([p,v])=>[p==='zIndex'?'z-index':p,typeof v==='number'?String(v):v]),trace];}}catch(e){{return [e.message,trace];}}}});JSON.stringify(results);"
    );
    // The producer's lazy renderer is compared with independent authored rule evaluation.
    let output = {
        reset_class_map();
        reset_file_map();
        extract(
            "scalar-lookup.ts",
            &source,
            ExtractOption {
                single_css: true,
                import_main_css: false,
                ..ExtractOption::default()
            },
        )
        .unwrap_or_else(|error| panic!("scalar extraction failed: {error}"))
    };
    let declarations: Vec<_> = output
        .styles
        .iter()
        .map(|style| match (style, style.extract(None)) {
            (ExtractStyleValue::Static(value), Some(StyleProperty::ClassName(name))) => {
                serde_json::json!([name, value.property(), value.value(), null])
            }
            (
                ExtractStyleValue::Dynamic(value),
                Some(StyleProperty::Variable {
                    class_name,
                    variable_name,
                    ..
                }),
            ) => serde_json::json!([class_name, value.property(), null, variable_name]),
            other => panic!("unexpected scalar declaration: {:?}", other.0),
        })
        .collect();
    let arena = Allocator::default();
    let mut program = parse(&arena, "scalar-lookup.ts", &output.code);
    let calls = renders(&program);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|s| !matches!(s, Statement::ImportDeclaration(_)));
    let generated = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{}const results={scenarios}.map(([on,next])=>{{enabled=on;key=next;trace.length=0;try{{const e=Card.render({{}},null);const active=new Set((e.props.className||'').split(/\\s+/));return [{declarations}.filter(([c])=>active.has(c)).map(([,p,v,k])=>[p,k?e.props.style[k]:v]).filter(([,v])=>v!=null).map(([p,v])=>[p,typeof v==='object'?v:String(Number(v))]).sort(([a],[b])=>a.localeCompare(b)),trace.slice()];}}catch(e){{return [e.message,trace.slice()];}}}});JSON.stringify(results);",
        Codegen::new().build(&program).code,
        declarations = serde_json::to_string(&declarations)
            .unwrap_or_else(|error| panic!("declarations must serialize: {error}"))
    );
    let observe = |script: &str| {
        Context::default()
            .eval(Source::from_bytes(script))
            .unwrap_or_else(|e| panic!("{e}\n{script}"))
            .as_string()
            .unwrap_or_else(|| panic!("observation must return JSON"))
            .to_std_string_escaped()
    };
    // When: all active declarations, coercions and throw-short-circuit effects are observed.
    let actual = observe(&generated);
    // Then: b must remain opacity data, and a missing key must omit it, not activate nested.
    assert_eq!(actual, observe(&authored), "{}", output.code);
    eprintln!(
        "SCALAR bytes={} actual={actual}\n{}",
        output.code.len(),
        output.code
    );
}

#[rstest]
#[case::underscore(("_hover:SELECTED", "&:hover"))]
#[case::pseudo(("':hover':SELECTED", "&:hover"))]
#[case::descendant(("'.child':SELECTED", "& .child"))]
#[case::selector_map(("selectors:{'.child':SELECTED}", "& .child"))]
#[serial]
fn finite_rule_lookup_keeps_objects_when_the_property_is_a_selector(#[case] fixture: (&str, &str)) {
    // Given: unlike opacity data, a hover value is explicitly a set of rules.
    reset_class_map();
    reset_file_map();
    let rules = fixture
        .0
        .replace("SELECTED", "({a:{color:'red'},b:{color:'blue'}})[key]");
    let source = format!("{IMPORT}let key;const Card=styled.div({{{rules}}});");
    // When: compile and execute both rule branches and a missing selection.
    let output = extract(
        "rule-lookup.ts",
        &source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("rule extraction failed: {error}"));
    let declarations = output
        .styles
        .iter()
        .map(|style| match (style, style.extract(None)) {
            (ExtractStyleValue::Static(value), Some(StyleProperty::ClassName(name))) => {
                serde_json::json!([
                    name,
                    value.property(),
                    value.value(),
                    value.selector().map(ToString::to_string)
                ])
            }
            other => panic!("rules must remain static: {:?}", other.0),
        })
        .collect::<Vec<_>>();
    let arena = Allocator::default();
    let mut program = parse(&arena, "rule-lookup.ts", &output.code);
    let calls = renders(&program);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|s| !matches!(s, Statement::ImportDeclaration(_)));
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{}JSON.stringify(['a','b','missing'].map(next=>{{key=next;const e=Card.render({{}},null);return {declarations}.filter(([c])=>e.props.className.split(' ').includes(c)).map(([,p,v,s])=>[p,v,s]);}}));",
        Codegen::new().build(&program).code,
        declarations = serde_json::to_string(&declarations)
            .unwrap_or_else(|error| panic!("declarations must serialize: {error}"))
    );
    let actual = Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("rule execution failed: {error}"))
        .as_string()
        .unwrap_or_else(|| panic!("rule observation must return JSON"))
        .to_std_string_escaped();
    // Then: finite rule objects remain supported without treating scalar objects as rules.
    assert_eq!(
        actual,
        serde_json::json!([
            [["color", "red", fixture.1]],
            [["color", "blue", fixture.1]],
            []
        ])
        .to_string()
    );
}
