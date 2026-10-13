use super::*;

fn usages(value: &str) -> Vec<String> {
    vec![
        format!("css({{styleOrder:{value},color:'red'}})"),
        format!("styled('div',{{styleOrder:{value},color:'red'}})"),
        format!("<Box styleOrder={{{value}}} color='red'/>"),
        format!("jsx(Box,{{styleOrder:{value},color:'red'}})"),
        format!("css({{_hover:{{styleOrder:{value},color:'red'}}}})"),
        format!("css({{styleOrder:2,...{{styleOrder:{value}}},color:'red'}})"),
    ]
}

#[test]
#[serial]
fn metadata_when_false_and_is_absent_survives_all_producers() {
    for usage in usages("false&&2") {
        let source = format!(
            "{BOX}{JSX_RUNTIME}import {{css,styled}} from '@devup-ui/react';const a={usage};"
        );
        assert_eq!(
            orders(&source),
            vec![("color".to_string(), 0, None)],
            "{usage}"
        );
    }
}

#[test]
#[serial]
fn metadata_when_unselected_literal_is_invalid_keeps_its_location() {
    for value in ["true?2:'01'", "false&&'01'", "false"] {
        for usage in usages(value) {
            let source = format!(
                "{BOX}{JSX_RUNTIME}import {{css,styled}} from '@devup-ui/react';\nconst a={usage};"
            );
            let line = source.lines().count();
            let column = source
                .lines()
                .last()
                .required("fixture line")
                .find(if value == "false" { "false" } else { "'01'" })
                .required("invalid literal")
                + 1;
            let actual = error(&source);
            assert!(
                actual.starts_with(&format!("a.tsx:{line}:{column}:")),
                "{usage}: {actual}"
            );
        }
    }
}

#[test]
#[serial]
fn metadata_when_constants_are_substituted_retains_absence_and_numeric_orders() {
    let source = "import {css} from '@devup-ui/react';const guard=false;const order=2;const a=css({styleOrder:guard&&order,color:'red'});const b=css({styleOrder:true?order:3,width:4});";
    assert_eq!(
        orders(source),
        vec![
            ("color".to_string(), 0, None),
            ("width".to_string(), 0, Some(2))
        ]
    );
}

#[test]
#[serial]
fn metadata_when_emotion_inlines_numbers_keeps_order_unitless() {
    let source = "import {ClassNames} from '@emotion/react';const order=2;const render=external=><ClassNames>{({css})=>css({styleOrder:order,width:3})}</ClassNames>;";
    let compiled = compile_emotion(source).required("Emotion metadata must compile");
    let actual = whole::evaluate_code(&compiled.code, "render('')");
    assert_eq!(actual.element, "width-0-3px--2-a");
}

#[test]
#[serial]
fn metadata_when_computed_finite_spread_overrides_selects_last_order() {
    let source = "import {css} from '@devup-ui/react';const a=css({styleOrder:2,...{[`style${'Order'}`]:3},color:'red'});";
    assert_eq!(orders(source), vec![("color".to_string(), 0, Some(3))]);
}

#[test]
#[serial]
fn metadata_when_computed_finite_spread_is_invalid_locates_literal() {
    let source = "import {css} from '@devup-ui/react';\nconst a=css({styleOrder:2,...{[`style${'Order'}`]:'01'},color:'red'});";
    let column = source
        .lines()
        .nth(1)
        .required("fixture line")
        .find("'01'")
        .required("invalid literal")
        + 1;
    let actual = error(source);
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}

#[test]
#[serial]
fn metadata_when_opaque_spreads_forward_props_filters_only_copied_metadata() {
    for usage in [
        "jsx(Box,{...config,styleOrder:3,color:'red'})",
        "<Box {...config} styleOrder={3} color='red'/>",
    ] {
        let source = format!(
            "{BOX}{JSX_RUNTIME}const symbol=Symbol('ordinary');const config={{get styleOrder(){{trace.push('order');return 2}},get ['style-order'](){{trace.push('kebab');return 4}},get title(){{trace.push('title');return 'kept'}},get [symbol](){{trace.push('symbol');return 7}}}};const a={usage};"
        );
        let actual = whole::evaluate(
            &source,
            "[a.props.title,a.props[symbol],a.props.className.trim(),Object.hasOwn(a.props,'styleOrder'),Object.hasOwn(a.props,'style-order'),Object.getOwnPropertyDescriptor(config,'styleOrder').get!==undefined]",
        );
        assert_eq!(
            actual.trace,
            serde_json::json!(["order", "kebab", "title", "symbol"])
        );
        assert_eq!(
            actual.element,
            serde_json::json!(["kept", 7, "color-0-red--3-a", false, false, true])
        );
    }
}

#[test]
#[serial]
fn classnames_when_dynamic_left_has_styles_array_fallback_selects_classes() {
    let source = "import {ClassNames} from '@emotion/react';const render=(external,flag)=><ClassNames>{({css})=>css(external||[{color:'blue'}])}</ClassNames>;";
    let compiled = compile_emotion(source).required("array fallback must compile");
    let actual = whole::evaluate_code(
        &compiled.code,
        "[render('',false).trim(),render('user',true).trim()]",
    );
    assert_eq!(
        actual.element,
        serde_json::json!(["color-0-blue--255-a", "user"])
    );
}

#[test]
#[serial]
fn classnames_when_dynamic_left_has_guarded_array_fallback_is_located_rejection() {
    let source = "import {ClassNames} from '@emotion/react';\nconst render=(external,flag)=><ClassNames>{({css})=>css(external||[flag&&{color:'blue'}])}</ClassNames>;";
    let actual = compile_emotion(source)
        .err()
        .required("guarded array fallback must reject");
    assert!(actual.starts_with("a.tsx:2:"), "{actual}");
    assert!(actual.contains("ClassNames"), "{actual}");
}

#[test]
#[serial]
fn classnames_when_dynamic_left_has_empty_array_fallback_preserves_external_class() {
    let source = "import {ClassNames} from '@emotion/react';const render=(external,flag)=><ClassNames>{({css})=>css(external||[])}</ClassNames>;";
    let compiled = compile_emotion(source).required("empty array fallback must compile");
    let actual = whole::evaluate_code(&compiled.code, "[render('',false),render('user',true)]");
    assert_eq!(actual.element, serde_json::json!(["", "user"]));
}
