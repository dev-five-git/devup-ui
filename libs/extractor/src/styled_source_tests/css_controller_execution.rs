use super::*;

#[rstest]
#[case::inline_call("css({color:mark('left','red')})&&{color:mark('right','blue')}", serde_json::json!(["ok",["left","right","after"],["blue"]]))]
#[case::dynamic_null("css({color:state.color})||{color:mark('right','blue')}", serde_json::json!(["ok",["left","after"],[null]]))]
#[case::nested("({color:mark('left','red')}||{color:mark('unused','pink')})&&{color:mark('right','blue')}", serde_json::json!(["ok",["left","right","after"],["blue"]]))]
#[case::coercion("`width:${{toString(){trace.push('coerce');throw Error('stop');}}};height:4px;`", serde_json::json!(["throw",["coerce"],"stop"]))]
#[serial]
fn css_controllers_preserve_branch_ownership_and_coercion_cutoff(
    #[case] rules: &str,
    #[case] expected: Value,
    #[values("css-controller.ts", "css-controller.tsx")] filename: &str,
    #[values(false, true)] single_css: bool,
) {
    let element = if filename.ends_with('x') {
        format!("<div css={{{rules}}} id={{mark('after','id')}}/>")
    } else {
        format!("jsx('div',{{css:{rules},id:mark('after','id')}})")
    };
    let source = format!(
        "const trace=[];function mark(name,value){{trace.push(name);return value;}}const state={{get color(){{return mark('left',null);}}}};function App(){{return {element};}}"
    );
    let result = execute_in_mode(
        filename,
        &source,
        "let result;try{const value=App();result=['ok',trace.slice(),values(value,'color')];}catch(error){result=['throw',trace.slice(),error.message];}JSON.stringify(result);",
        single_css,
    );
    assert_eq!(result, expected);
}
