use super::*;

mod spreads;

fn emotion(source: &str) -> Result<ExtractOutput, String> {
    compile_with(
        source,
        ExtractOption {
            single_css: true,
            import_aliases: HashMap::from([
                ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
                (
                    "@emotion/styled".to_string(),
                    ImportAlias::DefaultToNamed("styled".to_string()),
                ),
            ]),
            ..ExtractOption::default()
        },
    )
}

#[test]
#[serial]
fn capture_contract_when_typography_choice_is_saved_keeps_lookup_and_empty_classes() {
    let source = "import {Text} from '@devup-ui/react'; function render(bo,a,b){return <Text typography={bo?a:b}/>;} const result=[render(true,'heading','body'),render(false,'heading','body'),render(true,'','body'),render(false,'heading','')];";
    let compiled = output(source);
    let actual = whole::evaluate_code(&compiled.code, "result.map(a=>a.props.className||'')");
    assert_eq!(
        actual.element,
        serde_json::json!(["typo-heading", "typo-body", "", ""]),
        "{}",
        compiled.code
    );
    assert!(!compiled.styles.iter().any(|style| matches!(style, ExtractStyleValue::Dynamic(style) if style.property() == "typography")));
}

#[test]
#[serial]
fn capture_contract_when_typography_choice_has_getters_keeps_selected_source_once() {
    for active in [true, false] {
        let source = format!(
            "import {{Text}} from '@devup-ui/react'; const config={{get active(){{trace.push('choice');return {active}}},get a(){{trace.push('a');return 'heading'}},get b(){{trace.push('b');return ''}}}}; const a=<Text typography={{config.active?config.a:config.b}}/>;"
        );
        let actual = whole::evaluate(&source, "a.props.className||''");
        assert_eq!(
            actual.trace,
            if active {
                serde_json::json!(["choice", "a"])
            } else {
                serde_json::json!(["choice", "b"])
            }
        );
        assert_eq!(actual.element, if active { "typo-heading" } else { "" });
    }
}

#[test]
#[serial]
fn capture_contract_when_typography_with_order_is_read_has_one_source_reference() {
    let source = "import {Text} from '@devup-ui/react'; function render(isActive,typo){return <Text styleOrder={isActive?5:10} typography={typo}/>;}";
    let compiled = output(source);
    assert_eq!(
        super::order_reads::reads(&compiled.code, "typo"),
        1,
        "{}",
        compiled.code
    );
    assert_eq!(
        super::order_reads::reads(&compiled.code, "isActive"),
        1,
        "{}",
        compiled.code
    );
}

#[test]
#[serial]
fn capture_contract_when_typography_getter_has_order_keeps_source_order_and_empty_result() {
    for value in ["heading", ""] {
        for active in [true, false] {
            let source = format!(
                "import {{Text}} from '@devup-ui/react'; const config={{get active(){{trace.push('order');return {active}}},get typo(){{trace.push('typo');return '{value}'}}}}; const a=<Text id={{(trace.push('id'),'a')}} styleOrder={{config.active?5:10}} typography={{config.typo}} title={{(trace.push('title'),'b')}}/>;"
            );
            let compiled = output(&source);
            let actual = whole::evaluate_code(&compiled.code, "a.props.className||''");
            assert_eq!(
                actual.trace,
                serde_json::json!(["id", "order", "typo", "title"])
            );
            assert_eq!(
                actual.element,
                if value.is_empty() { "" } else { "typo-heading" }
            );
            assert!(!compiled.styles.iter().any(|style| matches!(style, ExtractStyleValue::Dynamic(style) if style.property() == "typography")));
        }
    }
}

#[test]
#[serial]
fn capture_contract_when_emotion_takes_dynamic_text_preserves_color_atom_and_variable() {
    let source = "export const App=({c})=><div css={`color: ${c};`}/>;";
    let compiled = emotion(source).required("dynamic Emotion CSS text must compile");
    assert!(
        compiled.styles.iter().any(
            |style| matches!(style,ExtractStyleValue::Dynamic(style) if style.property()=="color")
        ),
        "{}",
        compiled.code
    );
    let actual = whole::evaluate_code(&compiled.code, "App({c:'purple'}).props");
    assert!(
        !actual.element["className"]
            .as_str()
            .required("CSS text must emit classes")
            .contains("color:")
    );
    assert!(
        actual.element["style"]
            .as_object()
            .required("dynamic color must emit a style object")
            .values()
            .any(|value| value == "purple")
    );
}

#[test]
#[serial]
fn capture_contract_when_dynamic_text_is_nested_keeps_lazy_value_effects() {
    let source = "const read=()=>(trace.push('color'),'purple'); const App=(on)=><div id={(trace.push('id'),'a')} css={on?[`color: ${read()};`]:{color:'red'}} title={(trace.push('title'),'b')}/>; const a=App(true);const b=App(false);";
    let compiled = emotion(source).required("nested dynamic text must compile");
    let actual = whole::evaluate_code(&compiled.code, "[a.props,b.props]");
    assert_eq!(
        actual.trace,
        serde_json::json!(["id", "color", "title", "id", "title"])
    );
    assert!(
        actual.element[0]["style"]
            .as_object()
            .required("selected text must emit variables")
            .values()
            .any(|value| value == "purple")
    );
}

#[test]
#[serial]
fn capture_contract_when_unknown_local_rule_members_are_used_retains_original_errors() {
    for (source, column, code) in [
        (
            "export const App = () => { const s = { color: 'red' }; return <div css={[s.a]} />; };",
            74,
            "s.a",
        ),
        (
            "export const App = () => { const s = { color: 'red' }; return <div css={s[0]} />; };",
            73,
            "s[0]",
        ),
    ] {
        let message = emotion(source)
            .required_err("unknown local rule members must not become proven class strings");
        assert_eq!(
            message,
            format!(
                "a.tsx:1:{column}: `css` on `<div>` cannot use `{code}` at build time: a style object it composes must be written in it, or declared with `const` at the top level of the module, where the build reads it"
            )
        );
    }
}

#[test]
#[serial]
fn capture_contract_when_untagged_text_has_a_mixin_retains_original_located_error() {
    let source = "export const App = ({ base }) => <div css={`${base}; color: red;`} />;";
    let message =
        emotion(source).required_err("unknown declaration-position mixin must stay invalid");
    assert_eq!(
        message,
        "a.tsx:1:47: `css` on `<div>` cannot use `base` at build time: an interpolation in CSS text must be a value, or a mixin standing where a declaration would"
    );
}
