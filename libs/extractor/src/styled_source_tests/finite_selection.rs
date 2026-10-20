use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case::object(("({a:'red',b:'blue'})", &[("a",Some("red")),("b",Some("blue")),("missing",None),("toString",None),("__proto__",None)][..], true))]
#[case::duplicate(("({a:'blue',a:'red',b:'blue'})", &[("a",Some("red")),("b",Some("blue")),("missing",None)][..], true))]
#[case::decimal_string_key(("({'1.0':'red','1':'blue'})", &[("1.0",Some("red")),("1",Some("blue")),("1.0",Some("red")),("missing",None)][..], true))]
#[case::leading_zero_string_key(("({'01':'red','1':'blue'})", &[("01",Some("red")),("1",Some("blue")),("01",Some("red")),("missing",None)][..], true))]
#[case::numeric_keys(("({1:'red',2:'blue'})", &[("1",Some("red")),("2",Some("blue")),("missing",None)][..], true))]
#[case::boolean_keys(("({[true]:'red',[false]:'blue'})", &[("true",Some("red")),("false",Some("blue")),("missing",None)][..], true))]
#[case::unary_string_key(("({[+'inf']:'red',Infinity:'blue'})", &[("NaN",Some("red")),("Infinity",Some("blue")),("NaN",Some("red")),("missing",None)][..], false))]
#[case::unary_numeric_key(("({[-1]:'red',1:'blue'})", &[("-1",Some("red")),("1",Some("blue")),("missing",None)][..], false))]
#[case::fractional_numeric_key(("({[0.5]:'red',1:'blue'})", &[("0.5",Some("red")),("1",Some("blue")),("missing",None)][..], false))]
#[case::array(("['red','blue']", &[("0",Some("red")),("1",Some("blue")),("9",None)][..], true))]
#[case::sparse(("['red',,'blue']", &[("0",Some("red")),("2",Some("blue")),("1",None),("9",None)][..], true))]
#[case::nullable(("['red',null,'blue']", &[("0",Some("red")),("2",Some("blue")),("1",None),("9",None)][..], true))]
#[case::computed_own_proto(("({['__proto__']:'red',a:'blue'})", &[("__proto__",Some("red")),("a",Some("blue")),("missing",None)][..], true))]
#[case::own_prototype_names(("({toString:'red',constructor:'blue'})", &[("toString",Some("red")),("constructor",Some("blue")),("missing",None)][..], true))]
#[case::prototype_setter(("({__proto__:{a:'red',b:'blue'}})", &[("a",Some("red")),("b",Some("blue")),("missing",None)][..], false))]
#[case::quoted_prototype_setter(("({'__proto__':{a:'red',b:'blue'}})", &[("a",Some("red")),("b",Some("blue")),("missing",None)][..], false))]
#[serial]
fn selection_when_key_coercion_precedes_later_read(
    #[case] fixture: (&str, &[(&str, Option<&str>)], bool),
    #[values(false, true)] single_css: bool,
    #[values(false, true)] enabled: bool,
) {
    // Given: observable key coercion followed by an independently captured style value.
    let (map, observations, finite) = fixture;
    let setup = format!(
        "const trace=[];let selectedKey;const key={{[Symbol.toPrimitive](hint){{trace.push('coerce:'+hint);return selectedKey;}}}};let enabled={enabled};"
    );
    let rules = format!(
        "{{bg:'blue',...((trace.push('guard'),enabled)?{{bg:{map}[key],color:(trace.push('after'),'pink')}}:{{}})}}"
    );
    let keys: Vec<_> = observations.iter().map(|(key, _)| key).collect();
    let keys = serde_json::to_string(&keys)
        .unwrap_or_else(|error| panic!("selection keys must serialize: {error}"));
    let expected_trace = if enabled {
        vec!["guard", "coerce:string", "after"]
    } else {
        vec!["guard"]
    };
    let expected_original: Vec<_> = observations
        .iter()
        .map(|(_, value)| {
            serde_json::json!([
                if enabled { *value } else { Some("blue") },
                if enabled { Some("pink") } else { None },
                expected_trace
            ])
        })
        .collect();
    let original_script = format!(
        "const styled={{div:rules=>rules}};{setup}const make=()=>styled.div({rules});JSON.stringify({keys}.map(next=>{{selectedKey=next;trace.length=0;const record=make();return [typeof record.bg==='string'?record.bg:null,record.color??null,trace.slice()];}}));"
    );
    let mut original_context = Context::default();
    let original = original_context
        .eval(Source::from_bytes(&original_script))
        .unwrap_or_else(|error| panic!("authored selection execution failed: {error}"));
    assert_eq!(
        original
            .as_string()
            .unwrap_or_else(|| panic!("authored selection observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!(expected_original).to_string()
    );
    reset_class_map();
    reset_file_map();
    let source = format!("{IMPORT}{setup}const Card=styled.div({rules});");
    let output = extract(
        "selection.ts",
        &source,
        ExtractOption {
            single_css,
            import_main_css: false,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("selection extraction failed: {error}"));
    let mut declarations = Vec::new();
    let mut backgrounds = Vec::new();
    let mut dynamic = Vec::new();
    for style in &output.styles {
        match style {
            crate::ExtractStyleValue::Static(value) => {
                assert_eq!(value.property(), "background");
                backgrounds.push(value.value());
                let Some(crate::extract_style::style_property::StyleProperty::ClassName(name)) =
                    style.extract(if single_css {
                        None
                    } else {
                        Some("selection.ts")
                    })
                else {
                    panic!("static selection background must have a class");
                };
                declarations.push((name, value.value()));
            }
            crate::ExtractStyleValue::Dynamic(value) => dynamic.push(value.property()),
            _ => panic!("selection fixture must emit only static backgrounds and dynamic values"),
        }
    }
    backgrounds.sort_unstable();
    backgrounds.dedup();
    dynamic.sort_unstable();
    dynamic.dedup();
    let expected_backgrounds = match map {
        "['red','blue']" => vec!["8px", "blue", "red"],
        "['red',,'blue']" | "['red',null,'blue']" => vec!["12px", "blue", "red"],
        _ if finite => vec!["blue", "red"],
        _ => vec!["blue"],
    };
    assert_eq!(backgrounds, expected_backgrounds);
    assert_eq!(
        dynamic,
        if finite {
            vec!["color"]
        } else {
            vec!["background", "color"]
        }
    );
    let declarations = serde_json::to_string(&declarations)
        .unwrap_or_else(|error| panic!("selection declarations must serialize: {error}"));
    let arena = Allocator::default();
    let mut program = parse(&arena, "selection.ts", &output.code);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: render changing and missing keys through the actual emitted callback.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{code}\nconst before=trace.slice();const results={keys}.map(next=>{{selectedKey=next;trace.length=0;const rendered=Card.render({{}},null);const names=rendered.props.className.split(' ');const selected={declarations}.filter(([name])=>names.includes(name)).map(([,value])=>value).sort();const inline=Object.values(rendered.props.style||{{}}).filter(value=>value!=null).sort();return [selected,inline,trace.slice()];}});JSON.stringify([before,results]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("emitted selection execution failed: {error}"));
    let expected: Vec<_> = observations
        .iter()
        .map(|(_, value)| {
            let selected: Vec<_> = if !enabled {
                vec!["blue"]
            } else if finite {
                value.iter().copied().collect()
            } else {
                vec![]
            };
            let mut inline = if enabled { vec!["pink"] } else { vec![] };
            if enabled && !finite {
                inline.extend(value.iter().copied());
            }
            inline.sort_unstable();
            serde_json::json!([selected, inline, expected_trace])
        })
        .collect();
    // Then: static branch classes survive; coercion runs once before the later read.
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("emitted selection observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!([[], expected]).to_string()
    );
}
