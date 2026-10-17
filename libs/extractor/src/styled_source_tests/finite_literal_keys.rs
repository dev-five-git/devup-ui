use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case::canonical_object(("({1:'red',2:'blue'})[1]", Some("red"), true))]
#[case::canonical_array(("['red','blue']['1']", Some("blue"), true))]
#[case::decimal_key(("({'1.0':'red','1':'blue'})['1.0']", Some("red"), false))]
#[case::canonical_index_with_decimal_key(("({'1.0':'red','1':'blue'})['1']", Some("blue"), false))]
#[case::leading_zero_key(("({'01':'red','1':'blue'})['01']", Some("red"), false))]
#[case::canonical_index_with_leading_zero_key(("({'01':'red','1':'blue'})[1]", Some("blue"), false))]
#[case::array_noncanonical_index(("['red','blue']['01']", None, false))]
#[case::array_fractional_string_index(("['red','blue']['1.0']", None, false))]
#[case::unary_string_index(("({NaN:'red',Infinity:'blue'})[+'inf']", Some("red"), false))]
#[case::unary_string_property(("({[+'inf']:'red',Infinity:'blue'})['NaN']", Some("red"), false))]
#[serial]
fn native_selection_when_literal_lookup_cannot_use_legacy_key_conversion(
    #[case] fixture: (&str, Option<&str>, bool),
    #[values(false, true)] single_css: bool,
    #[values(false, true)] enabled: bool,
) {
    // Given: distinguish exact direct lookups from normalization-sensitive literal keys.
    let (lookup, value, finite) = fixture;
    let setup = format!("const trace=[];let enabled={enabled};");
    let rules = format!(
        "{{bg:'orange',...((trace.push('guard'),enabled)?{{bg:{lookup},color:(trace.push('after'),'pink')}}:{{}})}}"
    );
    let expected_value = if enabled { value } else { Some("orange") };
    let expected_trace = if enabled {
        vec!["guard", "after"]
    } else {
        vec!["guard"]
    };
    let original_script = format!(
        "const styled={{div:rules=>rules}};{setup}const Card=styled.div({rules});JSON.stringify([Card.bg??null,trace]);"
    );
    let mut original_context = Context::default();
    let original = original_context
        .eval(Source::from_bytes(&original_script))
        .unwrap_or_else(|error| panic!("authored literal lookup failed: {error}"));
    assert_eq!(
        original
            .as_string()
            .unwrap_or_else(|| panic!("authored literal observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!([expected_value, expected_trace]).to_string()
    );
    reset_class_map();
    reset_file_map();
    let source = format!("{IMPORT}{setup}const Card=styled.div({rules});");
    let output = extract(
        "literal-keys.ts",
        &source,
        ExtractOption {
            single_css,
            import_main_css: false,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("literal lookup extraction failed: {error}"));
    let mut declarations = Vec::new();
    let mut dynamic = Vec::new();
    for style in &output.styles {
        match style {
            crate::ExtractStyleValue::Static(value) => {
                assert_eq!(value.property(), "background");
                let Some(crate::extract_style::style_property::StyleProperty::ClassName(name)) =
                    style.extract(if single_css {
                        None
                    } else {
                        Some("literal-keys.ts")
                    })
                else {
                    panic!("static literal lookup must have a class");
                };
                declarations.push((name, value.value()));
            }
            crate::ExtractStyleValue::Dynamic(value) => dynamic.push(value.property()),
            _ => panic!("literal lookup fixture must emit only backgrounds and dynamic color"),
        }
    }
    dynamic.sort_unstable();
    dynamic.dedup();
    assert_eq!(
        dynamic,
        if finite {
            vec!["color"]
        } else {
            vec!["background", "color"]
        }
    );
    let declarations = serde_json::to_string(&declarations)
        .unwrap_or_else(|error| panic!("literal declarations must serialize: {error}"));
    let arena = Allocator::default();
    let mut program = parse(&arena, "literal-keys.ts", &output.code);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: execute the emitted renderer and resolve its actual selected CSS declarations.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{code}\nconst before=trace.slice();const rendered=Card.render({{}},null);const names=rendered.props.className.split(' ');const selected={declarations}.filter(([name])=>names.includes(name)).map(([,value])=>value).sort();const inline=Object.values(rendered.props.style||{{}}).filter(value=>value!=null).sort();JSON.stringify([before,selected,inline,trace]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("emitted literal lookup failed: {error}"));
    let selected: Vec<_> = if !enabled || finite {
        expected_value.into_iter().collect()
    } else {
        vec![]
    };
    let mut inline = if enabled { vec!["pink"] } else { vec![] };
    if enabled && !finite {
        inline.extend(value);
    }
    inline.sort_unstable();
    // Then: conservative fallback preserves the authored value and guard/after ordering.
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("emitted literal observation must be JSON"))
            .to_std_string_escaped(),
        serde_json::json!([[], selected, inline, expected_trace]).to_string()
    );
}
