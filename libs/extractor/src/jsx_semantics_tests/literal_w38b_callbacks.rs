use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("function(p){function inner(){return p.active?4:5}const arrow=()=>6;trace.push(inner(),arrow());return p.active?2:3}", serde_json::json!(["built",4,6,5,6]), (2, 3))]
#[case("p=>{trace.push(p.active);return p.active?2:3}", serde_json::json!(["built",true,false]), (2, 3))]
#[case("function(p){{trace.push('block');return p.active?4:5}}", serde_json::json!(["built","block","block"]), (4, 5))]
#[case("p=>{if(p.active){return 2}else{return 3}}", serde_json::json!(["built"]), (2, 3))]
#[serial]
fn order_callback_when_terminal_preserves_render_and_nested_returns(
    #[case] callback: &str,
    #[case] trace: serde_json::Value,
    #[case] orders: (u8, u8),
) {
    let (yes, no) = orders;
    // Given: real parser callbacks, nested functions and terminal control-flow shapes.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=(render)=>render;const Card=styled.div`style-order:${{{callback}}};color:red`;trace.push('built');const a=Card({{active:true}},null);const b=Card({{active:false}},null);"
    );
    // When: emitted construction and both renders execute.
    let actual = whole::evaluate(&source, "[a.props.className,b.props.className]");
    // Then: inner returns stay numeric and only renders select order.
    assert_eq!(actual.trace, trace);
    assert!(
        actual.element[0]
            .as_str()
            .required("true class")
            .contains(&format!("--{yes}-"))
    );
    assert!(
        actual.element[1]
            .as_str()
            .required("false class")
            .contains(&format!("--{no}-"))
    );
}

#[rstest]
#[case("async p=>2")]
#[case("function*(p){return 2}")]
#[case("p=>{return;}")]
#[case("p=>{trace.push('tail')}")]
#[case("p=>{if(p.active)return 2}")]
#[serial]
fn order_callback_when_unsupported_rejects_before_execution(#[case] callback: &str) {
    // Given: parser-valid callbacks which cannot produce a synchronous total order.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';\nconst Card=styled.div`style-order:${{{callback}}};color:red`;"
    );
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .find(callback)
        .required("callback")
        + 1;
    // When: public extraction rejects, without evaluating rejected code.
    let actual = error(&source);
    // Then: strict order metadata points to the authored callback.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
    assert!(actual.contains("styleOrder"), "{actual}");
}

#[test]
#[serial]
fn order_callback_when_value_is_a_class_rejects_numeric_coercion() {
    // Given: a class-producing callback is not numeric metadata.
    let source = "import {styled,css} from '@devup-ui/react';const Card=styled.div`style-order:${p=>css({color:p.active?'red':'blue'})};background:black`;";
    // When: public validation inspects the wrapper/finite provenance.
    let actual = error(source);
    // Then: no class string is accepted as an order.
    assert!(actual.contains("styleOrder"), "{actual}");
}
