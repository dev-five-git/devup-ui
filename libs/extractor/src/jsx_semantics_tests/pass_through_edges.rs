use super::*;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn computed_proto_when_local_const_is_only_copied_keeps_known_d1() {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=()=>{{const first={{['__proto__']:null,p:'20px'}};return jsx(Box,{{p:'4px',...first}});}};"
    );
    let compiled = output(&source);
    assert_eq!(
        static_styles(&compiled),
        vec![("padding".into(), "20px".into())]
    );
}

#[test]
#[serial]
fn computed_proto_when_known_in_jsx_keeps_the_literal_selector_diagnostic() {
    let direct =
        format!("{BOX}export const a=()=> <Box p='4px' {{...{{['__proto__']:null,p:'20px'}}}}/>;");
    let copied = format!(
        "{BOX}export const a=()=>{{const first={{['__proto__']:null,p:'20px'}};return <Box p='4px' {{...first}}/>;}};"
    );
    for source in [direct, copied] {
        let message = error(&source);
        assert!(message.contains("cannot use `__proto__`"), "{message}");
        assert!(message.contains(crate::utils::SELECTOR_NAME), "{message}");
    }
}

#[rstest]
#[case("jsx(Box,{color:()=>1,color:'red'})")]
#[case("jsx(Box,{...{color:()=>1},color:'red'})")]
#[case("jsx(Box,{color:(trace.push('discarded'),()=>1),color:'red'})")]
#[serial]
fn shadowed_factory_values_when_unreadable_keep_the_last_known_style(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=()=>{element};");
    let compiled = output(&source);
    assert_eq!(
        static_styles(&compiled),
        vec![("color".into(), "red".into())]
    );
    let actual = whole::execute(&compiled.code, "a()");
    let expected = if element.contains("trace.push") {
        serde_json::json!(["discarded"])
    } else {
        serde_json::json!([])
    };
    assert_eq!(actual.trace, expected);
}

#[test]
#[serial]
fn computed_proto_when_copied_into_ordinary_objects_stays_an_own_key() {
    let source = format!(
        "{BOX}export const a=()=>{{const first={{['__proto__']:null,p:'20px'}};const copy={{...first}};return <Box title={{Object.hasOwn(copy,'__proto__') && copy.__proto__===null}}/>;}};"
    );
    assert_eq!(whole::evaluate(&source, "a().props.title").element, true);
}

#[rstest]
#[case("<div css={{color:'red'}} {...{...{title:'x'}}}/>")]
#[case("jsx('div',{css:{color:'red'},...{...{title:'x'}}})")]
#[case("<div css={{color:'red'}} {...rest}/>")]
#[case("jsx('div',{css:{color:'red'},...rest})")]
#[serial]
fn native_emotion_when_spreads_are_not_flat_keeps_its_existing_refusal(#[case] element: &str) {
    let source = format!("{EMOTION}{JSX_RUNTIME}export const a=(rest)=>{element};");
    let message = compile_emotion(&source)
        .err()
        .unwrap_or_else(|| panic!("native admission must stay unchanged"));
    assert!(message.contains("a.tsx:3:"), "{message}");
    assert!(
        message.contains("a spread after the `css` prop"),
        "{message}"
    );
}

#[rstest]
#[case("<div css={{color:'red'}} {...{title:'x'}}/>")]
#[case("jsx('div',{css:{color:'red'},...{title:'x'}})")]
#[serial]
fn native_emotion_when_spreads_are_flat_keeps_its_known_style(#[case] element: &str) {
    let source = format!("{EMOTION}{JSX_RUNTIME}export const a=()=>{element};");
    let compiled = compile_emotion(&source).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        static_styles(&compiled),
        vec![("color".into(), "red".into())]
    );
    assert_eq!(
        whole::execute(&compiled.code, "a().props.title").element,
        "x"
    );
}
