use super::*;
use serial_test::serial;

#[test]
#[serial]
fn block_object_callbacks_when_they_return_early_or_fall_through_remain_finite() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=render=>render; const Card=styled.div({styleOrder:1,color:'red'},p=>{trace.push(p.active);if(p.active)return {styleOrder:1,color:'blue'};}); const a=Card({active:true},null); const b=Card({active:false},null);";
    let actual = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(actual.trace, serde_json::json!([true, false]));
    assert_eq!(
        actual.element[0]
            .as_str()
            .required("active callback must emit a class string")
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["color-0-blue--1-a"]
    );
    assert_eq!(
        actual.element[1]
            .as_str()
            .required("inactive callback must emit a class string")
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec!["color-0-red--1-a"]
    );
}

#[test]
#[serial]
fn recursive_orders_when_classname_has_tailwind_keep_each_source_read_once() {
    for runtime in [false, true] {
        let element = if runtime {
            "jsx(Box,{styleOrder:pick('outer')?1:2,className:pick('class')?'text-blue-500':'text-green-500',_hover:{styleOrder:pick('inner')?3:4,color:'red'}})"
        } else {
            "<Box styleOrder={pick('outer')?1:2} className={pick('class')?'text-blue-500':'text-green-500'} _hover={{styleOrder:pick('inner')?3:4,color:'red'}} />"
        };
        let source = format!(
            "{BOX}{JSX_RUNTIME}const pick=name=>(trace.push(name),true); const a={element};"
        );
        let actual = whole::evaluate(&source, "a.props.className");
        assert_eq!(actual.trace, serde_json::json!(["outer", "class", "inner"]));
        let classes = actual
            .element
            .as_str()
            .required("recursive orders must emit a class string");
        assert!(classes.contains("color-0-red-_a__c_hover-3-"), "{classes}");
        assert!(
            !(classes.contains("--2-") || classes.contains("-4-")),
            "{classes}"
        );
    }
}

#[test]
#[serial]
fn imported_alias_and_array_when_constructed_are_compared_as_saved_results() {
    for initial in [true, false] {
        let producer = format!(
            "import {{css}} from '@devup-ui/react'; let active={initial}; const check=()=>(trace.push('import'),active); const red=css({{styleOrder:check()?1:2,color:'red'}}); const other=css({{styleOrder:2,background:'black'}}); export const alias=red; export const parts=[alias,other]; active=false;"
        );
        let supplied = producer.clone();
        let resolver = move |_: &str, _: &str| {
            Some(ResolvedModule {
                path: "/producer.ts".to_string(),
                code: supplied.clone(),
            })
        };
        reset_class_map();
        reset_file_map();
        css::debug::set_debug(true);
        let consumer = extract_with_modules("/consumer.ts", "import {css} from '@devup-ui/react'; import {parts} from './producer'; const a=css(parts,{styleOrder:1,color:'blue'});", ExtractOption::default(), false, &resolver).required("imported finite array consumer must compile");
        let producer =
            extract_without_source_map("/producer.ts", &producer, ExtractOption::default())
                .required("finite array producer must compile");
        css::debug::set_debug(false);
        let actual = whole::evaluate_code(&format!("{}\n{}", producer.code, consumer.code), "a");
        assert_eq!(actual.trace, serde_json::json!(["import"]));
        let classes = actual
            .element
            .as_str()
            .required("imported finite array must emit a class string");
        assert!(classes.contains("color-0-blue--1-"), "{classes}");
        assert!(classes.contains("background-0-black--2-"), "{classes}");
        assert_eq!(classes.contains("red"), !initial, "{classes}");
    }
}
