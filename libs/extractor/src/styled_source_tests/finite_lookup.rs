use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case::object(("({a:'red',b:'blue'})", ["a", "b", "missing"]))]
#[case::array(("['red','blue']", ["0", "1", "9"]))]
#[case::typed_object(("({a:'red',b:'blue'} as const)", ["a", "b", "missing"]))]
#[case::typed_array(("(['red','blue'] as const)", ["0", "1", "9"]))]
#[case::sparse_array(("['red',,'blue']", ["0", "2", "9"]))]
#[case::nullable_array(("['red',null,'blue']", ["0", "2", "9"]))]
#[serial]
fn static_classes_when_finite_lookup_key_changes(
    #[case] fixture: (&str, [&str; 3]),
    #[values(false, true)] single_css: bool,
    #[values(None, Some(false), Some(true))] enabled: Option<bool>,
) {
    // Given: finite authored values and independent, observable guard/key reads.
    let (map, keys) = fixture;
    let rules = match enabled {
        None => format!("{{bg:{map}[state.key]}}"),
        Some(enabled) => {
            format!("{{...((trace.push('guard'),{enabled})?{{bg:{map}[state.key]}}:{{}})}}")
        }
    };
    let source = format!(
        "{IMPORT}const trace=[];let key;const state={{get key(){{trace.push('key');return key;}}}};const Card=styled.div({rules});"
    );
    reset_class_map();
    reset_file_map();
    let output = extract(
        "lookup.ts",
        &source,
        ExtractOption {
            single_css,
            import_main_css: false,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("finite lookup extraction failed: {error}"));
    let mut declarations = Vec::new();
    let mut values = Vec::new();
    for style in &output.styles {
        let crate::ExtractStyleValue::Static(value) = style else {
            panic!("finite lookup must not emit dynamic CSS variables");
        };
        assert_eq!(value.property(), "background");
        values.push(value.value());
        let Some(crate::extract_style::style_property::StyleProperty::ClassName(name)) =
            style.extract(if single_css { None } else { Some("lookup.ts") })
        else {
            panic!("finite lookup declarations must have classes");
        };
        declarations.push((name, value.value()));
    }
    values.sort_unstable();
    let expected_values = match map {
        "['red','blue']" | "(['red','blue'] as const)" => vec!["8px", "blue", "red"],
        "['red',,'blue']" | "['red',null,'blue']" => vec!["12px", "blue", "red"],
        _ => vec!["blue", "red"],
    };
    assert_eq!(values, expected_values);
    let declarations = serde_json::to_string(&declarations)
        .unwrap_or_else(|error| panic!("finite declarations must serialize: {error}"));
    let keys = serde_json::to_string(&keys)
        .unwrap_or_else(|error| panic!("finite lookup keys must serialize: {error}"));
    let arena = Allocator::default();
    let mut program = parse(&arena, "lookup.ts", &output.code);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: render selected, changed and missing keys through the emitted callback.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{code}\nconst before=trace.slice();const results={keys}.map(next=>{{key=next;const rendered=Card.render({{}},null);const names=rendered.props.className.split(' ');return [{declarations}.filter(([name])=>names.includes(name)).map(([,value])=>value),Object.values(rendered.props.style||{{}}).filter(value=>value!=null)];}});JSON.stringify([before,results,trace]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("finite lookup execution failed: {error}"));
    let result = result
        .as_string()
        .unwrap_or_else(|| panic!("finite lookup observation must be JSON"))
        .to_std_string_escaped();
    // Then: selected classes resolve authored CSS; keys run once only under the guard.
    let expected = match enabled {
        None => serde_json::json!([
            [],
            [[["red"], []], [["blue"], []], [[], []]],
            ["key", "key", "key"]
        ]),
        Some(true) => serde_json::json!([
            [],
            [[["red"], []], [["blue"], []], [[], []]],
            ["guard", "key", "guard", "key", "guard", "key"]
        ]),
        Some(false) => serde_json::json!([
            [],
            [[[], []], [[], []], [[], []]],
            ["guard", "guard", "guard"]
        ]),
    };
    assert_eq!(result, expected.to_string());
}

#[rstest]
#[case::unknown("state.map[state.key]", "get map(){trace.push('object');return {a:'red'}},", ["object", "key"])]
#[case::getter("({get a(){trace.push('value');return 'red'}})[state.key]", "", ["key", "value"])]
#[case::dynamic_value("({a:(trace.push('value'),'red')})[state.key]", "", ["value", "key"])]
#[case::spread("({...state.map})[state.key]", "get map(){trace.push('object');return {a:'red'}},", ["object", "key"])]
#[case::computed_key("({[(trace.push('property'),'a')]:'red'})[state.key]", "", ["property", "key"])]
#[serial]
fn whole_value_when_lookup_is_not_finite_static(
    #[case] lookup: &str,
    #[case] object: &str,
    #[case] order: [&str; 2],
) {
    // Given: unknown or impure lookup objects must not enter finite class extraction.
    let source = format!(
        "{IMPORT}const trace=[];const state={{{object}get key(){{trace.push('key');return 'a'}}}};const Card=styled.div({{bg:{lookup}}});"
    );
    let output = compile("lookup.ts", &source, true);
    let arena = Allocator::default();
    let mut program = parse(&arena, "lookup.ts", &output);
    let calls = renders(&program);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: execute the actual emitted dynamic value once per render.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{code}\nconst before=trace.slice();const rendered=Card.render({{}},null);JSON.stringify([before,Object.values(rendered.props.style),trace]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("whole lookup execution failed: {error}"));
    // Then: object, key and selected value effects retain their original order/count.
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("whole lookup observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!([[], ["red"], order]).to_string()
    );
}

#[rstest]
#[case::template_value(("({a:`0.${(trace.push('value'),5)}`})[state.key]", "a", &["guard", "value", "key"][..]))]
#[case::template_property_key(("({[`0.${(trace.push('property'),5)}`]:0.5})[state.key]", "0.5", &["guard", "property", "key"][..]))]
#[case::pure_template_value(("({a:`0.${5}`})[state.key]", "a", &["guard", "key"][..]))]
#[case::pure_template_array_value(("[`0.${5}`][state.key]", "0", &["guard", "key"][..]))]
#[case::pure_template_property_key(("({[`0.${5}`]:0.5})[state.key]", "0.5", &["guard", "key"][..]))]
#[case::pure_template_member_index(("({'0.5':0.5})[`0.${5}`]", "unused", &["guard"][..]))]
#[case::wrapped_template_value(("({a:+(`0.${5}`)})[state.key]", "a", &["guard", "key"][..]))]
#[case::wrapped_template_property_key(("({[+(`0.${5}`)]:0.5})[state.key]", "0.5", &["guard", "key"][..]))]
#[case::nested_object_template_value(("({a:0.5,b:{nested:`0.${5}`}})[state.key]", "a", &["guard", "key"][..]))]
#[case::nested_array_template_value(("[0.5,[`0.${5}`]][state.key]", "0", &["guard", "key"][..]))]
#[case::nested_template_member_index((concat!("(", "{", "a:0.5})[`$", "{", "state.key}`]"), "a", &["guard", "key"][..]))]
#[case::sequence_template_member_index(("({'0.5':0.5})[(trace.push('index'),`0.${5}`)]", "unused", &["guard", "index"][..]))]
#[case::typed_template_value(("({a:(`0.${5}` as const)} as const)[state.key]", "a", &["guard", "key"][..]))]
#[case::typed_template_property_key(("({[(`0.${5}` as const)]:0.5})[state.key]", "0.5", &["guard", "key"][..]))]
#[case::typed_template_member_index(("({'0.5':0.5})[(`0.${5}` as const)]", "unused", &["guard"][..]))]
#[serial]
fn whole_value_when_numeric_template_lookup_has_effects(
    #[case] fixture: (&str, &str, &[&str]),
    #[values(false, true)] single_css: bool,
    #[values(false, true)] enabled: bool,
) {
    // Given: template-bearing lookup values, property keys and member indices.
    let (lookup, key, selected_trace) = fixture;
    let source = format!(
        "{IMPORT}const trace=[];let enabled={enabled};const state={{get key(){{trace.push('key');return '{key}';}}}};const Card=styled.div({{opacity:0.3,...((trace.push('guard'),enabled)?{{opacity:{lookup}}}:{{}})}});"
    );
    let expected_trace = if enabled { selected_trace } else { &["guard"] };
    let expected_opacity = if enabled { "0.5" } else { "0.3" };
    let original = source
        .strip_prefix(IMPORT)
        .unwrap_or_else(|| panic!("numeric template fixture must start with the public import"))
        .replace(" as const", "");
    let original_script = format!(
        "const styled={{div:rules=>rules}};{original}\nJSON.stringify([String(Card.opacity),trace]);"
    );
    let mut original_context = Context::default();
    let original_result = original_context
        .eval(Source::from_bytes(&original_script))
        .unwrap_or_else(|error| panic!("original numeric template execution failed: {error}"));
    assert_eq!(
        original_result
            .as_string()
            .unwrap_or_else(|| panic!("original numeric template observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!([expected_opacity, expected_trace]).to_string()
    );
    reset_class_map();
    reset_file_map();
    let output = extract(
        "lookup.ts",
        &source,
        ExtractOption {
            single_css,
            import_main_css: false,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("numeric template lookup extraction failed: {error}"));
    let declarations: Vec<_> = output
        .styles
        .iter()
        .map(|style| {
            use crate::extract_style::style_property::StyleProperty;
            match (
                style,
                style.extract(if single_css { None } else { Some("lookup.ts") }),
            ) {
                (crate::ExtractStyleValue::Static(value), Some(StyleProperty::ClassName(name))) => {
                    serde_json::json!([
                        name,
                        value.property(),
                        value.value(),
                        null,
                        value.level(),
                        value.selector().map(ToString::to_string)
                    ])
                }
                (
                    crate::ExtractStyleValue::Dynamic(value),
                    Some(StyleProperty::Variable {
                        class_name,
                        variable_name,
                        ..
                    }),
                ) => serde_json::json!([
                    class_name,
                    value.property(),
                    null,
                    variable_name,
                    value.level(),
                    value.selector().map(ToString::to_string)
                ]),
                other => panic!("unexpected lookup declaration: {:?}", other.0),
            }
        })
        .collect();
    let declarations = serde_json::to_string(&declarations)
        .unwrap_or_else(|error| panic!("opacity declarations must serialize: {error}"));
    let arena = Allocator::default();
    let mut program = parse(&arena, "lookup.ts", &output.code);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    // The real TS bundler erases these fixture-only assertions before execution.
    let code = Codegen::new().build(&program).code.replace(" as const", "");
    // When: execute the actual emitted renderer, retaining definition-time observations.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{code}\nconst before=trace.slice();const rendered=Card.render({{}},null);const names=rendered.props.className.split(' ');const selected={declarations}.filter(([name])=>names.includes(name)).map(([,property,value,variable,level,selector])=>[property,String(Number(variable?(rendered.props.style||{{}})[variable]:value)),level,selector]).sort();JSON.stringify([before,selected,trace]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| {
            panic!("emitted numeric template lookup execution failed: {error}")
        });
    // Then: no eager reads; selected lookups retain exact effect order and dynamic value.
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("emitted numeric template observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!([[], [["opacity", expected_opacity, 0, null]], expected_trace])
            .to_string()
    );
}
