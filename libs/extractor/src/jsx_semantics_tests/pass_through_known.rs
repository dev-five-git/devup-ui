use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box px={1} {...rest} pl={2} {...{...{bg:'red'},background:'blue'}}/>")]
#[case("jsx(Box,{px:1,...rest,pl:2,...{...{bg:'red'},background:'blue'}})")]
#[serial]
fn known_composition_when_nested_keeps_aliases_and_partial_targets(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest)=>{element};");
    let actual = whole::evaluate(&source, "a({px:9,bg:'green',id:'kept'})");
    assert_eq!(actual.element["props"]["px"], 9);
    assert_eq!(actual.element["props"]["bg"], "green");
    assert_eq!(actual.element["props"]["id"], "kept");
    assert_eq!(
        static_styles(&output(&source)),
        vec![
            ("background".into(), "blue".into()),
            ("padding-left".into(), "8px".into()),
            ("padding-right".into(), "4px".into())
        ]
    );
}

#[rstest]
#[case("<Box color={first()} {...{color:second(),...{color:'blue'}}} {...read()}/>")]
#[case("jsx(Box,{color:first(),...{color:second(),...{color:'blue'}},...read()})")]
#[serial]
fn known_duplicates_when_discarded_keep_original_effects(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(first,second,read)=>{element};");
    let actual = whole::evaluate(
        &source,
        "a(()=>{trace.push('first');return 'red';},()=>{trace.push('second');return 'green';},()=>{trace.push('spread');return {color:'raw'};})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["first", "second", "spread"])
    );
    assert_eq!(actual.element["props"]["color"], "raw");
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "blue".into())]
    );
}

#[rstest]
#[case("<Box typography='body' positioning='top' {...rest}/>")]
#[case("jsx(Box,{typography:'body',positioning:'top',...rest})")]
#[serial]
fn explicit_shapes_when_spreads_follow_keep_ordinary_rules(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest)=>{element};");
    let actual = whole::evaluate(&source, "a({typography:'raw',positioning:'raw'})");
    assert_eq!(actual.element["props"]["typography"], "raw");
    assert_eq!(
        static_styles(&output(&source)),
        vec![("top".into(), "0".into())]
    );
    assert!(output(&source).code.contains("typo-body"));
}

#[rstest]
#[case("<Box css={{color:'red'}} {...rest}/>")]
#[case("jsx(Box,{css:{color:'red'},...rest})")]
#[serial]
fn devup_css_when_unknown_data_follows_compiles_without_replacement(#[case] element: &str) {
    let source = format!("{EMOTION}{BOX}{JSX_RUNTIME}export const a=(rest)=>{element};");
    let compiled = compile_emotion(&source).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        static_styles(&compiled),
        vec![("color".into(), "red".into())]
    );
    assert!(compiled.code.contains("..."));
    let actual = whole::execute(
        &compiled.code,
        "(()=>{const raw={color:'blue'};const e=a({css:raw});return [e.props.css===raw,e.props.className];})()",
    );
    assert_eq!(actual.element[0], true);
    assert!(
        actual.element[1]
            .as_str()
            .unwrap_or_else(|| panic!("expected an authored class name"))
            .contains("red")
    );
}

#[rstest]
#[case("<Box p='4px' {...first} {...rest}/>")]
#[case("jsx(Box,{p:'4px',...first,...rest})")]
#[serial]
fn local_known_objects_when_only_copied_compose_without_runtime_evaluation(#[case] element: &str) {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=(rest)=>{{const first={{p:'20px'}};return {element};}};"
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("padding".into(), "20px".into())]
    );
    assert_eq!(
        whole::evaluate(&source, "a({p:'raw'}).props.p").element,
        "raw"
    );
}

#[rstest]
#[case("first.p='mutated';")]
#[case("const alias=first;alias.p='mutated';")]
#[serial]
fn local_objects_when_mutated_or_aliased_remain_opaque(#[case] mutation: &str) {
    let source = format!(
        "{BOX}export const a=()=>{{const first={{p:'20px'}};{mutation}return <Box p='4px' {{...first}}/>;}};"
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("padding".into(), "4px".into())]
    );
    assert_eq!(whole::evaluate(&source, "a().props.p").element, "mutated");
}

#[test]
#[serial]
fn raw_values_when_unit_policies_differ_keep_identity_without_coercion() {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=(rest)=>jsx(Box,{{p:1,animationDuration:1,opacity:0.3,'--custom':'authored',...rest}});"
    );
    for input in [
        "undefined",
        "null",
        "false",
        "true",
        "NaN",
        "Infinity",
        "-Infinity",
        "''",
        "0",
        "-0",
        "Number.MIN_VALUE",
        "Number.MAX_VALUE",
        "'02'",
        "1n",
        "Symbol('s')",
        "({[Symbol.toPrimitive](){throw 'coerced';}})",
        "(()=>0)",
    ] {
        let actual = whole::evaluate(
            &source,
            &format!(
                "(()=>{{const raw={input};const e=a({{p:raw,animationDuration:raw,opacity:raw,'--custom':raw}});return ['p','animationDuration','opacity','--custom'].every(key=>Object.hasOwn(e.props,key)&&Object.is(e.props[key],raw));}})()"
            ),
        );
        assert_eq!(actual.element, true, "{input}");
    }
    assert_eq!(
        static_styles(&output(&source)),
        vec![
            ("--custom".into(), "authored".into()),
            ("animation-duration".into(), "1ms".into()),
            ("opacity".into(), ".3".into()),
            ("padding".into(), "4px".into())
        ]
    );
}

#[test]
#[serial]
fn iterators_when_local_objects_are_spread_keep_the_original_receiver() {
    let source = format!(
        "{BOX}export const a=()=>{{const first={{p:0}};return <Box title={{[...first,...first].join(',')}}/>;}};"
    );
    let actual = whole::evaluate(
        &source,
        "(()=>{Object.prototype[Symbol.iterator]=function*(){this.p++;yield this.p;};return a().props.title;})()",
    );
    assert_eq!(actual.element, "1,2");
}
