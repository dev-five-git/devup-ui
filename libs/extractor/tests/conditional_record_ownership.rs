mod retained;
use retained::{BASE, RunOptions, run_with_options};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::local("const record={color:'red',id:'local'};", "record", "local")]
#[case::module("", "moduleRecord", "module")]
#[case::imported("", "knownImport", "imported")]
#[serial]
fn safe_conditional_record_preserves_css_and_selected_native_props(
    #[case] setup: &str,
    #[case] expression: &str,
    #[case] expected_id: &str,
    #[values(false, true)] single_css: bool,
) {
    let app = format!(
        "import {{Box}} from '@devup-ui/react';import {{styles as knownImport}} from './known';const moduleRecord={{color:'red',id:'module'}};function render(cond){{{setup}return <Box {{...(cond ? {expression} : {{}})}}/>;}}export {{render}};"
    );
    let known = "const styles={color:'red',id:'imported'};export {styles};";
    let files = [("/src/known.ts", known), ("/src/app.tsx", app.as_str())];
    let observation = "const a=modules['/src/app.tsx'];JSON.stringify([false,true].map(cond=>{const e=render(a.render(cond));return [e.target,e.props.id??null,values(e,'color')]}));";
    let result = if single_css {
        retained::run(&files, observation)
    } else {
        run_with_options(
            &files,
            observation,
            RunOptions {
                readable: true,
                single_css,
            },
        )
    };
    assert_eq!(
        result,
        serde_json::json!([["div", null, []], ["div", expected_id, ["red"]]])
    );
}

const SHADOW_SOURCE: &str = "import {Box} from '@devup-ui/react';const trace=[];function renderOuter(cond){const record={color:'red',id:'outer'};trace.push('outer');return <Box {...(cond ? record : {})}/>;}function renderInner(cond){const record={color:'blue',id:'inner'};trace.push('inner');record.color='green';return <Box {...(cond ? record : {})}/>;}export {trace,renderOuter,renderInner};";

#[rstest]
#[serial]
fn separate_functions_keep_their_own_record_binding(#[values(false, true)] single_css: bool) {
    let result = run_with_options(
        &[("/src/app.tsx", SHADOW_SOURCE)],
        "const a=modules['/src/app.tsx'];const before=a.trace.slice();const outer=[false,true].map(cond=>{const e=render(a.renderOuter(cond));return [e.target,e.props.id??null,values(e,'color')];});const inner=[false,true].map(cond=>{const e=render(a.renderInner(cond));return [e.target,e.props.id??null,e.props.color??null,values(e,'color')];});JSON.stringify([before,a.trace,outer,inner]);",
        RunOptions {
            readable: true,
            single_css,
        },
    );
    assert_eq!(
        result,
        serde_json::json!([
            [],
            ["outer", "outer", "inner", "inner"],
            [["div", null, []], ["div", "outer", ["red"]]],
            [["div", null, null, []], ["div", "inner", "green", []]]
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn structured_whole_leaves_keep_lexical_closure_and_authored_order(
    #[case] template: bool,
    #[values(false, true)] single_css: bool,
) {
    let callback = "(p,unused=(trace.push('default'),0))=>({...((trace.push('guard'),p.on)?{color:'blue'}:{}),margin:function(){'use strict';trace.push(this===undefined?'direct':'bound');return p.input.value;},padding:(trace.push('after'),'1px')})";
    let factory = if template {
        format!("styled(Base)`${{{callback}}}`")
    } else {
        format!("styled(Base)({callback})")
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{Base}} from './base';const trace=[];const Card={factory};export {{Card,trace}};"
    );
    let value = run_with_options(
        &[("/src/base.ts", BASE), ("/src/app.ts", &app)],
        "const a=modules['/src/app.ts'];const before=a.trace.slice();const input={get value(){a.trace.push('value');return '2px';}};const first=render(jsx(a.Card,{on:true,input}));const second=render(jsx(a.Card,{on:false,input}));JSON.stringify([before,a.trace,[first,second].map(e=>[values(e,'color'),values(e,'margin'),values(e,'padding')])]);",
        RunOptions {
            readable: true,
            single_css,
        },
    );
    assert_eq!(
        value,
        serde_json::json!([
            [],
            [
                "default", "guard", "after", "direct", "value", "default", "guard", "after",
                "direct", "value"
            ],
            [[["blue"], ["2px"], ["1px"]], [["green"], ["2px"], ["1px"]]]
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn structured_whole_mapped_theme_leaves_never_read_runtime_theme(
    #[case] template: bool,
    #[values(false, true)] single_css: bool,
) {
    let callback = "(p,unused=(trace.push('default'),0))=>({...((trace.push('guard'),p.on)?{color:p.theme.colors.brand}:{}),margin:(trace.push('after'),'2px')})";
    let factory = if template {
        format!("styled(Base)`${{{callback}}}`")
    } else {
        format!("styled(Base)({callback})")
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{Base}} from './base';const trace=[];const Card={factory};export {{Card,trace}};"
    );
    let value = run_with_options(
        &[("/src/base.ts", BASE), ("/src/app.ts", &app)],
        concat!(
            "const a=modules['/src/app.ts'];const before=a.trace.slice();const first=render(jsx(a.Card,",
            "{",
            "on:true",
            "}",
            "));const second=render(jsx(a.Card,",
            "{",
            "on:false",
            "}",
            "));JSON.stringify([before,a.trace,[first,second].map(e=>[e.target,values(e,'margin')]),Object.values(rules).some(r=>r[0]==='color'&&r[1]==='var(--colors-brand)')]);",
        ),
        RunOptions {
            readable: true,
            single_css,
        },
    );
    assert_eq!(
        value,
        serde_json::json!([
            [],
            ["default", "guard", "after", "default", "guard", "after"],
            [["article", ["2px"]], ["article", ["2px"]]],
            true
        ])
    );
}

#[rstest]
#[case::direct("record.color='blue';")]
#[case::alias("const alias=record;alias.color='blue';")]
#[serial]
fn mutated_records_keep_getters_native_props_and_abrupt_order(
    #[case] mutation: &str,
    #[values(false, true)] logical: bool,
    #[values(false, true)] single_css: bool,
) {
    let expression = if logical {
        "cond && record"
    } else {
        "cond ? record : {}"
    };
    let app = format!(
        "import {{Box}} from '@devup-ui/react';const trace=[];const payload={{}};function handler(){{}}function renderCase(cond,phase){{trace.length=0;const record={{color:'red',get className(){{trace.push('class');if(phase==='class')throw Error('class');return 'raw';}},get style(){{trace.push('style');if(phase==='style')throw Error('style');return payload;}},get 'data-payload'(){{trace.push('payload');return payload;}},ref:handler}};trace.push('mutation');{mutation}return <Box data-before={{(trace.push('before'),'before')}} {{...({expression})}} data-after={{(trace.push('after'),'after')}}>{{(trace.push('child'),'child')}}</Box>;}}export {{renderCase,trace,payload,handler}};"
    );
    let result = run_with_options(
        &[("/src/app.tsx", &app)],
        "const a=modules['/src/app.tsx'];JSON.stringify([false,true].flatMap(cond=>['ok','class','style'].map(phase=>{try{const e=render(a.renderCase(cond,phase));return [cond,phase,a.trace.slice(),e.target,e.props.color??null,values(e,'color'),e.props['data-before'],e.props['data-after'],e.props.children,cond?[e.props['data-payload']===a.payload,e.props.ref===a.handler,e.props.style===a.payload,e.props.className==='raw']:[]];}catch(error){return [cond,phase,a.trace.slice(),error.message];}})));",
        RunOptions {
            readable: true,
            single_css,
        },
    );
    assert_eq!(
        result,
        serde_json::json!([
            [
                false,
                "ok",
                ["mutation", "before", "after", "child"],
                "div",
                null,
                [],
                "before",
                "after",
                "child",
                []
            ],
            [
                false,
                "class",
                ["mutation", "before", "after", "child"],
                "div",
                null,
                [],
                "before",
                "after",
                "child",
                []
            ],
            [
                false,
                "style",
                ["mutation", "before", "after", "child"],
                "div",
                null,
                [],
                "before",
                "after",
                "child",
                []
            ],
            [
                true,
                "ok",
                [
                    "mutation", "before", "class", "style", "payload", "after", "child"
                ],
                "div",
                "blue",
                [],
                "before",
                "after",
                "child",
                [true, true, true, true]
            ],
            [true, "class", ["mutation", "before", "class"], "class"],
            [
                true,
                "style",
                ["mutation", "before", "class", "style"],
                "style"
            ]
        ])
    );
}

fn namespace_error(source: &str, single_css: bool) -> String {
    use extractor::{ExtractOption, ImportAlias, ResolvedModule, extract_with_modules};
    use std::collections::HashMap;

    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./base").then(|| ResolvedModule {
            path: "/src/base.ts".into(),
            code:
                "import styled from '@emotion/styled'; export const Child=styled.span`color:red;`;"
                    .into(),
        })
    };
    let options = ExtractOption {
        single_css,
        import_aliases: HashMap::from([
            ("@emotion/react".into(), ImportAlias::NamedToNamed),
            (
                "@emotion/styled".into(),
                ImportAlias::DefaultToNamed("styled".into()),
            ),
        ]),
        ..ExtractOption::default()
    };
    match extract_with_modules("/src/app.tsx", source, options, false, &resolver) {
        Err(error) => error.to_string(),
        Ok(_) => panic!("fixture must report the original independent diagnostics"),
    }
}

#[rstest]
#[case::owner_0(r"import * as UI from './base';import {jsx} from '@emotion/react/jsx-runtime';export const P=jsx('div',{css:{'@layer':{'':{color:'red'}}},...props});", r"/src/app.tsx:1:118: `css()` cannot use `@layer ` at build time: name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`
/src/app.tsx:1:137: `css` on `<div>` cannot use `...props` at build time: a spread after the `css` prop may carry a `css` of its own, which replaces it whole where only the runtime could tell: write the spread before `css`")]
#[case::owner_1(r"import * as UI from './base';import {jsx} from '@emotion/react/jsx-runtime';export const P=jsx('div',{css:{'@layer':{'':{color:'red'}}},...props});", r"/src/app.tsx:1:118: `css()` cannot use `@layer ` at build time: name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`
/src/app.tsx:1:137: `css` on `<div>` cannot use `...props` at build time: a spread after the `css` prop may carry a `css` of its own, which replaces it whole where only the runtime could tell: write the spread before `css`")]
#[case::owner_2(r"import * as UI from './base';import {jsx} from '@emotion/react/jsx-runtime';export const P=jsx(Custom,{css:{'@layer':{'':{color:'red'}}},...props});", r"/src/app.tsx:1:119: `css()` cannot use `@layer ` at build time: name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`
/src/app.tsx:1:138: `css` on `<Custom>` cannot use `...props` at build time: a spread after the `css` prop may carry a `css` of its own, which replaces it whole where only the runtime could tell: write the spread before `css`")]
#[case::owner_3(r"import * as UI from './base';import {jsx} from '@emotion/react/jsx-runtime';export const P=jsx(Custom,{css:{'@layer':{'':{color:'red'}}},...props});", r"/src/app.tsx:1:119: `css()` cannot use `@layer ` at build time: name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`
/src/app.tsx:1:138: `css` on `<Custom>` cannot use `...props` at build time: a spread after the `css` prop may carry a `css` of its own, which replaces it whole where only the runtime could tell: write the spread before `css`")]
#[case::owner_4(r"import * as UI from './base';import {jsx} from '@emotion/react/jsx-runtime';export const P=[jsx(UI.Child,{css:{color:'blue'},...props}),jsx('div',{css:{'@layer':{'':{color:'red'}}},...otherProps})];", r"/src/app.tsx:1:107: `css` on `<UI.Child>` overrides styles `UI.Child` sets, which the build orders only for a styled component rendering a tag with no attrs or props read, given no spread, `as` or `forwardedAs`: move these styles into `styled(UI.Child)(...)`
/src/app.tsx:1:163: `css()` cannot use `@layer ` at build time: name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`
/src/app.tsx:1:182: `css` on `<div>` cannot use `...otherProps` at build time: a spread after the `css` prop may carry a `css` of its own, which replaces it whole where only the runtime could tell: write the spread before `css`")]
#[case::owner_5(r"import * as UI from './base';import {jsx} from '@emotion/react/jsx-runtime';export const P=[jsx(UI['Child'],{css:{color:'blue'},...props}),jsx('div',{css:{'@layer':{'':{color:'red'}}},...otherProps})];", r#"/src/app.tsx:1:110: `css` on `<UI["Child"]>` overrides styles `UI["Child"]` sets, which the build orders only for a styled component rendering a tag with no attrs or props read, given no spread, `as` or `forwardedAs`: move these styles into `styled(UI["Child"])(...)`
/src/app.tsx:1:166: `css()` cannot use `@layer ` at build time: name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`
/src/app.tsx:1:185: `css` on `<div>` cannot use `...otherProps` at build time: a spread after the `css` prop may carry a `css` of its own, which replaces it whole where only the runtime could tell: write the spread before `css`"#)]
#[case::owner_6(r"import * as UI from './base';import {jsx} from '@emotion/react/jsx-runtime';export const P=jsx(UI.Child,{css:[{color:'blue'},{'@layer':{'':{padding:'4px'}}}],...props});", r"/src/app.tsx:1:106: `css` on `<UI.Child>` overrides styles `UI.Child` sets, which the build orders only for a styled component rendering a tag with no attrs or props read, given no spread, `as` or `forwardedAs`: move these styles into `styled(UI.Child)(...)`
/src/app.tsx:1:137: `css()` cannot use `@layer ` at build time: name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`")]
#[case::owner_7(r"import * as UI from './base';import {jsx} from '@emotion/react/jsx-runtime';export const P=jsx(UI['Child'],{css:[{color:'blue'},{'@layer':{'':{padding:'4px'}}}],...props});", r#"/src/app.tsx:1:109: `css` on `<UI["Child"]>` overrides styles `UI["Child"]` sets, which the build orders only for a styled component rendering a tag with no attrs or props read, given no spread, `as` or `forwardedAs`: move these styles into `styled(UI["Child"])(...)`
/src/app.tsx:1:140: `css()` cannot use `@layer ` at build time: name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`"#)]
#[serial]
fn rejected_owner_keeps_independent_layer_errors_in_original_order(
    #[case] source: &str,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    assert_eq!(namespace_error(source, single_css), expected);
}
