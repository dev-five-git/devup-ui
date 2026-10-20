use crate::{ExtractOption, ExtractOutput, ImportAlias, ResolvedModule, extract_with_modules};
use boa_engine::{Context, Source};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

#[path = "class_names_nested_binding_tests.rs"]
mod nested;

fn compile(code: &str) -> ExtractOutput {
    reset_class_map();
    reset_file_map();
    extract_with_modules(
        "binding.tsx",
        code,
        ExtractOption {
            single_css: true,
            import_main_css: false,
            import_aliases: HashMap::from([(
                "@emotion/react".to_string(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
        false,
        &|specifier, _| {
            (specifier == "./data").then(|| ResolvedModule {
                path: "data.ts".to_string(),
                code: "export const data = { padding: 7 }; export const space = 3;".to_string(),
            })
        },
    )
    .unwrap_or_else(|error| panic!("{code}\n{error}"))
}

fn execute(output: &ExtractOutput) -> String {
    let arena = Allocator::default();
    let mut parsed = Parser::new(&arena, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    parsed
        .program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&parsed.program).code;
    let script = format!("const data={{padding:7}};{code}\nJSON.stringify(App());");
    Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{script}\n{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("expected JSON: {script}"))
        .to_std_string_escaped()
}

fn styles(output: &ExtractOutput) -> Vec<(&str, &str)> {
    let mut styles: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::ExtractStyleValue::Static(style) => {
                (style.property.as_str(), style.value.as_str())
            }
            other => panic!("unexpected style: {other:?}"),
        })
        .collect();
    styles.sort_unstable();
    styles
}

#[rstest]
#[case("((css)=>css('color:red;'))(()=>{trace.push('shadow-call');return 'kept';})")]
#[case(
    "(()=>{const css=()=>{trace.push('shadow-call');return 'kept';};return css('color:red;');})()"
)]
#[case(
    "(()=>{let css=()=>{trace.push('shadow-call');return 'kept';};return css('color:red;');})()"
)]
#[case(
    "(()=>{var css=()=>{trace.push('shadow-call');return 'kept';};return css('color:red;');})()"
)]
#[case(
    "(()=>{function css(){trace.push('shadow-call');return 'kept';}return css('color:red;');})()"
)]
#[case(
    "(()=>{try{throw ()=>{trace.push('shadow-call');return 'kept';};}catch(css){return css('color:red;');}})()"
)]
#[serial]
fn class_names_binding_shadow_calls_keep_effects(#[case] title: &str) {
    // Given: an ordinary shadow beside an actual outer API read in a nested function.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';const trace=[];const App=()=> <ClassNames>{{({{css}})=> ({{title:{title},trace,className:(()=>css({{color:'blue'}}))()}})}}</ClassNames>;"
    );
    // When: compile and execute the generated render function.
    let output = compile(&source);
    let value = execute(&output);
    // Then: only the real API emits CSS; ordinary application effects survive.
    assert_eq!(styles(&output), [("color", "blue")]);
    assert_eq!(
        value,
        r#"{"title":"kept","trace":["shadow-call"],"className":"a"}"#
    );
}

#[test]
#[serial]
fn class_names_binding_numeric_shadow_keeps_values() {
    // Given: aliases and independent css/cx calls taking numbers and arrays.
    let output = compile(
        "import {ClassNames as Names} from '@emotion/react';const App=()=> <Names>{({css:style,cx:join})=> ({local:((style,join)=>[style({padding:[1,2]}),join({kept:2})])(x=>x,x=>x),className:join(style({padding:[3,4]}),'caller')})}</Names>;",
    );
    // When: execute the actual output, not a string-token approximation of the pass.
    let value = execute(&output);
    // Then: only genuine styles receive px/fallback conversion and class-map lowering.
    assert_eq!(styles(&output), [("padding", "3px;padding:4px")]);
    assert_eq!(
        value,
        r#"{"local":[{"padding":[1,2]},{"kept":2}],"className":"caller a"}"#
    );
}

#[test]
#[serial]
fn class_names_binding_theme_shadow_keeps_value() {
    // Given: the outer theme is read from nested code; an independent local theme is ordinary.
    let output = compile(
        "import {ClassNames} from '@emotion/react';const App=()=> <ClassNames>{({css,theme})=> ({local:((theme)=>theme.spacing(2))({spacing:x=>x+5}),className:(()=>css({color:theme.colors.text}))()})}</ClassNames>;",
    );
    // When: compile and execute the application value.
    let value = execute(&output);
    // Then: real theme reads lower to variables without rejecting the ordinary local call.
    assert_eq!(styles(&output), [("color", "var(--colors-text)")]);
    assert_eq!(value, r#"{"local":7,"className":"a"}"#);
}

#[test]
#[serial]
fn class_names_binding_shadow_import_stays_ordinary() {
    // Given: imported data is used only by an ordinary shadow, while a real API reads space.
    let output = compile(
        "import {ClassNames} from '@emotion/react';import {data,space} from './data';const App=()=> <ClassNames>{({css})=> ({local:((css)=>css(data))(x=>x),same:((css)=>css(data))(x=>x)===data,className:css({padding:space})})}</ClassNames>;",
    );
    // When: execute with the runtime import's original object.
    let value = execute(&output);
    // Then: inlining/pixelification does not copy or change ordinary imported data.
    assert_eq!(styles(&output), [("padding", "3px")]);
    assert_eq!(
        value,
        r#"{"local":{"padding":7},"same":true,"className":"a"}"#
    );
}

#[test]
#[serial]
fn class_names_binding_shadow_does_not_fall_back_to_imported_api() {
    // Given: the file also imports css, whose spelling cannot identify the nested local.
    let output = compile(
        "import {ClassNames,css} from '@emotion/react';const App=()=> <ClassNames>{({css})=> ({local:((css)=>css({padding:[1,2]}))(x=>x),className:css({color:'blue'})})}</ClassNames>;",
    );
    // When: execute the ordinary call beside the genuine callback API.
    let value = execute(&output);
    // Then: neither the number pass, constant pass nor visitor reclassifies it as the import.
    assert_eq!(styles(&output), [("color", "blue")]);
    assert_eq!(value, r#"{"local":{"padding":[1,2]},"className":"a"}"#);
}

#[test]
#[serial]
fn class_names_binding_shadow_only_import_is_not_loaded() {
    // Given: an unresolved runtime import is not a style dependency.
    let output = compile(
        "import {ClassNames} from '@emotion/react';import {data} from './runtime';const App=()=> <ClassNames>{({css})=> ((css)=>css(data))(x=>x)}</ClassNames>;",
    );
    // When: execute the emitted ordinary callback.
    let value = execute(&output);
    // Then: no false style read, dependency, or unsupported diagnostic is introduced.
    assert_eq!(output.dependencies, Vec::<String>::new());
    assert_eq!(value, r#"{"padding":7}"#);
    assert_eq!(output.styles.len(), 0);
}

#[test]
#[serial]
fn class_names_binding_component_shadow_is_not_compiled() {
    // Given: the component parameter shares only the imported API's spelling.
    let output = compile(
        "import {ClassNames} from '@emotion/react';const App=ClassNames=><ClassNames marker={7}>{({css})=>css({padding:[1,2]})}</ClassNames>;",
    );
    // When: inspect the surviving ordinary component's public props and child values.
    // Then: neither compilation nor preprocessing treats it as the imported component.
    assert!(
        output.code.contains("<ClassNames marker={7}>"),
        "{}",
        output.code
    );
    assert!(output.code.contains("padding: [1, 2]"), "{}", output.code);
    assert_eq!(output.styles.len(), 0);
}

#[rstest]
#[case(
    "const App=()=> <ClassNames>{({css})=>((css)=>css(data))(mutate)}</ClassNames>;",
    true
)]
#[case(
    "const App=ClassNames=> <ClassNames ref={data}>{({css})=>css({color:'blue'})}</ClassNames>;",
    true
)]
#[case("const App=()=> <ClassNames>{({css})=>css(data)}</ClassNames>;", false)]
#[case(
    "const App=()=> <ClassNames>{({css:style})=>(()=>style(data))()}</ClassNames>;",
    false
)]
#[case(
    "const App=()=> <ClassNames>{({css})=>css`${data}`}</ClassNames>;",
    false
)]
fn class_names_binding_mutation_shadow_is_runtime_escape(
    #[case] body: &str,
    #[case] escapes: bool,
) {
    // Given: the mutation classifier owns this program's semantic analysis.
    let arena = Allocator::default();
    let source =
        format!("import {{ClassNames}} from '@emotion/react';const data={{padding:1}};{body}");
    let parsed = Parser::new(&arena, &source, SourceType::tsx()).parse();
    let takers = crate::css_prop::CssTakers::new(
        &parsed.program,
        crate::css_prop::CssProp::Off,
        "@devup-ui/react/compat",
    );
    // When: classify whether the ordinary shadow may change data.
    let uses = crate::mutations::uses(&parsed.program, &|name| name == "ClassNames", Some(&takers));
    // Then: data escapes to runtime code; it must not be treated as compile-only use.
    assert_eq!(
        uses.get("data").is_some_and(|uses| uses
            .iter()
            .any(|usage| matches!(usage, crate::mutations::Use::Escapes { .. }))),
        escapes,
        "{body}\n{uses:?}"
    );
}
