use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box color='red' {...rest}/>")]
#[case("<Box {...rest} color='red'/>")]
#[case("jsx(Box,{color:'red',...rest})")]
#[case("jsx(Box,{...rest,color:'red'})")]
#[case("createElement(Box,{color:'red',...rest})")]
#[case("createElement(Box,{...rest,color:'red'})")]
#[serial]
fn authored_styles_when_unknown_keys_collide_keep_raw_props(#[case] element: &str) {
    let control = output(&format!("{BOX}export const a=()=> <Box color='red'/>;"));
    let source = format!(
        "{BOX}{JSX_RUNTIME}import {{createElement}} from 'react';export const a=(rest)=>{element};"
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
        "2",
        "'2'",
        "'.5'",
        "'1.e2'",
        "'0x10'",
        "'4\\n'",
        "({toString(){throw 'coerced';}})",
        "Symbol('raw')",
        "1n",
        "(()=>1)",
    ] {
        let actual = whole::evaluate(
            &source,
            &format!(
                "(()=>{{const raw={input};const e=a({{color:raw}});return [e.type,Object.hasOwn(e.props,'color'),Object.is(e.props.color,raw),Object.keys(e.props.style||{{}}),e.props.className];}})()"
            ),
        );
        assert_eq!(actual.element[0], "div");
        assert_eq!(actual.element[1], true);
        assert_eq!(actual.element[2], true);
        assert_eq!(actual.element[3], serde_json::json!([]));
        assert_eq!(
            actual.element[4]
                .as_str()
                .unwrap_or_else(|| panic!("expected an authored class name"))
                .trim(),
            "color-0-red--255-a",
            "{element}/{input}"
        );
        assert_eq!(static_styles(&output(&source)), static_styles(&control));
        assert_eq!(output(&source).styles.len(), control.styles.len());
    }
}

#[rstest]
#[case("<Box color='red' {...{color:'blue',...rest}}/>")]
#[case("<Box color='red' {...{...rest,color:'blue'}}/>")]
#[case("<Box color='red' {...{get color(){trace.push('get');return 'blue';}}}/>")]
#[case("<Box color='red' {...{color:'blue',[key]:'kept'}}/>")]
#[case("jsx(Box,{color:'red',...{color:'blue',...rest}})")]
#[case("jsx(Box,{color:'red',[key]:'blue'})")]
#[serial]
fn opaque_objects_when_named_keys_are_visible_stay_whole(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest,key)=>{element};");
    let actual = whole::evaluate(&source, "a({},'title')");
    if element.contains("[key]:'blue'") {
        assert_eq!(actual.element["props"]["title"], "blue");
    } else {
        assert_eq!(actual.element["props"]["color"], "blue");
    }
    assert_eq!(
        actual.element["props"]["className"]
            .as_str()
            .unwrap_or_else(|| panic!("expected an authored class name"))
            .trim(),
        "color-0-red--255-a"
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "red".into())]
    );
}

#[rstest]
#[case("<Box _hover={{color:'red'}} {...rest}/>")]
#[case("<Box {...rest} _hover={{color:'red'}}/>")]
#[case("jsx(Box,{_hover:{color:'red'},...rest})")]
#[serial]
fn selectors_when_unknown_spreads_follow_compile_independently(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest)=>{element};");
    let actual = whole::evaluate(
        &source,
        "(()=>{const raw={color:'blue'};const e=a({_hover:raw});return [e.props._hover===raw,e.props.className];})()",
    );
    assert_eq!(actual.element[0], true);
    assert_eq!(
        actual.element[1]
            .as_str()
            .unwrap_or_else(|| panic!("expected a selector class name"))
            .trim(),
        "color-0-red-_a__c_hover-255-a"
    );
}

#[rstest]
#[case("jsx", "react/jsx-runtime")]
#[case("jsxs", "react/jsx-runtime")]
#[case("jsxDEV", "react/jsx-dev-runtime")]
#[serial]
fn wrapped_factory_props_when_no_unknown_spread_exists_attach_authored_styles(
    #[case] factory: &str,
    #[case] module: &str,
) {
    for wrapper in [
        "BODY",
        "(BODY)",
        "((BODY))",
        "(BODY as const)",
        "(BODY satisfies object)",
    ] {
        let props = wrapper.replace("BODY", "{color:first()}");
        let source = format!(
            "{BOX}import {{{factory} as jsx}} from '{module}';export const a=(first)=>jsx(Box,{props});"
        );
        let actual = whole::evaluate(
            &source,
            "(()=>{const e=a(()=>{trace.push('first');return 'red';});return [e.type,(e.props.className||'').length>0,Object.values(e.props.style||{})];})()",
        );
        assert_eq!(actual.trace, serde_json::json!(["first"]));
        assert_eq!(
            actual.element,
            serde_json::json!(["div", true, ["red"]]),
            "{factory}/{wrapper}"
        );
    }
}
