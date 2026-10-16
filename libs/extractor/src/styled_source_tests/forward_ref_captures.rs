use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case("test.ts")]
#[case("test.cts")]
#[serial]
fn callback_contract_when_many_dependent_guarded_captures_render(
    #[case] filename: &str,
    #[values(false, true)] outer: bool,
    #[values(false, true)] inner: bool,
) {
    // Given: shallow guarded reads, an anonymous controller and one eager target getter.
    let properties = [
        "color",
        "backgroundColor",
        "borderColor",
        "outlineColor",
        "width",
        "height",
        "minWidth",
        "minHeight",
        "maxWidth",
        "maxHeight",
        "top",
        "right",
        "bottom",
        "left",
        "fontSize",
        "lineHeight",
        "letterSpacing",
        "wordSpacing",
        "textIndent",
        "borderWidth",
        "borderRadius",
        "outlineWidth",
        "outlineOffset",
        "columnGap",
        "rowGap",
        "flexBasis",
        "marginTop",
        "marginRight",
        "marginBottom",
        "marginLeft",
        "paddingTop",
        "paddingRight",
    ];
    let leaves = properties
        .iter()
        .enumerate()
        .map(|(index, property)| format!("{property}:(trace.push('leaf:{index}'),previous+='x')"))
        .collect::<Vec<_>>()
        .join(",");
    let defaults = properties
        .iter()
        .map(|property| format!("{property}:'initial'"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "{IMPORT}const trace=[];const names=[];let reads=0;let previous='';const outer={outer},inner={inner};const __devupRecord0='authored';const targets={{get div(){{reads++;trace.push('target');return 'article';}}}};const name='div';const Card=styled(targets[name]).attrs({{id:'card'}})({{{defaults},...((class {{static {{names.push(this.name)}}}})?{{...((trace.push('outer'),outer)?{{...((trace.push('inner'),inner)?{{{leaves}}}:{{}})}}:{{}})}}:{{}})}});"
    );
    let output = compile(filename, &source, true);
    let arena = Allocator::default();
    let mut program = parse(&arena, filename, &output);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    let prelude = match filename {
        "test.ts" => format!(
            "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});"
        ),
        "test.cts" => "function require(name){if(name==='react')return {forwardRef:render=>({render})};if(name==='react/jsx-runtime')return {jsx:(target,props)=>({target,props})};throw new Error(name);}".to_string(),
        _ => panic!("unexpected capture fixture dialect"),
    };

    // When: the actual emitted forwardRef callback renders twice with distinct callback refs.
    let script = format!(
        "{prelude}{code}\nconst before=[reads,trace.slice(),names.slice()];const first=Card.render({{ref:99,children:'first',className:'caller',forwardedAs:'nav'}},7);previous='';const second=Card.render({{ref:99,children:'second',as:'section',forwardedAs:'aside'}},8);JSON.stringify([before,reads,first.target,second.target,first.props.ref,second.props.ref,first.props.children,second.props.children,first.props.as,second.props.as,first.props.id,second.props.id,first.props.className.includes('caller'),Object.values(first.props.style||{{}}).map(value=>value===undefined?'absent':value),Object.values(second.props.style||{{}}).map(value=>value===undefined?'absent':value),names,trace,__devupRecord0]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("forwardRef capture execution failed: {error}"));
    let result = result
        .as_string()
        .unwrap_or_else(|| panic!("forwardRef capture observation must be JSON"))
        .to_std_string_escaped();

    // Then: target/ref/attrs stay intact; selected dependent reads run once in authored order.
    let mut trace = vec!["target".to_string()];
    let mut values = Vec::new();
    for _ in 0..2 {
        trace.push("outer".to_string());
        if outer {
            trace.push("inner".to_string());
            if inner {
                for index in 0..properties.len() {
                    trace.push(format!("leaf:{index}"));
                }
            }
        }
    }
    if outer && inner {
        values.extend((1..=properties.len()).map(|length| "x".repeat(length)));
    } else {
        values.extend(properties.iter().map(|_| "absent".to_string()));
    }
    let expected = serde_json::json!([
        [1, ["target"], []],
        1,
        "article",
        "section",
        7,
        8,
        "first",
        "second",
        "nav",
        "aside",
        "card",
        "card",
        true,
        values,
        values,
        ["", ""],
        trace,
        "authored"
    ]);
    assert_eq!(result, expected.to_string());
}
