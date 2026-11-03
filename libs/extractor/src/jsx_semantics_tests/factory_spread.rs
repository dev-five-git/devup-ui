use super::*;

const PREFIX: &str = "import { jsx as _jsx } from '@emotion/react/jsx-runtime';";

fn extract_spread(source: &str, single_css: bool) -> ExtractOutput {
    reset_class_map();
    reset_file_map();
    extract(
        "factory.tsx",
        source,
        ExtractOption {
            single_css,
            import_aliases: HashMap::from([
                ("@emotion/react".into(), ImportAlias::NamedToNamed),
                (
                    "@emotion/react/jsx-runtime".into(),
                    ImportAlias::NamedToNamed,
                ),
            ]),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}: {source}"))
}

// Only factory argument construction is observed. The mocks never render a component.
const PRELUDE: &str = "const trace=[],poison={};let reads=0;const Custom='Custom',Box='div',k='id';function automatic(type,props,key){return {type,props,key};}const React={get createElement(){trace.push('callee');return automatic;}};";

// factory_script already binds React automatic-runtime imports. Bind the other
// actual imports here; no executable body or component implementation is rewritten.
fn spread_script(source: &str) -> String {
    let arena = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx()).parse();
    assert!(
        !parsed.fatal_error && parsed.diagnostics.is_empty(),
        "{source}"
    );
    let mut bindings = String::new();
    for statement in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = statement {
            for specifier in import.specifiers.iter().flatten() {
                let binding = match (import.source.value.as_str(), specifier) {
                    (
                        Some("@emotion/react/jsx-runtime"),
                        ImportDeclarationSpecifier::ImportSpecifier(named),
                    ) => {
                        assert!(matches!(
                            named.imported.to_string().as_str(),
                            "jsx" | "jsxs"
                        ));
                        Some((named.local.name.as_str(), "automatic"))
                    }
                    (Some("react"), ImportDeclarationSpecifier::ImportSpecifier(named))
                        if matches!(named.imported.to_string().as_str(), "forwardRef") =>
                    {
                        Some((named.local.name.as_str(), "()=>Custom"))
                    }
                    (
                        Some("@emotion/styled"),
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(default),
                    ) => Some((default.local.name.as_str(), "{button:()=>Custom}")),
                    _ => None,
                };
                if let Some((name, value)) = binding {
                    bindings.push_str("const ");
                    bindings.push_str(name);
                    bindings.push('=');
                    bindings.push_str(value);
                    bindings.push_str(";\n");
                }
            }
        }
    }
    bindings + &factory_script(source, "automatic")
}

fn observe_spread(script: &str, setup: &str, probe: &str) -> serde_json::Value {
    let text = run(&format!("{PRELUDE}{setup}{script}\n{probe}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{error}: {text}"))
}

#[test]
#[serial_test::serial]
fn factory_css_spread_getter_is_read_once() {
    let _debug = RestoreDebug(css::debug::is_debug());
    css::debug::set_debug(false);
    let source = format!(
        "{PREFIX}function App(props){{return _jsx(Custom,{{...props,css:{{color:'red'}}}});}}"
    );
    let setup = "const props={get className(){trace.push('class:'+ ++reads);if(reads>1)throw poison;return 'first';}};";
    let probe = "let result;try{const node=App(props);result={reads,trace,first:node.props.className.split(' ')[0]};}catch(error){result={reads,trace,poison:error===poison};}return JSON.stringify(result);";
    let authored = observe_spread(&spread_script(&source), setup, probe);
    assert_eq!(
        authored,
        serde_json::json!({"reads":1,"trace":["class:1"],"first":"first"})
    );
    for single_css in [false, true] {
        let output = extract_spread(&source, single_css);
        assert_eq!(static_styles(&output), vec![("color".into(), "red".into())]);
        let actual = observe_spread(&spread_script(&output.code), setup, probe);
        assert_eq!(
            actual, authored,
            "getter must read the copied slot, mode={single_css}: {}",
            output.code
        );
    }
}

#[test]
#[serial_test::serial]
fn factory_css_spread_keeps_proxy_order_errors_and_authored_reads() {
    let _debug = RestoreDebug(css::debug::is_debug());
    css::debug::set_debug(false);
    let setup = "const props=new Proxy({className:'first',id:'id'},{ownKeys(t){trace.push('keys');return Reflect.ownKeys(t);},getOwnPropertyDescriptor(t,k){trace.push('descriptor:'+String(k));return Reflect.getOwnPropertyDescriptor(t,k);},get(t,k,r){trace.push('get:'+String(k));if(k==='className')reads++;return Reflect.get(t,k,r);}});";
    let cases = [
        ("proxy", "{...props,css:{color:'red'}}", setup),
        (
            "class-before",
            "{className:'before',...props,css:{color:'red'}}",
            setup,
        ),
        (
            "class-after",
            "{...props,className:'after',css:{color:'red'}}",
            setup,
        ),
        (
            "multiple",
            "{...props,...other,css:{color:'red'}}",
            "const props={get className(){trace.push('first');reads++;return 'first';}};const other={get className(){trace.push('last');reads++;return 'last';}};",
        ),
        (
            "computed",
            "{...props,[key()]:value(),css:{color:'red'}}",
            "const props={get className(){trace.push('spread');reads++;return 'first';}};function key(){trace.push('key');return 'className';}function value(){trace.push('value');return 'computed';}",
        ),
        (
            "authored-read",
            "{...props,title:props.className,css:{color:'red'}}",
            "const props={get className(){trace.push('class:'+ ++reads);return reads===1?'first':'second';}};",
        ),
        (
            "getter-throw",
            "{id:(trace.push('id'),'id'),...props,css:{color:'red'},title:(trace.push('title'),'title')}",
            "const props={get className(){trace.push('throw');reads++;throw poison;}};",
        ),
        ("null", "{...props,css:{color:'red'}}", "const props=null;"),
    ];
    // React.createElement on an unknown component intentionally keeps its css prop.
    let negative = format!(
        "{PREFIX}import * as React from 'react';function App(props){{return React.createElement(Custom,{{...props,css:{{color:'red'}}}});}}"
    );
    let negative_probe = "const node=App(props);return JSON.stringify({trace,reads,css:node.props.css,className:node.props.className});";
    let negative_authored = observe_spread(&spread_script(&negative), setup, negative_probe);
    for single_css in [false, true] {
        let output = extract_spread(&negative, single_css);
        assert!(
            output.styles.is_empty(),
            "unknown React component is not a css owner"
        );
        assert_eq!(
            observe_spread(&spread_script(&output.code), setup, negative_probe),
            negative_authored
        );
    }
    for (id, props, setup) in cases {
        let source = format!(
            "{PREFIX}function App(props){{return _jsx((trace.push('type'),Custom),{props},(trace.push('key'),'key'));}}"
        );
        let probe = "let result;try{const node=App(props);result={trace,reads,className:node.props.className||'',id:node.props.id||null,title:node.props.title||null,key:node.key};}catch(error){result={trace,reads,poison:error===poison};}return JSON.stringify(result);";
        let authored = observe_spread(&spread_script(&source), setup, probe);
        if id == "getter-throw" {
            assert_eq!(
                authored,
                serde_json::json!({"trace":["type","id","throw"],"reads":1,"poison":true})
            );
        }
        for single_css in [false, true] {
            let output = extract_spread(&source, single_css);
            assert_eq!(static_styles(&output), vec![("color".into(), "red".into())]);
            let mut actual = observe_spread(&spread_script(&output.code), setup, probe);
            if let Some(class_name) = actual.get_mut("className") {
                let text = class_name
                    .as_str()
                    .unwrap_or_else(|| panic!("className must be text"));
                let suffix = output
                    .styles
                    .iter()
                    .find_map(|style| {
                        match style.extract(
                            css_bucket(
                                "factory.tsx",
                                &ExtractOption {
                                    single_css,
                                    ..ExtractOption::default()
                                },
                            )
                            .as_deref(),
                        ) {
                            Some(StyleProperty::ClassName(name)) => Some(name),
                            _ => None,
                        }
                    })
                    .unwrap_or_else(|| panic!("missing emitted red class"));
                assert!(
                    text == suffix || text.ends_with(&format!(" {suffix}")),
                    "{id}: {text}"
                );
                *class_name = serde_json::Value::String(
                    text.strip_suffix(&format!(" {suffix}"))
                        .unwrap_or("")
                        .to_string(),
                );
            }
            assert_eq!(actual, authored, "{id}/{single_css}: {}", output.code);
        }
    }
}

#[test]
#[serial_test::serial]
fn saved_eight_factory_calls_keep_spread_value_and_direct_identity() {
    let _debug = RestoreDebug(css::debug::is_debug());
    css::debug::set_debug(false);
    let source = SAVED_FIXTURE;
    let setup = "const props={get className(){trace.push('class:'+ ++reads);return reads===1?'external-first':'external-second';}};";
    let probe = "const nodes=App(props);return JSON.stringify({calls:nodes.length,reads,trace,first:nodes[3].props.className.split(' ')[0],identity:nodes[6].props===props});";
    let authored = observe_spread(&spread_script(source), setup, probe);
    assert_eq!(
        authored,
        serde_json::json!({"calls":8,"reads":1,"trace":["class:1"],"first":"external-first","identity":true})
    );
    for single_css in [false, true] {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "test.ts",
            source,
            ExtractOption {
                package: "@devup-ui/react".into(),
                css_dir: "@devup-ui/react".into(),
                single_css,
                import_main_css: false,
                import_aliases: HashMap::from([
                    ("@emotion/react".into(), ImportAlias::NamedToNamed),
                    (
                        "@emotion/styled".into(),
                        ImportAlias::DefaultToNamed("styled".into()),
                    ),
                ]),
            },
        )
        .unwrap_or_else(|error| panic!("saved original context: {error}"));
        let actual = observe_spread(&spread_script(&output.code), setup, probe);
        assert_eq!(
            actual, authored,
            "saved factory row/{single_css}: {}",
            output.code
        );
        println!(
            "LANE756_FACTORY_SAVED {}",
            serde_json::json!({"singleCss":single_css,"source":source,"output":output.code,"authored":authored,"actual":actual,"mockComponentsNotExecuted":true})
        );
    }
}

#[test]
#[serial_test::serial]
fn factory_css_unknown_trailing_spread_keeps_original_error_location() {
    let source = format!(
        "{PREFIX}\nfunction App(props){{return _jsx(Custom,{{css:{{color:'red'}},...props}});}}"
    );
    let offset = source
        .find("...props")
        .unwrap_or_else(|| panic!("authored spread"));
    let column = source[..offset]
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .len()
        + 1;
    for single_css in [false, true] {
        let actual = compile_with(
            &source,
            ExtractOption {
                single_css,
                import_aliases: HashMap::from([
                    ("@emotion/react".into(), ImportAlias::NamedToNamed),
                    (
                        "@emotion/react/jsx-runtime".into(),
                        ImportAlias::NamedToNamed,
                    ),
                ]),
                ..ExtractOption::default()
            },
        )
        .err()
        .unwrap_or_else(|| panic!("unknown css owner must fail"));
        assert_eq!(
            actual,
            format!(
                "a.tsx:2:{column}: `css` on `<Custom>` cannot use `...props` at build time: a spread after the `css` prop may carry a `css` of its own, which replaces it whole where only the runtime could tell: write the spread before `css`"
            )
        );
    }
}

const SAVED_FIXTURE: &str = "import styled from '@emotion/styled';\nimport { Box } from '@devup-ui/react';\nimport { jsx as _jsx } from '@emotion/react/jsx-runtime';\nconst Button = styled.button({ color: 'blue' });\nexport const App = (props) => [_jsx(Custom, { className: 'x', css: { color: 'red' } }), _jsx(Box, { p: 1, css: { padding: 4 } }), _jsx(Button, { css: { color: 'red' } }), _jsx(Button, { ...props, css: { margin: 1 } }), _jsx(Button, { as: 'a', css: { margin: 2 } }), _jsx(Button, { [k]: 1, css: { padding: 3 } }), _jsx('div', props), _jsx('div', { ['css']: { color: 'red' }, id: 'x' })];";
