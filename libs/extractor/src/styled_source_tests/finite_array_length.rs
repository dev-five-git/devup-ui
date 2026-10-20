use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case::ordinary(("[5,9]", &[("length",Some("2")),("0",Some("5")),("length",Some("2")),("1",Some("9")),("missing",None)][..]))]
#[case::sparse(("[5,,9,,]", &[("length",Some("4")),("2",Some("9")),("1",None),("length",Some("4")),("0",Some("5")),("4",None)][..]))]
#[case::empty(("[]", &[("length",Some("0")),("0",None),("length",Some("0")),("missing",None)][..]))]
#[case::elisions_only(("[,,]", &[("length",Some("2")),("0",None),("length",Some("2")),("1",None)][..]))]
#[serial]
fn static_z_index_when_array_own_length_and_element_keys_alternate(
    #[case] fixture: (&str, &[(&str, Option<&str>)]),
    #[values(false, true)] single_css: bool,
    #[values(false, true)] enabled: bool,
) {
    // Given: own length includes elisions; the fallback differs even from empty length.
    let (array, observations) = fixture;
    let setup = format!(
        "const trace=[];let selectedKey;const key={{[Symbol.toPrimitive](hint){{trace.push('coerce:'+hint);return selectedKey;}}}};let enabled={enabled};"
    );
    let rules = format!(
        "{{zIndex:-1,...((trace.push('guard'),enabled)?{{zIndex:{array}[key],color:(trace.push('after'),'pink')}}:{{}})}}"
    );
    let keys: Vec<_> = observations.iter().map(|(key, _)| key).collect();
    let keys = serde_json::to_string(&keys)
        .unwrap_or_else(|error| panic!("array length keys must serialize: {error}"));
    let expected_trace = if enabled {
        vec!["guard", "coerce:string", "after"]
    } else {
        vec!["guard"]
    };
    let expected_original: Vec<_> = observations
        .iter()
        .map(|(_, value)| {
            serde_json::json!([
                if enabled { *value } else { Some("-1") },
                if enabled { Some("pink") } else { None },
                expected_trace
            ])
        })
        .collect();
    let original_script = format!(
        "const styled={{div:rules=>rules}};{setup}const make=()=>styled.div({rules});JSON.stringify({keys}.map(next=>{{selectedKey=next;trace.length=0;const record=make();return [typeof record.zIndex==='number'?String(record.zIndex):null,record.color??null,trace.slice()];}}));"
    );
    let mut original_context = Context::default();
    let original = original_context
        .eval(Source::from_bytes(&original_script))
        .unwrap_or_else(|error| panic!("authored array length execution failed: {error}"));
    assert_eq!(
        original
            .as_string()
            .unwrap_or_else(|| panic!("authored array length observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!(expected_original).to_string()
    );
    reset_class_map();
    reset_file_map();
    let source = format!("{IMPORT}{setup}const Card=styled.div({rules});");
    let output = extract(
        "array-length.ts",
        &source,
        ExtractOption {
            single_css,
            import_main_css: false,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("array length extraction failed: {error}"));
    let mut declarations = Vec::new();
    let mut dynamic = Vec::new();
    for style in &output.styles {
        match style {
            crate::ExtractStyleValue::Static(value) => {
                assert_eq!(value.property(), "z-index");
                let Some(crate::extract_style::style_property::StyleProperty::ClassName(name)) =
                    style.extract(if single_css {
                        None
                    } else {
                        Some("array-length.ts")
                    })
                else {
                    panic!("static array length z-index must have a class");
                };
                declarations.push((name, value.value()));
            }
            crate::ExtractStyleValue::Dynamic(value) => dynamic.push(value.property()),
            _ => panic!("array length fixture must emit only z-index and color declarations"),
        }
    }
    dynamic.sort_unstable();
    dynamic.dedup();
    assert_eq!(dynamic, vec!["color"], "finite z-index must stay static");
    let declarations = serde_json::to_string(&declarations)
        .unwrap_or_else(|error| panic!("array length declarations must serialize: {error}"));
    let arena = Allocator::default();
    let mut program = parse(&arena, "array-length.ts", &output.code);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: changing keys repeatedly use the same emitted renderer, not a new definition.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{code}\nconst before=trace.slice();const results={keys}.map(next=>{{selectedKey=next;trace.length=0;const rendered=Card.render({{}},null);const names=rendered.props.className.split(' ');const selected={declarations}.filter(([name])=>names.includes(name)).map(([,value])=>value).sort();const inline=Object.values(rendered.props.style||{{}}).filter(value=>value!=null).sort();return [selected,inline,trace.slice()];}});JSON.stringify([before,results]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("emitted array length execution failed: {error}"));
    let expected: Vec<_> = observations
        .iter()
        .map(|(_, value)| {
            let selected: Vec<_> = if enabled {
                value.iter().copied().collect()
            } else {
                vec!["-1"]
            };
            let inline = if enabled { vec!["pink"] } else { vec![] };
            serde_json::json!([selected, inline, expected_trace])
        })
        .collect();
    // Then: actual selected static CSS matches authored values, with once-only ordered coercion.
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("emitted array length observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!([[], expected]).to_string()
    );
}
