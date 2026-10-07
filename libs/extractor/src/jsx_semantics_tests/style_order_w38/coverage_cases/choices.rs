use super::*;

#[test]
#[serial]
fn guarded_ordered_parts_when_nested_choices_are_lazy_keep_the_fallback() {
    for (outer, inner, expected, trace) in [
        (
            false,
            true,
            "color-0-red--2-a",
            serde_json::json!(["outer"]),
        ),
        (
            true,
            true,
            "color-0-blue--2-a",
            serde_json::json!(["outer", "inner"]),
        ),
        (
            true,
            false,
            "color-0-green--2-a",
            serde_json::json!(["outer", "inner"]),
        ),
    ] {
        let source = format!(
            "import {{css}} from '@devup-ui/react';const flag=(name,value)=>(trace.push(name),value);const a=css({{styleOrder:2,color:'red'}},flag('outer',{outer})&&(flag('inner',{inner})?{{styleOrder:2,color:'blue'}}:{{styleOrder:2,color:'green'}}));"
        );
        let actual = whole::evaluate(&source, "a");
        assert_eq!(actual.trace, trace);
        assert_eq!(actual.element, expected);
    }
}

#[test]
#[serial]
fn classes_when_logical_fallback_is_empty_keep_selected_external_class() {
    let source = "import {css} from '@devup-ui/react';const make=external=>css(external||void 0);";
    let actual = whole::evaluate(source, "[make('user'),make('')]");
    assert_eq!(actual.element, serde_json::json!(["user", ""]));
}

#[test]
#[serial]
fn classes_when_saved_known_styles_are_used_in_a_lazy_fallback_keep_the_winner() {
    let source = "import {css} from '@devup-ui/react';const saved=css({styleOrder:2,color:'blue'});const make=external=>css({styleOrder:2,color:'red'},external||saved);";
    let actual = whole::evaluate(source, "make('').trim()");
    assert_eq!(actual.element, "color-0-blue--2-a");
}

#[test]
#[serial]
fn classnames_when_array_fallback_is_empty_or_known_preserves_classes() {
    let source = "import {ClassNames} from '@emotion/react';const render=external=><ClassNames>{({css,cx})=>[css([]||{color:'red'}),css([{color:'blue'}]||{color:'red'}),cx(['user']||'other')]}</ClassNames>;";
    let compiled = compile_emotion(source).required("ClassNames must compile");
    let actual = whole::evaluate_code(&compiled.code, "render('')");
    assert_eq!(
        actual.element,
        serde_json::json!(["", "color-0-blue--255-a", "user"])
    );
}

#[test]
#[serial]
fn classnames_when_array_fallback_has_nested_choice_or_mixed_parts_is_rejected() {
    for array in ["[flag?external:'user']", "['user',{color:'blue'},external]"] {
        let source = format!(
            "import {{ClassNames}} from '@emotion/react';\nconst render=(external,flag)=><ClassNames>{{({{css}})=>css({array}||{{color:'black'}})}}</ClassNames>;"
        );
        let actual = compile_emotion(&source)
            .err()
            .required("mixed or guarded fallback must be rejected");
        assert!(actual.starts_with("a.tsx:2:"), "{actual}");
        assert!(actual.contains("cannot"), "{actual}");
    }
}

#[test]
#[serial]
fn imported_unknown_when_nested_in_array_or_conditional_is_located_at_the_consumer() {
    let missing = |_: &str, _: &str| None;
    for argument in [
        "[missing]",
        "flag?missing:{color:'blue'}",
        "flag?{color:'blue'}:missing",
    ] {
        let source = format!(
            "import {{css}} from '@devup-ui/react';import {{missing}} from './missing';\nconst a=css({argument});"
        );
        let actual = extract_with_modules(
            "/consumer.tsx",
            &source,
            ExtractOption::default(),
            false,
            &missing,
        )
        .err()
        .required("unresolved imported rules must be rejected")
        .to_string();
        assert!(actual.starts_with("/consumer.tsx:2:"), "{actual}");
        assert!(actual.contains("missing"), "{actual}");
    }
}

#[test]
#[serial]
fn guarded_external_class_when_condition_is_false_keeps_only_the_base() {
    let source = "import {css} from '@devup-ui/react';const make=(flag,external)=>css({color:'red'},flag&&external);";
    let actual = whole::evaluate(
        source,
        "[make(false,'user').trim(),make(true,'user').trim()]",
    );
    assert_eq!(
        actual.element,
        serde_json::json!(["color-0-red--255-a", "color-0-red--255-a user"])
    );
}
