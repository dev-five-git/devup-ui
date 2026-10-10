use crate::assignment_blocker_support::selected;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "state.flag?false:0",
    true,
    "[]",
    "[\"before\",\"flag\",\"after\",\"child\"]"
)]
#[case(
    "state.flag?false:0",
    false,
    "[\"padding:0:0\"]",
    "[\"before\",\"flag\",\"after\",\"child\"]"
)]
#[case(
    "state.flag&&state.zero",
    true,
    "[\"padding:0:0\"]",
    "[\"before\",\"flag\",\"zero\",\"after\",\"child\"]"
)]
#[case(
    "state.flag&&state.zero",
    false,
    "[]",
    "[\"before\",\"flag\",\"after\",\"child\"]"
)]
#[case(
    "({yes:false,no:0})[state.key]",
    true,
    "[]",
    "[\"before\",\"key\",\"after\",\"child\"]"
)]
#[case(
    "({yes:false,no:0})[state.key]",
    false,
    "[\"padding:0:0\"]",
    "[\"before\",\"key\",\"after\",\"child\"]"
)]
#[serial]
fn composite_controller_uses_its_raw_falsy_value_when_branch_shapes_overlap(
    #[case] controller: &str,
    #[case] flag: bool,
    #[case] expected: &str,
    #[case] trace: &str,
) {
    for expression in [
        format!(
            "<Box id={{state.before}} p={{({controller})&&state.right}} title={{state.after}}>{{state.child}}</Box>"
        ),
        format!(
            "jsx(Box,{{id:state.before,p:({controller})&&state.right,title:state.after,children:state.child}})"
        ),
    ] {
        let source = format!(
            "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{}};for(const[key,value]of Object.entries({{before:'before',flag:{flag},zero:0,key:{flag}?'yes':'no',after:'after',child:'child'}}))Object.defineProperty(state,key,{{get(){{trace.push(key);return value}}}});Object.defineProperty(state,'right',{{get(){{throw Error('unselected right')}}}});const jsx=(tag,props)=>props;const node=render(state);"
        );
        let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
        assert_eq!(actual, format!("[{trace},{expected}]"));
    }
}

#[rstest]
#[case(true, "[]")]
#[case(false, "[\"padding:0:0\"]")]
#[serial]
fn css_composite_controller_keeps_zero_selection_when_the_controller_is_conditional(
    #[case] flag: bool,
    #[case] expected: &str,
) {
    let source = format!(
        "import {{css}}from '@devup-ui/react';function render(state){{return css({{p:(state.flag?false:0)&&'16px'}})}}let reads=0;const state={{get flag(){{reads++;return {flag}}}}};const node={{className:render(state)}};"
    );
    let actual = selected(&source, "JSON.stringify([reads,assigned(node)]);");
    assert_eq!(actual, format!("[1,{expected}]"));
}
