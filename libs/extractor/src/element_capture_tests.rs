use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn nested_types_keep_authored_order_when_getters_change_results() {
    // Given: the rejected exact nested shape, with changing global identifier getters.
    let source = "import {Box}from '@devup-ui/react';function render(){return <Box as={x}><Box as={y} p={1}/></Box>}let trace=[],reads=0;for(const key of ['x','y'])Object.defineProperty(globalThis,key,{get(){trace.push(key);return ++reads===1?'section':'article'}});const node=render();";
    // When: execute emitted captures and join the child's selected class to padding.
    let actual = selected(
        source,
        "JSON.stringify([trace,reads,node.tag,node.children[0].tag,assigned(node.children[0])]);",
    );
    // Then: x precedes y once each, and the selected inner type retains padding4.
    assert_eq!(
        actual,
        "[[\"x\",\"y\"],2,\"section\",\"article\",[\"padding:4px:0\"]]"
    );
}

#[rstest]
#[case("<Box as={state.x}><Box as={state.y} p={1}/></Box>")]
#[case("jsx(Box,{as:state.x,children:jsx(Box,{as:state.y,p:1})})")]
#[serial]
fn nested_type_stops_before_child_when_outer_getter_throws(#[case] expression: &str) {
    // Given: an outer type getter that fails before the inner type is reached.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[],error;const state={{get x(){{trace.push('x');throw Error('outer')}},get y(){{trace.push('y');return 'a'}}}};const jsx=(tag,props)=>({{...props,tag}});try{{render(state)}}catch(e){{error=e.message}}JSON.stringify([trace,error]);"
    );
    // When: execute the transformed expression.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: a child cannot run ahead of the throwing authored outer operand.
    assert_eq!(actual, "[[\"x\"],\"outer\"]");
}

#[rstest]
#[case("<Box as={state.x}><Text as={state.y}><Box as={state.z} p={1}/></Text></Box>")]
#[case("jsx(Box,{as:state.x,children:jsx(Text,{as:state.y,children:jsx(Box,{as:state.z,p:1})})})")]
#[serial]
fn nested_types_keep_three_levels_when_defaults_differ(#[case] expression: &str) {
    // Given: three authored type sites whose values change on every getter read.
    let source = format!(
        "import {{Box,Text}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[],reads=0;const state={{}};for(const key of ['x','y','z'])Object.defineProperty(state,key,{{get(){{trace.push(key);return ++reads===3?'article':null}}}});const jsx=(tag,props)=>({{...props,tag,children:props.children?[props.children]:[]}});const node=render(state);"
    );
    // When: observe all three selected types and the innermost class association.
    let actual = selected(
        &source,
        "const middle=node.children[0],inner=middle.children[0];JSON.stringify([trace,reads,node.tag,middle.tag,inner.tag,assigned(inner)]);",
    );
    // Then: fallbacks remain component-specific while authored nesting stays ordered.
    assert_eq!(
        actual,
        "[[\"x\",\"y\",\"z\"],3,\"div\",\"span\",\"article\",[\"padding:4px:0\"]]"
    );
}

#[rstest]
#[case(
    "<Box id={state.id} className={state.className} style={state.style} p={state.pad} as={state.type} {...{title:state.spread.title}}>{state.child}</Box>",
    "Box",
    "div"
)]
#[case(
    "<Text id={state.id} className={state.className} style={state.style} p={state.pad} as={state.type} {...{title:state.spread.title}}>{state.child}</Text>",
    "Text",
    "span"
)]
#[case(
    "jsx(Box,{id:state.id,className:state.className,style:state.style,p:state.pad,as:state.type,...{title:state.spread.title},children:state.child})",
    "Box",
    "div"
)]
#[case(
    "jsx(Text,{id:state.id,className:state.className,style:state.style,p:state.pad,as:state.type,...{title:state.spread.title},children:state.child})",
    "Text",
    "span"
)]
#[serial]
fn captured_type_keeps_sibling_order_when_styles_and_spreads_are_present(
    #[case] expression: &str,
    #[case] component: &str,
    #[case] default: &str,
) {
    // Given: type is authored after ordinary/style operands and before a spread/child.
    // A written title key keeps unknown CSS-spread composition in the #756 gate.
    let source = format!(
        "import {{{component}}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{}};for(const[key,value]of Object.entries({{id:'target',className:'',style:{{opacity:'0.5'}},pad:'16px',type:false,spread:{{title:'spread'}},child:'text'}}))Object.defineProperty(state,key,{{get(){{trace.push(key);return value}}}});const jsx=(tag,props)=>({{...props,tag}});const node=render(state);"
    );
    // When: observe execution and selected padding rather than generated spellings.
    let actual = selected(
        &source,
        "JSON.stringify([trace,node.tag,node.id,node.title,node.style.opacity,assigned(node)]);",
    );
    // Then: fallback adds no extra read and cannot move type before its siblings.
    assert_eq!(
        actual,
        format!(
            "[[\"id\",\"className\",\"style\",\"pad\",\"type\",\"spread\",\"child\"],\"{default}\",\"target\",\"spread\",\"0.5\",[\"padding:16px:0\"]]"
        )
    );
}

#[rstest]
#[case("<Box id={state.id} as={state.type}><Box as={state.inner} p={1}/></Box>")]
#[case("jsx(Box,{id:state.id,as:state.type,children:jsx(Box,{as:state.inner,p:1})})")]
#[serial]
fn captured_type_skips_later_reads_when_ordinary_sibling_throws(#[case] expression: &str) {
    // Given: an ordinary getter throws before both the outer type and the child.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[],error;const state={{get id(){{trace.push('id');throw Error('sibling')}},get type(){{trace.push('type');return 'a'}},get inner(){{trace.push('inner');return 'b'}}}};const jsx=(tag,props)=>props;try{{render(state)}}catch(e){{error=e.message}}JSON.stringify([trace,error]);"
    );
    // When: execute the transformed source.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: source timing, not type-priority scheduling, controls the exception.
    assert_eq!(actual, "[[\"id\"],\"sibling\"]");
}

#[rstest]
#[case("0")]
#[case("false")]
#[case("null")]
#[case("undefined")]
#[case("''")]
#[serial]
fn selected_type_defaults_when_runtime_value_is_falsy(#[case] value: &str) {
    // Given: literal object/array selections and scalar types on both caller paths.
    let expressions = [
        "<Box as={state.type} id={state.id} p={1}/>",
        "<Text as={state.type} id={state.id} p={1}/>",
        "<Box as={{a:state.type}[state.key]} id={state.id} p={1}/>",
        "<Text as={[state.type][state.index]} id={state.id} p={1}/>",
        "jsx(Box,{as:state.type,id:state.id,p:1})",
        "jsx(Text,{as:{a:state.type}[state.key],id:state.id,p:1})",
        "jsx(Text,{as:[state.type][state.index],id:state.id,p:1})",
    ];
    for expression in expressions {
        let source = format!(
            "import {{Box,Text}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let reads=0;const state={{get type(){{reads++;return {value}}},id:'target',key:'a',index:0}};const jsx=(tag,props)=>({{...props,tag}});const node=render(state);"
        );
        // When: run the emitted selected type with a falsy authored value.
        let actual = selected(&source, "JSON.stringify([reads,node.tag,assigned(node)]);");
        // Then: all falsy values use the component's default without losing padding.
        let default = if expression.contains("Text") {
            "span"
        } else {
            "div"
        };
        assert_eq!(
            actual,
            format!("[1,\"{default}\",[\"padding:4px:0\"]]"),
            "{expression}"
        );
    }
}

#[rstest]
#[case("<Box as={{a:'a',b:'button'}[state.key]} id={state.id} p={1}/>", "div")]
#[case("<Text as={['div','a'][state.index]} id={state.id} p={1}/>", "span")]
#[case("jsx(Box,{as:{a:'a',b:'button'}[state.key],id:state.id,p:1})", "div")]
#[case("jsx(Text,{as:['div','a'][state.index],id:state.id,p:1})", "span")]
#[serial]
fn literal_member_type_defaults_when_selected_key_is_missing(
    #[case] expression: &str,
    #[case] default: &str,
) {
    // Given: the existing literal-member type shapes with missing object/array keys.
    let source = format!(
        "import {{Box,Text}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{get key(){{trace.push('key');return 'missing'}},get index(){{trace.push('key');return 7}},get id(){{trace.push('id');return 'target'}}}};const jsx=(tag,props)=>({{...props,tag}});const node=render(state);"
    );
    // When: execute the actual member selection and selected class.
    let actual = selected(&source, "JSON.stringify([trace,node.tag,assigned(node)]);");
    // Then: undefined selection defaults, retaining key-before-id timing and padding4.
    assert_eq!(
        actual,
        format!("[[\"key\",\"id\"],\"{default}\",[\"padding:4px:0\"]]")
    );
}

#[rstest]
#[case("state.flag?<Box as={state.x}><Text as={state.y} p={1}/></Box>:null")]
#[case("state.flag&&<Box as={state.x}><Text as={state.y} p={1}/></Box>")]
#[case("state.flag?jsx(Box,{as:state.x,children:jsx(Text,{as:state.y,p:1})}):null")]
#[serial]
fn nested_type_capture_stays_lazy_when_branch_is_not_selected(#[case] expression: &str) {
    // Given: an unselected branch containing both type getters.
    let source = format!(
        "import {{Box,Text}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{flag:false,get x(){{trace.push('x');throw Error('unused')}},get y(){{trace.push('y');throw Error('unused')}}}};const jsx=(tag,props)=>props;render(state);JSON.stringify(trace);"
    );
    // When: execute the false branch.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: wrapper metadata never moves captures outside their lazy source scope.
    assert_eq!(actual, "[]");
}

#[rstest]
#[case("0")]
#[case("false")]
#[case("null")]
#[case("undefined")]
#[case("''")]
#[serial]
fn literal_member_type_defaults_when_selected_entry_is_falsy(#[case] value: &str) {
    // Given: literal entries rather than runtime getters, selected by a runtime key.
    let expressions = [
        format!("<Box as={{{{a:{value}}}[state.key]}} id={{state.id}} p={{1}}/>"),
        format!("<Text as={{[{value}][state.index]}} id={{state.id}} p={{1}}/>"),
        format!("jsx(Box,{{as:{{a:{value}}}[state.key],id:state.id,p:1}})"),
        format!("jsx(Text,{{as:[{value}][state.index],id:state.id,p:1}})"),
    ];
    for expression in expressions {
        let source = format!(
            "import {{Box,Text}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{get key(){{trace.push('key');return 'a'}},get index(){{trace.push('key');return 0}},get id(){{trace.push('id');return 'target'}}}};const jsx=(tag,props)=>({{...props,tag}});const node=render(state);"
        );
        // When: execute the selected literal entry and its associated class.
        let actual = selected(&source, "JSON.stringify([trace,node.tag,assigned(node)]);");
        // Then: literal-member lowering preserves the same established default.
        let default = if expression.contains("Text") {
            "span"
        } else {
            "div"
        };
        assert_eq!(
            actual,
            format!("[[\"key\",\"id\"],\"{default}\",[\"padding:4px:0\"]]")
        );
    }
}

#[rstest]
#[case("Button", "button")]
#[case("Image", "img")]
#[case("Input", "input")]
#[serial]
fn captured_type_defaults_to_component_tag_when_value_is_empty(
    #[case] component: &str,
    #[case] default: &str,
) {
    // Given: the remaining distinct component default tags on both caller paths.
    for expression in [
        format!("<{component} as={{state.type}} id={{state.id}}/>"),
        format!("jsx({component},{{as:state.type,id:state.id}})"),
    ] {
        let source = format!(
            "import {{{component}}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let reads=0;const state={{get type(){{reads++;return ''}},id:'target'}};const jsx=(tag,props)=>({{...props,tag}});const node=render(state);JSON.stringify([reads,node.tag]);"
        );
        // When: execute the captured falsy type.
        let actual = evaluate(&compiled_jsx(&source));
        // Then: capture never hardcodes Box's div fallback for another component.
        assert_eq!(actual, format!("[1,\"{default}\"]"));
    }
}

#[rstest]
#[case("0")]
#[case("false")]
#[case("null")]
#[case("undefined")]
#[case("''")]
#[serial]
fn jsx_runtime_type_defaults_when_authored_literal_is_falsy(#[case] value: &str) {
    // Given: falsy literal types, including fixed-key literal member selections.
    for (component, default) in [("Box", "div"), ("Text", "span")] {
        for tag in [
            value.to_string(),
            format!("({{a:{value}}})['a']"),
            format!("[{value}][0]"),
        ] {
            let source = format!(
                "import {{{component}}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return jsx({component},{{id:state.id,as:{tag},p:1}})}}let trace=[];const state={{get id(){{trace.push('id');return 'target'}}}};const jsx=(tag,props)=>({{...props,tag}});const node=render(state);"
            );
            // When: execute the authored literal and its selected padding class.
            let actual = selected(&source, "JSON.stringify([trace,node.tag,assigned(node)]);");
            // Then: CSS literal stringification cannot stand in for type defaulting.
            assert_eq!(
                actual,
                format!("[[\"id\"],\"{default}\",[\"padding:4px:0\"]]")
            );
        }
    }
}
