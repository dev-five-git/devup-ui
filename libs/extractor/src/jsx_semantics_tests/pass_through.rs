use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box color='red' {...rest}/>")]
#[case("<Box {...rest} color='red'/>")]
#[case("jsx(Box,{color:'red',...rest})")]
#[case("jsx(Box,{...rest,color:'red'})")]
#[case("createElement(Box,{color:'red',...rest})")]
#[case("createElement(Box,{...rest,color:'red'})")]
#[serial]
fn authored_styles_when_unknown_keys_collide_keep_raw_props(#[case] element: &str) {
    let control = output(&format!("{BOX}export const a=()=> <Box color='red'/>;"));
    let source = format!(
        "{BOX}{JSX_RUNTIME}import {{createElement}} from 'react';export const a=(rest)=>{element};"
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
        "2",
        "'2'",
        "'.5'",
        "'1.e2'",
        "'0x10'",
        "'4\\n'",
        "({toString(){throw 'coerced';}})",
        "Symbol('raw')",
        "1n",
        "(()=>1)",
    ] {
        let actual = whole::evaluate(
            &source,
            &format!(
                "(()=>{{const raw={input};const e=a({{color:raw}});return [e.type,Object.hasOwn(e.props,'color'),Object.is(e.props.color,raw),Object.keys(e.props.style||{{}}),e.props.className];}})()"
            ),
        );
        assert_eq!(actual.element[0], "div");
        assert_eq!(actual.element[1], true);
        assert_eq!(actual.element[2], true);
        assert_eq!(actual.element[3], serde_json::json!([]));
        assert_eq!(
            actual.element[4]
                .as_str()
                .unwrap_or_else(|| panic!("expected an authored class name"))
                .trim(),
            "color-0-red--255-a",
            "{element}/{input}"
        );
        assert_eq!(static_styles(&output(&source)), static_styles(&control));
        assert_eq!(output(&source).styles.len(), control.styles.len());
    }
}

// The build cannot tell which of the styles an object names a key only the
// running code knows replaces, and the rest would reach the element as props
#[rstest]
#[case("<Box color='red' {...{color:'blue',...rest}}/>")]
#[case("<Box color='red' {...{...rest,color:'blue'}}/>")]
#[case("<Box color='red' {...{color:'blue',[key]:'kept'}}/>")]
#[case("<Box {...rest} {...{_hover:{color:'red'},[key]:'x'}}/>")]
#[case("<Box {...{...rest,p:1}}/>")]
#[case("<Box {...rest} {...{p:1,onClick:rest,[key]:1}}/>")]
#[case("jsx(Box,{color:'red',...{color:'blue',...rest}})")]
#[serial]
fn opaque_objects_when_named_style_keys_are_visible_are_build_errors(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest,key)=>{element};");
    let message = error(&source);
    assert!(message.contains("`<Box>` cannot use `{"), "{message}");
    assert!(message.contains(crate::utils::SPREAD_KEYS), "{message}");
}

#[rstest]
#[case("<Box {...{onClick:rest,[key]:'kept','data-x':1}}/>")]
#[case("jsx(Box,{...{onClick:rest,[key]:'kept'}})")]
#[serial]
fn opaque_objects_when_no_style_key_is_visible_stay_whole(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest,key)=>{element};");
    let actual = whole::evaluate(&source, "a('handler','title')");
    assert_eq!(actual.element["props"]["onClick"], "handler");
    assert_eq!(actual.element["props"]["title"], "kept");
    assert!(output(&source).styles.is_empty());
}

#[rstest]
#[case("<Box color='red' {...{get color(){trace.push('get');return 'blue';}}}/>")]
#[case("jsx(Box,{color:'red',[key]:'blue'})")]
#[serial]
fn opaque_objects_when_named_keys_are_visible_stay_whole(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest,key)=>{element};");
    let actual = whole::evaluate(&source, "a({},'title')");
    if element.contains("[key]:'blue'") {
        assert_eq!(actual.element["props"]["title"], "blue");
    } else {
        assert_eq!(actual.element["props"]["color"], "blue");
    }
    assert_eq!(
        actual.element["props"]["className"]
            .as_str()
            .unwrap_or_else(|| panic!("expected an authored class name"))
            .trim(),
        "color-0-red--255-a"
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "red".into())]
    );
}

#[rstest]
#[case("<Box _hover={{color:'red'}} {...rest}/>")]
#[case("<Box {...rest} _hover={{color:'red'}}/>")]
#[case("jsx(Box,{_hover:{color:'red'},...rest})")]
#[serial]
fn selectors_when_unknown_spreads_follow_compile_independently(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest)=>{element};");
    let actual = whole::evaluate(
        &source,
        "(()=>{const raw={color:'blue'};const e=a({_hover:raw});return [e.props._hover===raw,e.props.className];})()",
    );
    assert_eq!(actual.element[0], true);
    assert_eq!(
        actual.element[1]
            .as_str()
            .unwrap_or_else(|| panic!("expected a selector class name"))
            .trim(),
        "color-0-red-_a__c_hover-255-a"
    );
}

#[rstest]
#[case("jsx", "react/jsx-runtime")]
#[case("jsxs", "react/jsx-runtime")]
#[case("jsxDEV", "react/jsx-dev-runtime")]
#[serial]
fn wrapped_factory_props_when_no_unknown_spread_exists_attach_authored_styles(
    #[case] factory: &str,
    #[case] module: &str,
) {
    for wrapper in [
        "BODY",
        "(BODY)",
        "((BODY))",
        "(BODY as const)",
        "(BODY satisfies object)",
    ] {
        let props = wrapper.replace("BODY", "{color:first()}");
        let source = format!(
            "{BOX}import {{{factory} as jsx}} from '{module}';export const a=(first)=>jsx(Box,{props});"
        );
        let actual = whole::evaluate(
            &source,
            "(()=>{const e=a(()=>{trace.push('first');return 'red';});return [e.type,(e.props.className||'').length>0,Object.values(e.props.style||{})];})()",
        );
        assert_eq!(actual.trace, serde_json::json!(["first"]));
        assert_eq!(
            actual.element,
            serde_json::json!(["div", true, ["red"]]),
            "{factory}/{wrapper}"
        );
    }
}

#[test]
#[serial]
fn distinct_esm_and_commonjs_dynamic_bindings_when_captured_match_runtime_metadata() {
    // Given: the snapshot fixture's two DYNAMIC exports deliberately have different runtime sentinels.
    let source = "import {Box} from '@devup-ui/react';import {DYNAMIC,PRIMARY} from './tokens';import {DYNAMIC as CJS_DYNAMIC} from './cjs-tokens';export const a=state=><Box accentColor={(state.esm,DYNAMIC)} lightingColor={(state.cjs,CJS_DYNAMIC)} {...state.raw}/>;export const shadowed=PRIMARY=><Box color={PRIMARY}/>;";
    let resolver = |specifier: &str, _: &str| match specifier {
        "./tokens" => Some(ResolvedModule {
            path: "/src/tokens.ts".into(),
            code: "export const DYNAMIC=readEsm();export const PRIMARY='global';".into(),
        }),
        "./cjs-tokens" => Some(ResolvedModule {
            path: "/src/cjs-tokens.js".into(),
            code: "exports.DYNAMIC=readCjs();".into(),
        }),
        _ => None,
    };
    reset_class_map();
    reset_file_map();
    let output = extract_with_modules(
        "/src/App.tsx",
        source,
        ExtractOption {
            single_css: true,
            import_main_css: false,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("distinct import extraction: {error}"));
    let mut metadata: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| {
            let ExtractStyleValue::Dynamic(value) = style else {
                return None;
            };
            match style.extract(None) {
                Some(StyleProperty::Variable {
                    class_name,
                    variable_name,
                    ..
                }) => Some((value.property().to_string(), class_name, variable_name)),
                _ => panic!("dynamic declaration must expose its variable"),
            }
        })
        .collect();
    metadata.sort();
    assert_eq!(metadata.len(), 3);
    let metadata =
        serde_json::to_string(&metadata).unwrap_or_else(|error| panic!("metadata: {error}"));
    let compiled = format!(
        "const DYNAMIC='esm-sentinel';const CJS_DYNAMIC='cjs-sentinel';{}",
        output.code
    );
    // When: executable output reads both getters once and uses metadata to select the real variables.
    let result = whole::execute(
        &compiled,
        &format!(
            "(()=>{{const raw={{p:{{sentinel:true}}}};const e=a({{get esm(){{trace.push('esm');return 1;}},get cjs(){{trace.push('cjs');return 2;}},raw}});const local=shadowed('local-sentinel');return [{metadata}.map(([property,name,variable])=>[property,((property==='color'?local:e).props.className||'').split(/\\s+/).includes(name),(property==='color'?local:e).props.style[variable]]),e.props.p===raw.p];}})()"
        ),
    );
    // Then: neither code nor metadata aliases CJS to ESM, and PRIMARY remains the parameter.
    assert_eq!(
        result.element,
        serde_json::json!([
            [
                ["accent-color", true, "esm-sentinel"],
                ["color", true, "local-sentinel"],
                ["lighting-color", true, "cjs-sentinel"]
            ],
            true
        ])
    );
    assert_eq!(result.trace, serde_json::json!(["esm", "cjs"]));
}
