use super::*;
use serial_test::serial;

#[test]
#[serial]
fn literal_order_when_direct_class_apis_selects_numeric_layer() {
    for usage in [
        "css`style-order:2;color:red`",
        "css('style-order:2;color:red')",
        "styled.div`style-order:2;color:red`",
        "styled.div('style-order:2;color:red')",
    ] {
        let source = format!("import {{css,styled}} from '@devup-ui/react'; const a={usage};");
        let result = output(&source);
        let orders: Vec<_> = result
            .styles
            .into_iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Static(style) => {
                    Some((style.property().to_string(), style.style_order()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(orders, vec![("color".to_string(), Some(2))], "{usage}");
    }
}

#[test]
#[serial]
fn literal_global_when_static_order_uses_global_atoms() {
    let result = output(
        "import {globalCss} from '@devup-ui/react'; globalCss`body{style-order:2;color:red}`;",
    );
    let orders: Vec<_> = result
        .styles
        .into_iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property().to_string(), style.style_order()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(orders, vec![("color".to_string(), Some(2))]);
}

#[test]
#[serial]
fn literal_keyframes_when_metadata_present_reports_no_effect() {
    let result = compile(
        "import {keyframes} from '@devup-ui/react'; const a=keyframes`from{style-order:2;color:red}`;",
    );
    assert!(
        result
            .required_err("keyframe metadata must have no effect")
            .contains("has no effect")
    );
}

#[test]
#[serial]
fn literal_order_when_finite_getter_is_captured_runs_once() {
    let source = "import {css} from '@devup-ui/react'; const config={get active(){trace.push('order');return true}}; const a=css`style-order:${config.active?2:3};color:red`;";
    let result = whole::evaluate(source, "a");
    assert_eq!(result.trace, serde_json::json!(["order"]));
    assert!(
        result
            .element
            .as_str()
            .required("finite literal order must emit classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn literal_order_when_scopes_override_inherits_only_missing_orders() {
    let source = "import {css} from '@devup-ui/react'; const a=css`color:red;&:hover{color:blue;style-order:3;}@media print{color:green;}style-order:2;`";
    let result = output(source);
    let mut orders: Vec<_> = result
        .styles
        .into_iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.value().to_string(), style.style_order()))
            }
            _ => None,
        })
        .collect();
    orders.sort();
    assert_eq!(
        orders,
        vec![
            ("blue".to_string(), Some(3)),
            ("green".to_string(), Some(2)),
            ("red".to_string(), Some(2))
        ]
    );
}

#[test]
#[serial]
fn literal_order_when_root_mixin_splits_text_keeps_scope() {
    let source = "import {css} from '@devup-ui/react'; const base=css({color:'red'}); const a=css`background:blue;${base};style-order:2;border-color:green;`";
    let result = output(source);
    let mut orders: Vec<_> = result
        .styles
        .into_iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.style_order() == Some(2) => {
                Some(style.property().to_string())
            }
            _ => None,
        })
        .collect();
    orders.sort();
    assert_eq!(orders, vec!["background", "border-color", "color"]);
}

#[test]
#[serial]
fn literal_order_when_mixed_finite_text_uses_canonical_strings() {
    let source = "import {css} from '@devup-ui/react'; const config={get active(){trace.push('mixed');return false}}; const a=css`style-order:1${config.active?2:3};color:red`;";
    let result = whole::evaluate(source, "a");
    assert_eq!(result.trace, serde_json::json!(["mixed"]));
    assert!(
        result
            .element
            .as_str()
            .required("finite mixed text must emit classes")
            .contains("--13-")
    );
}

#[test]
#[serial]
fn literal_order_when_primitive_values_are_not_numbers_rejects_without_coercion() {
    for value in [
        "${'02'}",
        "${' 2'}",
        "${null}",
        "${true}",
        "0",
        "255",
        "2px",
        "2 !important",
        "'02'",
    ] {
        let source = format!(
            "import {{css}} from '@devup-ui/react'; const a=css`style-order:{value};color:red`; "
        );
        assert!(
            compile(&source)
                .required_err("noncanonical primitive order must be invalid")
                .contains("styleOrder"),
            "{value}"
        );
    }
}

#[test]
#[serial]
fn literal_order_when_quoted_content_contains_directives_does_not_parse_them() {
    let source = concat!(
        "import {css} from '@devup-ui/react'; const a=css`/*style-order:7*/content:';style-order:8;{",
        "}';--style-order:9;[style-order='2']{color:blue}style-order:2;`"
    );
    let result = output(source);
    let mut names: Vec<_> = result
        .styles
        .into_iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.property().to_string()),
            _ => None,
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["--style-order", "color", "content"]);
}

#[test]
#[serial]
fn literal_global_when_conditional_order_has_no_runtime_selector_rejects() {
    let source = "import {globalCss} from '@devup-ui/react'; const a=(on)=>globalCss`body{style-order:${on?2:3};color:red}`";
    assert!(
        compile(source)
            .required_err("runtime global order must be invalid")
            .contains("global styles require a static order")
    );
}

#[test]
#[serial]
fn literal_global_when_nested_descriptors_have_orders_rejects_no_effect() {
    for body in [
        "@font-face{style-order:2;font-family:test}",
        "@keyframes spin{from{style-order:2;opacity:0}}",
    ] {
        let source = format!(
            "import {{globalCss}} from '@devup-ui/react'; globalCss`@media print{{{body}}}`"
        );
        assert!(
            compile(&source)
                .required_err("nested descriptor metadata must be invalid")
                .contains("has no effect")
        );
    }
}

#[test]
#[serial]
fn literal_styled_when_order_is_construction_time_survives_retarget() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; let active=true; const config={get active(){trace.push('construct');return active}}; const Card=styled.div`style-order:${config.active?2:3};color:red`; const Other=Card.withComponent('section'); active=false; const a=Other({},null); const b=Card({},null);";
    let result = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(result.trace, serde_json::json!(["construct"]));
    assert_eq!(result.element[0], result.element[1]);
    assert!(
        result.element[0]
            .as_str()
            .required("retargeted literal component must emit classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn literal_order_when_overwritten_and_empty_keeps_effects_in_source_order() {
    let source = "import {css} from '@devup-ui/react'; const flag=(name)=>(trace.push(name),true); const a=css`style-order:${flag('old')?2:3};color:${flag('color')?'red':'blue'};style-order:${flag('new')?4:4};`; const b=css`style-order:${flag('empty')?2:3}`;";
    let result = whole::evaluate(source, "[a,b]");
    assert_eq!(
        result.trace,
        serde_json::json!(["old", "color", "new", "empty"])
    );
    assert!(
        result.element[0]
            .as_str()
            .required("overwritten literal order must emit classes")
            .contains("--4-")
    );
    assert_eq!(result.element[1], "");
}

#[test]
#[serial]
fn literal_styled_when_order_callback_is_finite_selects_each_render() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const flag=(value)=>(trace.push(value),value); const Card=styled.div`style-order:${p=>flag(p.active)?2:3};color:red`; const a=Card({active:true},null); const b=Card({active:false},null);";
    let result = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(result.trace, serde_json::json!([true, false]));
    assert!(
        result.element[0]
            .as_str()
            .required("active literal callback must emit classes")
            .contains("--2-")
    );
    assert!(
        result.element[1]
            .as_str()
            .required("inactive literal callback must emit classes")
            .contains("--3-")
    );
}
