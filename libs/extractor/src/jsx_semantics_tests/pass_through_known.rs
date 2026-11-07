use super::*;
use rstest::rstest;
use serial_test::serial;

fn compile_imported_spread(
    source: &str,
    known: &'static str,
    single_css: bool,
) -> Result<ExtractOutput, String> {
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./known").then(|| ResolvedModule {
            path: "known.ts".into(),
            code: known.into(),
        })
    };
    reset_class_map();
    reset_file_map();
    extract_with_modules(
        "case.tsx",
        source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
        true,
        &resolver,
    )
    .map_err(|error| error.to_string())
}

#[rstest]
#[case("cond ? knownImport : rest")]
#[case("cond ? rest : knownImport")]
#[serial]
fn imported_rule_and_runtime_conditional_arms_refuse_at_the_authored_read(
    #[case] branch: &str,
    #[values(false, true)] single_css: bool,
) {
    // The independent real-WASM agreement fixture must keep the constants owner's rule admission.
    let prefix = "import { Box, type DevupProps } from '@devup-ui/react'; import { styles as knownImport } from './known';\n";
    let body = format!(
        "export function render(cond: boolean, rest: DevupProps) {{ return <Box {{...({branch})}} /> }}"
    );
    let source = format!("{prefix}{body}");
    let error =
        match compile_imported_spread(&source, "export const styles = { bg: 'red' }", single_css) {
            Err(error) => error,
            Ok(_) => panic!("a known rule arm must not turn into raw props"),
        };
    let column = body.rfind("rest").unwrap_or_else(|| panic!("runtime arm")) + 1;
    assert!(
        error.starts_with(&format!("case.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("cannot use `rest`"), "{error}");
    assert_eq!(error.lines().count(), 1, "{error}");
}

#[rstest]
#[case::known_first("cond ? knownImport : rest", "known", "rest")]
#[case::runtime_first("cond ? rest : knownImport", "rest", "known")]
#[case::runtime_only("cond ? rest : other", "rest", "other")]
#[case::authored_literal(concat!("cond ? {", "p:1} : rest"), "literal", "rest")]
#[case::authored_literal_first_runtime(concat!("cond ? rest : {", "p:1}"), "rest", "literal")]
#[serial]
fn conditional_props_without_a_proven_rule_owner_keep_raw_values(
    #[case] branch: &str,
    #[case] consequent: &str,
    #[case] alternate: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "{BOX}import {{styles as knownImport}} from './known';export const render=(cond,rest,other)=><Box {{...({branch})}}/>;"
    );
    let output =
        compile_imported_spread(&source, "export const styles = { id: 'kept' }", single_css)
            .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.styles.is_empty());
    // Select both arms; distinct objects expose accidental eager reads or wrong-arm reuse.
    for (cond, selected) in [(true, consequent), (false, alternate)] {
        let observed = whole::execute(
            &output.code,
            &format!(
                "(()=>{{const first={{}};const second={{}};const firstCallback=()=>0;const secondCallback=()=>1;const make=(label,p,payload,callback)=>({{get p(){{trace.push(label+':p');return p;}},get payload(){{trace.push(label+':payload');return payload;}},get onClick(){{trace.push(label+':onClick');return callback;}}}});const e=render({cond},make('rest',7,first,firstCallback),make('other',9,second,secondCallback));return [e.props.p??null,e.props.id??null,e.props.payload===first,e.props.payload===second,e.props.onClick===firstCallback,e.props.onClick===secondCallback];}})()"
            ),
        );
        let expected = match selected {
            "rest" => serde_json::json!([7, null, true, false, true, false]),
            "other" => serde_json::json!([9, null, false, true, false, true]),
            "known" => serde_json::json!([null, "kept", false, false, false, false]),
            "literal" => serde_json::json!([1, null, false, false, false, false]),
            _ => panic!("unknown selected arm"),
        };
        assert_eq!(observed.element, expected, "{branch}/{cond}");
        let trace = if matches!(selected, "rest" | "other") {
            serde_json::json!([
                format!("{selected}:p"),
                format!("{selected}:payload"),
                format!("{selected}:onClick")
            ])
        } else {
            serde_json::json!([])
        };
        assert_eq!(observed.trace, trace, "{branch}/{cond}");
    }
}

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

#[test]
#[serial]
fn local_objects_when_ordinary_callbacks_capture_them_remain_raw() {
    let source = format!(
        "{BOX}export const a=()=>{{const first={{p:'20px'}};return <Box p='4px' {{...first}} onClick={{()=>{{first.p='mutated';}}}} data-object={{first}}/>;}};"
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("padding".into(), "4px".into())]
    );
    assert_eq!(
        whole::evaluate(
            &source,
            "(()=>{const e=a();const before=e.props.p;e.props.onClick();return [before,e.props.p,e.props['data-object'].p];})()"
        )
        .element,
        serde_json::json!(["20px", "20px", "mutated"])
    );
}

#[rstest]
#[case::conditional(
    concat!(
    "export const a=(on,rest)=><Box ",
    "{",
    "...(on?",
    "{",
    "p:1}:rest)}/>;",
),
    "a(false,raw)",
    3
)]
#[case::nested("export const a=(rest)=><Box {...{...rest,p:1}}/>;", "a(raw)", 1)]
#[case::shadow_undefined(
    concat!(
    "export const a=(on,undefined)=><Box ",
    "{",
    "...(on?",
    "{",
    "p:1}:undefined)}/>;",
),
    "a(false,raw)",
    3
)]
#[case::shadow_nan(
    concat!(
    "export const a=(on,NaN)=><Box ",
    "{",
    "...(on?",
    "{",
    "p:1}:NaN)}/>;",
),
    "a(false,raw)",
    3
)]
#[case::shadow_infinity(
    concat!(
    "export const a=(on,Infinity)=><Box ",
    "{",
    "...(on?",
    "{",
    "p:1}:Infinity)}/>;",
),
    "a(false,raw)",
    3
)]
#[serial]
fn opaque_local_props_preserve_original_copy_and_cutoff(
    #[case] body: &str,
    #[case] call: &str,
    #[case] padding: i32,
    #[values(false, true)] single_css: bool,
) {
    let source = format!("{BOX}{body}");
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    assert!(output.styles.is_empty(), "{output:?}");
    if padding == 3 {
        let active = whole::execute(
            &output.code,
            "(()=>{const raw={get p(){trace.push('unexpected');throw new Error('opaque arm ran');}};return a(true,raw).props.p;})()",
        );
        assert_eq!(active.element, 1);
        assert_eq!(active.trace, serde_json::json!([]));
    }
    for throwing in [false, true] {
        let observed = whole::execute(
            &output.code,
            &format!(
                r"(()=>{{
const callback=()=>0;const token={{}};const style={{opacity:0.4}};
const raw={{get p(){{trace.push('p');if({throwing})throw new Error('stop');return 3;}},get onClick(){{trace.push('callback');return callback;}},get className(){{trace.push('class');return 'raw';}},get style(){{trace.push('style');return style;}},get payload(){{trace.push('payload');return token;}}}};
try{{const e={call};return ['ok',e.props.p,e.props.onClick===callback,e.props.className==='raw',e.props.style===style,e.props.payload===token];}}catch(error){{return [error.message];}}
}})()"
            ),
        );
        if throwing {
            assert_eq!(observed.element, serde_json::json!(["stop"]));
            assert_eq!(observed.trace, serde_json::json!(["p"]));
        } else {
            assert_eq!(
                observed.element,
                serde_json::json!(["ok", padding, true, true, true, true])
            );
            assert_eq!(
                observed.trace,
                serde_json::json!(["p", "callback", "class", "style", "payload"])
            );
        }
    }
}

#[rstest]
#[case::member(
    concat!(
    "export const a=(on,rest)=><Box ",
    "{",
    "...(on?",
    "{",
    "p:1}:rest.rules)}/>;",
),
    "rest.rules"
)]
#[case::call(concat!(
    "export const a=(on,rest)=><Box ",
    "{",
    "...(on?",
    "{",
    "p:1}:read())}/>;",
), "read()")]
#[case::module(
    concat!(
    "export const a=(on)=><Box ",
    "{",
    "...(on?",
    "{",
    "p:1}:unknownRules)}/>;",
),
    "unknownRules"
)]
#[case::selector("export const a=(rest)=><Box _hover={rest}/>;", "rest")]
#[serial]
fn opaque_props_dispatch_preserves_explicit_style_refusals(
    #[case] body: &str,
    #[case] leaf: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!("{BOX}import {{css}} from '@devup-ui/react';{body}");
    let error = match compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    ) {
        Err(error) => error,
        Ok(_) => panic!("style refusal must remain"),
    };
    assert!(error.starts_with("a.tsx:"), "{error}");
    assert!(error.contains(&format!("cannot use `{leaf}`")), "{error}");
}

#[rstest]
#[case::empty("{}")]
#[case::inert("undefined")]
#[serial]
fn known_conditional_rules_keep_style_admission(
    #[case] alternate: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!("{BOX}export const a=(on)=><Box {{...(on?{{p:1}}:{alternate})}}/>;");
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(
        static_styles(&output),
        vec![("padding".into(), "4px".into())]
    );
}

#[rstest]
#[case::known(
    "const rules=css({color:'red'});return <Box {...(on?{p:1}:rules)}/>;",
    false
)]
#[case::shadow(
    "const rules=css({color:'red'});return ((rules)=><Box {...(on?{p:1}:rules)}/>)(raw);",
    true
)]
#[serial]
fn local_known_class_and_shadowed_props_keep_their_own_bindings(
    #[case] body: &str,
    #[case] shadowed: bool,
    #[values(false, true)] single_css: bool,
) {
    let source =
        format!("{BOX}import {{css}} from '@devup-ui/react';export const a=(on,raw)=>{{{body}}};");
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let observed = whole::execute(
        &output.code,
        "(()=>{const token={};const raw={payload:token,className:'external'};const e=a(false,raw);return [e.props.payload===token,e.props.className];})()",
    );
    if shadowed {
        assert_eq!(static_styles(&output), vec![("color".into(), "red".into())]);
        assert_eq!(observed.element, serde_json::json!([true, "external"]));
    } else {
        assert_eq!(
            static_styles(&output),
            vec![
                ("color".into(), "red".into()),
                ("padding".into(), "4px".into())
            ]
        );
        assert_eq!(observed.element[0], false);
        assert_eq!(observed.element[1], "");
    }
}

#[rstest]
#[case::function("export const a=(rest)=>css(rest);", "a(token)")]
#[case::native_prop("export const a=(rest)=><div css={rest}/>;", "a(token).props.css")]
#[serial]
fn opaque_css_controls_keep_passthrough_identity_without_reading_the_value(
    #[case] body: &str,
    #[case] call: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!("{BOX}import {{css}} from '@devup-ui/react';{body}");
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    assert!(output.styles.is_empty(), "{output:?}");
    let observed = whole::execute(
        &output.code,
        &format!(
            r"(()=>{{
const callback=()=>0;const style={{opacity:0.4}};
const token={{get onClick(){{trace.push('callback');return callback;}},get style(){{trace.push('style');return style;}}}};
const actual={call};const before=trace.slice();
return [actual===token,before,actual.onClick===callback,actual.style===style];
}})()"
        ),
    );
    assert_eq!(observed.element, serde_json::json!([true, [], true, true]));
    assert_eq!(observed.trace, serde_json::json!(["callback", "style"]));
}

#[derive(Default)]
struct StaticCaptureWork {
    calls: usize,
    wrappers: usize,
    arrays: usize,
    sequences: usize,
    class_reads: Vec<String>,
}

impl<'a> oxc_ast_visit::Visit<'a> for StaticCaptureWork {
    fn visit_call_expression(&mut self, value: &oxc_ast::ast::CallExpression<'a>) {
        self.calls += 1;
        self.wrappers += usize::from(matches!(
            value.callee.get_inner_expression(),
            oxc_ast::ast::Expression::ArrowFunctionExpression(_)
                | oxc_ast::ast::Expression::FunctionExpression(_)
        ));
        oxc_ast_visit::walk::walk_call_expression(self, value);
    }
    fn visit_array_expression(&mut self, value: &oxc_ast::ast::ArrayExpression<'a>) {
        self.arrays += 1;
        oxc_ast_visit::walk::walk_array_expression(self, value);
    }
    fn visit_sequence_expression(&mut self, value: &oxc_ast::ast::SequenceExpression<'a>) {
        self.sequences += 1;
        oxc_ast_visit::walk::walk_sequence_expression(self, value);
    }
    fn visit_identifier_reference(&mut self, value: &oxc_ast::ast::IdentifierReference<'a>) {
        if matches!(value.name.as_str(), "base" | "extra") {
            self.class_reads.push(value.name.to_string());
        }
    }
}

fn static_capture_work(source: &str) -> StaticCaptureWork {
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
    assert!(
        !parsed.fatal_error && parsed.diagnostics.is_empty(),
        "{source}\n{:?}",
        parsed.diagnostics
    );
    let mut work = StaticCaptureWork::default();
    oxc_ast_visit::Visit::visit_program(&mut work, &parsed.program);
    work
}

#[rstest]
#[case::original_focused(
    "export const App = () => <div css={css`${base}; color: red;`} />;",
    "App()",
    false
)]
#[case::original_mixed(
    "export const App = ({ cls }) => <><div css={[base, { color: 'red' }]} /><div css={css`${base}; color: red;`} /><div css={css({ color: 'red' }, cond && { color: 'blue' })} /><div css={[cls, { color: 'red' }]} /></>;",
    "(()=>{globalThis.cond=false;return App({cls:'external'}).children[1];})()",
    false
)]
#[case::nested_child(
    "export const App=()=> <section><div css={css`${base}; color: red;`}/></section>;",
    "App().children[0]",
    false
)]
#[case::nested_attribute(
    "export const App=()=> <section child={<div css={css`${base}; color: red;`}/>}/>;",
    "App().props.child",
    false
)]
#[case::fragment(
    "export const App=()=> <><aside/><div css={css`${base}; color: red;`}/></>;",
    "App().children[1]",
    false
)]
#[case::text(
    "export const App=()=> <div css={css`${base}; color: red;`}>ready</div>;",
    "App()",
    false
)]
#[case::multiple_reads(
    concat!(
    "const extra=css(",
    "{",
    "padding:0});export const App=()=> <div css=",
    "{",
    "css`$",
    "{",
    "base}; $",
    "{",
    "extra}; color: red;`}/>;",
),
    "App()",
    true
)]
#[serial]
fn static_css_captures_keep_reads_without_unused_calls(
    #[case] body: &str,
    #[case] probe: &str,
    #[case] multiple: bool,
    #[values(false, true)] single_css: bool,
) {
    struct RestoreDebug(bool);
    impl Drop for RestoreDebug {
        fn drop(&mut self) {
            css::debug::set_debug(self.0);
        }
    }
    let source = format!(
        "import {{ css }} from '@emotion/react';\nconst base = css({{ color: 'blue', margin: 0 }});\n{body}"
    );
    let options = ExtractOption {
        single_css,
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    let bucket = crate::css_bucket("a.tsx", &options);
    let output = compile_with(&source, options).unwrap_or_else(|error| panic!("{error:?}"));
    let work = static_capture_work(&output.code);
    assert_eq!(
        (work.calls, work.wrappers, work.arrays),
        (0, 0, 0),
        "{}",
        output.code
    );
    assert!(
        work.sequences > 0,
        "original reads must remain: {}",
        output.code
    );
    assert_eq!(
        work.class_reads,
        if multiple {
            vec!["base", "extra"]
        } else {
            vec!["base"]
        }
    );
    let observed = whole::execute(&output.code, probe);
    assert_eq!(observed.element["type"], "div");
    let names: Vec<_> = observed.element["props"]["className"]
        .as_str()
        .unwrap_or_else(|| panic!("expected rendered class name"))
        .split_whitespace()
        .collect();
    let restore_debug = RestoreDebug(css::debug::is_debug());
    css::debug::set_debug(true);
    let red_is_active = output.styles.iter().any(|style| {
        matches!(style, ExtractStyleValue::Static(value) if value.property()=="color" && value.value()=="red")
            && matches!(style.extract(bucket.as_deref()), Some(StyleProperty::ClassName(name)) if names.contains(&name.as_str()))
    });
    drop(restore_debug);
    assert!(red_is_active, "{output:?}\n{:?}", observed.element);
    assert_eq!(observed.trace, serde_json::json!([]));
    let map: serde_json::Value = serde_json::from_str(
        output
            .map
            .as_deref()
            .unwrap_or_else(|| panic!("expected extraction source map")),
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(map["sourcesContent"][0], source);
}

#[rstest]
#[case::component(
    "export const App=(View)=> <View css={css`${base}; color: red;`}/>;",
    "(()=>{const View=()=>0;return App(View).type===View;})()",
    false,
    true
)]
#[case::capture_like_component(
    "export const App=(__devupValue0)=> <__devupValue0 css={css`${base}; color: red;`}/>;",
    "(()=>{const View=()=>0;return App(View).type===View;})()",
    false,
    true
)]
#[case::expression_attribute(
    "export const App=(token)=> <div data-token={token} css={css`${base}; color: red;`}/>;",
    "(()=>{const token={};return App(token).props['data-token']===token;})()",
    false,
    true
)]
#[case::partial_value(
    "export const App=(gap)=> <div css={css`${base}; padding: ${gap};`}/>;",
    "typeof App('12px').props.style==='object'",
    false,
    true
)]
#[case::expression_child(
    "export const App=(child)=> <div css={css`${base}; color: red;`}>{child}</div>;",
    "(()=>{const child={};return App(child).children[0]===child;})()",
    false,
    true
)]
#[case::nested_element_child(
    "export const App=()=> <div css={css`${base}; color: red;`}><span/></div>;",
    "App().children[0].type==='span'",
    false,
    true
)]
#[case::spread_attribute(
    "export const App=(props)=> <div {...props} css={css`${base}; color: red;`}/>;",
    "(()=>{const token={};return App({token}).props.token===token;})()",
    false,
    true
)]
#[serial]
fn static_css_capture_elision_keeps_used_values_and_component_names(
    #[case] body: &str,
    #[case] probe: &str,
    #[case] captured: bool,
    #[case] sequenced: bool,
    #[values(false, true)] single_css: bool,
) {
    let source =
        format!("import {{css}} from '@emotion/react';const base=css({{color:'blue'}});{body}");
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let work = static_capture_work(&output.code);
    assert_eq!(work.wrappers > 0, captured, "{}", output.code);
    assert_eq!(work.sequences > 0, sequenced, "{}", output.code);
    let observed = whole::execute(&output.code, probe);
    assert_eq!(observed.element, true);
    assert_eq!(observed.trace, serde_json::json!([]));
}

#[rstest]
#[case::first_uninitialized("", "const base=css({color:'blue'});", "base")]
#[case::second_uninitialized("const base=css({color:'blue'});", "", "extra")]
#[serial]
fn static_css_capture_reads_keep_tdz_and_first_error_order(
    #[case] before: &str,
    #[case] after: &str,
    #[case] first_error: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{css}} from '@emotion/react';{before}export const early=(()=>{{try{{App();return ['ok'];}}catch(error){{return [error.name,error.message];}}}})();{after}const extra=css({{margin:0}});export function App(){{return <div css={{css`${{base}}; ${{extra}}; color: red;`}}/>;}}"
    );
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let observed = whole::execute(&output.code, "early");
    assert_eq!(observed.element[0], "ReferenceError");
    assert!(
        observed.element[1]
            .as_str()
            .unwrap_or_else(|| panic!("expected first ReferenceError message"))
            .contains(first_error),
        "{:?}",
        observed.element
    );
    let work = static_capture_work(&output.code);
    // The one authored immediate arrow and App() call remain.
    assert_eq!(
        (work.calls, work.wrappers, work.arrays),
        (2, 1, 2),
        "{}",
        output.code
    );
    assert!(work.sequences > 0, "{}", output.code);
    assert_eq!(work.class_reads, vec!["base", "extra"]);
}

#[rstest]
#[serial]
fn static_css_capture_fallback_preserves_trailing_spread_refusal(
    #[values(false, true)] single_css: bool,
) {
    let source = "import {css} from '@emotion/react';const base=css({color:'blue'});export const App=(props)=> <div css={css`${base}; color: red;`} {...props}/>;";
    let error = match compile_with(
        source,
        ExtractOption {
            single_css,
            import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    ) {
        Err(error) => error,
        Ok(_) => panic!("trailing spread must remain refused"),
    };
    assert_eq!(
        error,
        "a.tsx:1:131: `css` on `<div>` cannot use `...props` at build time: a spread after the `css` prop may carry a `css` of its own, which replaces it whole where only the runtime could tell: write the spread before `css`"
    );
}

#[rstest]
#[case::and_right(
    "cond ? (gate && knownImport) : rest",
    "gate && knownImport",
    r#"gate && { "bg": "red" }"#
)]
#[case::and_right_alternate(
    "cond ? rest : (gate && knownImport)",
    "gate && knownImport",
    r#"gate && { "bg": "red" }"#
)]
#[case::or_left(
    "cond ? (knownImport || null) : rest",
    "knownImport || null",
    r#"({ "bg": "red" }) || null"#
)]
#[case::coalesce_left(
    "cond ? (knownImport ?? null) : rest",
    "knownImport ?? null",
    r#"({ "bg": "red" }) ?? null"#
)]
#[case::or_right(
    "cond ? (false || knownImport) : rest",
    "false || knownImport",
    r#"false || { "bg": "red" }"#
)]
#[case::coalesce_right(
    "cond ? (null ?? knownImport) : rest",
    "null ?? knownImport",
    r#"null ?? { "bg": "red" }"#
)]
#[serial]
fn nested_logical_rule_owners_keep_the_authored_runtime_refusal(
    #[case] branch: &str,
    #[case] authored_logical: &str,
    #[case] rendered_logical: &str,
    #[values(false, true)] single_css: bool,
) {
    let prefix =
        "import {Box} from '@devup-ui/react';import {styles as knownImport} from './known';\n";
    let body = format!("export const render=(cond,gate,rest)=><Box {{...({branch})}}/>;");
    let source = format!("{prefix}{body}");
    let error =
        match compile_imported_spread(&source, "export const styles = { bg: 'red' }", single_css) {
            Err(error) => error,
            Ok(_) => panic!("a nested rule owner must retain runtime rule refusal"),
        };
    let logical_column = body
        .find(authored_logical)
        .unwrap_or_else(|| panic!("authored logical arm"))
        + 1;
    let rest_column = body.rfind("rest").unwrap_or_else(|| panic!("runtime arm")) + 1;
    let mut expected = [(logical_column, rendered_logical), (rest_column, "rest")];
    expected.sort_by_key(|(column, _)| *column);
    let expected = expected.map(|(column, code)| {
        format!("case.tsx:2:{column}: `<Box>` cannot use `{code}` at build time: its styles must be an object literal or a constant object, or be computed from constants")
    });
    assert_eq!(error, expected.join("\n"));
    assert_eq!(error.lines().count(), 2);
    assert_eq!(error.matches("cannot use `rest`").count(), 1);
}

#[rstest]
#[case::and_left("cond ? (knownImport && rest) : other", "rest", "other", false)]
#[case::and_left_alternate("cond ? rest : (knownImport && other)", "rest", "other", false)]
#[case::literal_condition(
    concat!("cond ? ({", "p:1} && rest) : other"),
    "rest",
    "other",
    false
)]
#[case::local_shadow("cond ? (knownImport && rest) : other", "rest", "other", true)]
#[serial]
fn nested_and_conditions_without_rule_results_keep_raw_props(
    #[case] branch: &str,
    #[case] consequent: &str,
    #[case] alternate: &str,
    #[case] shadowed: bool,
    #[values(false, true)] single_css: bool,
) {
    let parameters = if shadowed {
        "cond,rest,other,knownImport"
    } else {
        "cond,rest,other"
    };
    let source = format!(
        "{BOX}import {{styles as knownImport}} from './known';export const render=({parameters})=><Box {{...({branch})}}/>;"
    );
    let output =
        compile_imported_spread(&source, "export const styles = { bg: 'red' }", single_css)
            .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.styles.is_empty(), "{output:?}");
    for (cond, selected) in [(true, consequent), (false, alternate)] {
        let observed = whole::execute(
            &output.code,
            &format!(
                "(()=>{{const first={{}};const second={{}};const callback=()=>0;const make=(name,p,payload)=>({{get p(){{trace.push(name+':p');return p;}},get payload(){{trace.push(name+':payload');return payload;}},get onClick(){{trace.push(name+':onClick');return callback;}}}});const e=render({cond},make('rest',7,first),make('other',9,second),{{get bg(){{throw Error('condition copied');}}}});return [e.props.p,e.props.payload===first,e.props.payload===second,e.props.onClick===callback];}})()"
            ),
        );
        assert_eq!(
            observed.element,
            if selected == "rest" {
                serde_json::json!([7, true, false, true])
            } else {
                serde_json::json!([9, false, true, true])
            },
            "{branch}/{cond}"
        );
        assert_eq!(
            observed.trace,
            serde_json::json!([
                format!("{selected}:p"),
                format!("{selected}:payload"),
                format!("{selected}:onClick")
            ]),
            "{branch}/{cond}"
        );
    }
}

#[rstest]
#[case::or_fallback("readGate() || knownImport", "null", "known")]
#[case::or_value("readGate() || knownImport", "rest", "rest")]
#[case::coalesce_fallback("readGate() ?? knownImport", "null", "known")]
#[case::coalesce_value("readGate() ?? knownImport", "rest", "rest")]
#[case::coalesce_falsy("readGate() ?? knownImport", "0", "none")]
#[case::and_falsy("readGate() && knownImport", "0", "none")]
#[case::and_value("readGate() && knownImport", "rest", "known")]
#[serial]
fn nested_logical_data_arms_keep_short_circuit_reads_and_identity(
    #[case] logical: &str,
    #[case] gate: &str,
    #[case] selected: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "{BOX}import {{styles as knownImport}} from './known';export const render=(cond,readGate,other)=><Box {{...(cond ? ({logical}) : other)}}/>;"
    );
    let output =
        compile_imported_spread(&source, "export const styles = { id: 'kept' }", single_css)
            .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.styles.is_empty(), "{output:?}");
    for cond in [true, false] {
        let observed = whole::execute(
            &output.code,
            &format!(
                "(()=>{{const token={{}};const callback=()=>0;const make=name=>({{get p(){{trace.push(name+':p');return 7;}},get payload(){{trace.push(name+':payload');return token;}},get onClick(){{trace.push(name+':onClick');return callback;}}}});const rest=make('rest');const other=make('other');const readGate=()=>{{trace.push('gate');return {gate};}};const e=render({cond},readGate,other);return [e.props.p??null,e.props.id??null,e.props.payload===token,e.props.onClick===callback];}})()"
            ),
        );
        let chosen = if cond { selected } else { "other" };
        assert_eq!(
            observed.element,
            match chosen {
                "rest" | "other" => serde_json::json!([7, null, true, true]),
                "known" => serde_json::json!([null, "kept", false, false]),
                "none" => serde_json::json!([null, null, false, false]),
                _ => panic!("unknown selected arm"),
            },
            "{logical}/{gate}/{cond}"
        );
        let mut expected = Vec::new();
        if cond {
            expected.push("gate".to_string());
        }
        if matches!(chosen, "rest" | "other") {
            for key in ["p", "payload", "onClick"] {
                expected.push(format!("{chosen}:{key}"));
            }
        }
        assert_eq!(
            observed.trace,
            serde_json::json!(expected),
            "{logical}/{gate}/{cond}"
        );
    }
}
