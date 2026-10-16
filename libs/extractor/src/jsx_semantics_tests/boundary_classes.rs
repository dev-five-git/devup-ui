use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn class_boundary_when_isolated_member_call_runs_keeps_receiver_getter_and_iterator_once() {
    let source = format!(
        "{BOX}export const a=(receiver,args)=><Box className={{receiver.read(...args)}}/>;"
    );
    let actual = evaluate(
        &source,
        "a({marker:'receiver',get read(){trace.push('get');return function(value){trace.push(this.marker+':'+value);return 'opaque';};}},{[Symbol.iterator](){trace.push('iterator');return ['arg'][Symbol.iterator]();}})",
    );
    assert_eq!(actual.element["props"]["className"], "opaque");
    assert_eq!(
        actual.trace,
        serde_json::json!(["get", "iterator", "receiver:arg"])
    );
    assert!(!code(&source).contains("__devupValue"));
}

#[rstest]
#[case("{get read(){trace.push('get');throw 'boom';}}", "[]", serde_json::json!(["get"]))]
#[case("{get read(){trace.push('get');return ()=>{trace.push('call');};}}", "{[Symbol.iterator](){trace.push('iterator');throw 'boom';}}", serde_json::json!(["get","iterator"]))]
#[case("{get read(){trace.push('get');return function(){trace.push('call');throw 'boom';};}}", "{[Symbol.iterator](){trace.push('iterator');return [][Symbol.iterator]();}}", serde_json::json!(["get","iterator","call"]))]
#[serial]
fn class_boundary_when_call_parts_throw_preserves_the_original_cutoff(
    #[case] receiver: &str,
    #[case] args: &str,
    #[case] trace: serde_json::Value,
) {
    let source = format!(
        "{BOX}export const a=(receiver,args)=><Box className={{receiver.read(...args)}}/>;"
    );
    let actual = evaluate(
        &source,
        &format!("(()=>{{try{{return a({receiver},{args});}}catch(error){{return error;}}}})()"),
    );
    assert_eq!(actual.element, "boom");
    assert_eq!(actual.trace, trace);
}

#[rstest]
#[case("<Box className={read()} id={later()}/>", "'id'", "early")]
#[case(
    "<Box className={read()} {...later()}/>",
    "{className:'override'}",
    "override"
)]
#[case(
    "<Box className={read()} className={later()}/>",
    "'override'",
    "override"
)]
#[serial]
fn class_boundary_when_other_attributes_can_interfere_still_captures(
    #[case] element: &str,
    #[case] later: &str,
    #[case] expected: &str,
) {
    let source = format!("{BOX}export const a=(read,later)=>{element};");
    let actual = evaluate(
        &source,
        &format!(
            "(()=>{{let value='early';return a(()=>{{trace.push('read');return value;}},()=>{{trace.push('later');value='late';return {later};}});}})()"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["read", "later"]));
    assert_eq!(actual.element["props"]["className"], expected);
    assert!(code(&source).contains("__devupValue"));
}

#[test]
#[serial]
fn class_boundary_when_style_order_expands_calls_still_evaluates_once() {
    let source = format!(
        "{BOX}export const a=(read,active)=><Box className={{read()}} styleOrder={{active?1:2}}/>;"
    );
    let actual = evaluate(&source, "a(()=>{trace.push('read');return 'opaque';},true)");
    assert_eq!(actual.trace, serde_json::json!(["read"]));
    assert_eq!(actual.element["props"]["className"], "opaque");
    assert!(code(&source).contains("__devupValue"));
}

#[test]
#[serial]
fn class_boundary_when_child_yields_keeps_the_class_before_suspension() {
    let source = format!(
        "{BOX}export function* a(read){{return <Box className={{read()}}>{{yield (trace.push('yield'),'pause')}}</Box>;}}"
    );
    let actual = evaluate(
        &source,
        "(()=>{const iterator=a(()=>{trace.push('read');return 'opaque';});iterator.next();trace.push('resume');return iterator.next('child').value;})()",
    );
    assert_eq!(actual.trace, serde_json::json!(["read", "yield", "resume"]));
    assert_eq!(actual.element["children"], serde_json::json!(["child"]));
    assert!(code(&source).contains("__devupValue"));
}

#[test]
#[serial]
fn class_boundary_when_only_a_css_value_calls_still_captures_its_repeated_read() {
    let source = format!("{BOX}export const a=(read)=><Box p={{read()}}/>;");
    let actual = evaluate(&source, "a(()=>{trace.push('read');return 2;})");
    assert_eq!(actual.trace, serde_json::json!(["read"]));
    assert!(code(&source).contains("__devupValue"));
}

#[rstest]
#[case("<Box className={`${read()}`}/>")]
#[case("<Box className={(read())}/>")]
#[case("<Box className={read()}>{'child'}</Box>")]
#[case("<Card className={read()}/>")]
#[serial]
fn class_boundary_when_root_or_route_is_not_isolated_remains_captured(#[case] element: &str) {
    let source = format!(
        "import {{Box,styled}} from '@devup-ui/react';const Card=styled(Box,{{color:'red'}});export const a=(read)=>{element};"
    );
    assert!(code(&source).contains("__devupValue"));
}

#[test]
#[serial]
fn class_boundary_when_outer_text_is_forced_to_capture_compiles_before_later_spread() {
    let source = format!(
        "{BOX}const base=`p-4`;export const a=(clsx,on,rest)=><Box className={{clsx(base,on&&'m-2')}} {{...rest}}/>;"
    );
    let compiled = output(&source);
    assert_eq!(
        static_styles(&compiled),
        vec![
            ("margin".into(), ".5rem".into()),
            ("padding".into(), "1rem".into())
        ]
    );
    let actual = evaluate(
        &source,
        "a((...parts)=>{trace.push('class');return parts.filter(Boolean).join(' ');},true,{get className(){trace.push('spread');return 'override';}})",
    );
    assert_eq!(actual.trace, serde_json::json!(["class", "spread"]));
    assert_eq!(actual.element["props"]["className"], "override");
}

#[test]
#[serial]
fn class_boundary_when_parameter_shadows_text_and_id_reads_outer_binding_preserves_both() {
    let source = format!(
        "{BOX}const base='p-4';export const a=(clsx,rest)=><Box id={{base}} className={{clsx(base)}} {{...rest}}/>;export const b=(base,clsx,rest)=><Box id={{base}} className={{clsx(base)}} {{...rest}}/>;"
    );
    let actual = evaluate(
        &source,
        "(()=>{const clsx=(value)=>value;return [a(clsx,{}).props,b('shadow',clsx,{}).props];})()",
    );
    assert_eq!(actual.element[0]["id"], "p-4");
    assert_eq!(actual.element[0]["className"], "padding-0-1rem--255-a");
    assert_eq!(actual.element[1]["id"], "shadow");
    assert_eq!(actual.element[1]["className"], "shadow");
}

#[test]
#[serial]
fn class_boundary_when_call_argument_yields_retains_capture_in_the_generator_scope() {
    let source = format!(
        "{BOX}export function* a(read){{return <Box className={{read(yield (trace.push('yield'),'pause'))}}/>;}}"
    );
    let actual = evaluate(
        &source,
        "(()=>{const iterator=a(value=>{trace.push('call:'+value);return 'opaque';});iterator.next();trace.push('resume');return iterator.next('arg').value;})()",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["yield", "resume", "call:arg"])
    );
    assert_eq!(actual.element["props"]["className"], "opaque");
    assert!(code(&source).contains("__devupValue"));
}

#[test]
#[serial]
fn class_boundary_when_merge_reads_outer_text_before_spread_keeps_only_final_padding() {
    let source = format!(
        "{BOX}const base='p-4';export const a=(rest)=><Box className={{cn(base,'p-8')}} {{...rest}}/>;"
    );
    let compiled = output(&source);
    assert_eq!(
        static_styles(&compiled),
        vec![("padding".into(), "2rem".into())]
    );
    let actual = evaluate(
        &source,
        "a({get className(){trace.push('spread');return 'override';}})",
    );
    assert_eq!(actual.trace, serde_json::json!(["spread"]));
    assert_eq!(actual.element["props"]["className"], "override");
}
