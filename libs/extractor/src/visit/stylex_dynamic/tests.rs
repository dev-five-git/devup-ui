use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

use crate::{ExtractOption, ExtractOutput, ExtractStyleValue};

mod assignments;
mod callers;
mod edges;
mod helpers;
mod includes;
mod reader;
mod regressions;
mod remediation;

fn extract(source: &str) -> Result<ExtractOutput, Box<dyn std::error::Error>> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    crate::extract(
        "/src/stylex.tsx",
        source,
        ExtractOption {
            import_main_css: false,
            single_css: true,
            ..ExtractOption::default()
        },
    )
}

fn execute(output: &ExtractOutput, setup: &str, result: &str) -> String {
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, &output.code, SourceType::tsx())
        .parse()
        .program;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = format!(
        "{setup}\n{}\nJSON.stringify({result})",
        Codegen::new().build(&program).code
    );
    Context::default()
        .eval(Source::from_bytes(&code))
        .unwrap_or_else(|error| panic!("{error}: {code}"))
        .as_string()
        .expect("JSON string")
        .to_std_string_escaped()
}

#[rstest]
#[case("void 0", "[\"red\"]")]
#[case("null", "[null]")]
#[case("'blue'", "[\"blue\"]")]
#[case("false", "[false]")]
#[serial]
fn stylex_dynamic_defaults_when_runtime_input_is_undefined_only(
    #[case] input: &str,
    #[case] expected: &str,
) {
    let source = "import stylex from '@stylexjs/stylex'; const styles = stylex.create({tone:(color='red')=>({color})}); const result=stylex.props(styles.tone(input));";
    let output = extract(source).expect("dynamic extraction");
    let actual = execute(
        &output,
        &format!("const input={input};"),
        "Object.values(result.style)",
    );
    assert_eq!(actual, expected);
}

#[rstest]
#[case("", Some("red"))]
#[case("undefined", Some("red"))]
#[case("void 0", Some("red"))]
#[case("null", None)]
#[case("'blue'", Some("blue"))]
#[serial]
fn stylex_dynamic_calls_when_all_arguments_are_known_emit_static_classes(
    #[case] input: &str,
    #[case] expected: Option<&str>,
) {
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const styles = stylex.create({{tone:(color='red')=>({{color}})}}); const result=stylex.props(styles['tone']({input}));"
    );
    let output = extract(&source).expect("static extraction");
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.value.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(values, expected.into_iter().collect::<Vec<_>>());
    assert_eq!(execute(&output, "", "'style' in result"), "false");
    if let Some(expected) = expected {
        let class = execute(&output, "", "result.className");
        let css: Vec<_> = output
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Static(style) => Some(format!(
                    "\"{}\"",
                    css::sheet_to_classname(
                        &style.property,
                        style.level,
                        Some(&style.value),
                        None,
                        style.style_order,
                        None
                    )
                )),
                _ => None,
            })
            .collect();
        assert!(css.contains(&class), "{expected}: {class}, {css:?}");
    }
}

#[test]
#[serial]
fn stylex_dynamic_capture_when_input_is_reused_preserves_single_evaluation_and_extra_arguments() {
    let source = "import stylex from '@stylexjs/stylex'; const styles=stylex.create({size:(height=8,unused)=>({height,width:height})}); const result=stylex.props(styles.size(next(),unused(),extra()));";
    let output = extract(source).expect("captured extraction");
    let actual = execute(
        &output,
        "const trace=[]; function next(){trace.push('next');return undefined} function unused(){trace.push('unused');return 2} function extra(){trace.push('extra');return 3}",
        "[trace,Object.values(result.style)]",
    );
    assert_eq!(
        actual,
        "[[\"next\",\"unused\",\"extra\"],[\"8px\",\"8px\"]]"
    );
}

#[test]
#[serial]
fn stylex_dynamic_defaults_when_undefined_is_shadowed_keep_the_runtime_binding() {
    let source = "import stylex from '@stylexjs/stylex'; const styles=stylex.create({tone:(color='red')=>({color})}); function render(undefined){return stylex.props(styles.tone(undefined))} const result=render(null);";
    let output = extract(source).expect("shadowed binding extraction");
    let actual = execute(&output, "", "Object.values(result.style)");
    assert_eq!(actual, "[null]");
}

#[test]
#[serial]
fn stylex_dynamic_defaults_when_a_later_argument_is_absent_still_assign_its_default() {
    let source = "import stylex from '@stylexjs/stylex'; const styles=stylex.create({size:(height,width=12)=>({height,width})}); const result=stylex.props(styles.size(input));";
    let output = extract(source).expect("missing argument extraction");
    let actual = execute(&output, "const input=null;", "Object.values(result.style)");
    assert_eq!(actual, "[null,\"12px\"]");
}

#[rstest]
#[case("async (x)=>({'color':x})", "async arrow function", "async")]
#[case("(...x)=>({'color':x})", "rest parameter", "...")]
#[case(
    "({'x':value}={})=>({'color':value})",
    "destructuring parameter/default",
    "{'x'"
)]
#[case("(x)=>{return {color:x}}", "block/statement body", "{return")]
#[case("(x=getColor())=>({'color':x})", "non-exact default", "getColor")]
#[case("(x)=>({color:x+1})", "non-exact body value", "x+1")]
#[case("function*(x){yield x}", "generator function expression", "function")]
#[case("function(x){return {color:x}}", "function expression", "function")]
#[case(
    "async function(x){return {color:x}}",
    "async function expression",
    "async"
)]
#[serial]
fn stylex_dynamic_rejection_when_function_cannot_compile_is_located(
    #[case] form: &str,
    #[case] cause: &str,
    #[case] marker: &str,
) {
    let source = format!(
        "import stylex from '@stylexjs/stylex';\nconst styles=stylex.create({{tone:{form}}});"
    );
    let error = extract(&source)
        .expect_err("unsupported function")
        .to_string();
    let line = source.lines().nth(1).expect("definition");
    let column = line.find(marker).expect("error marker") + 1;
    assert!(
        error.contains(&format!("/src/stylex.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains(cause), "{error}");
    assert!(
        error.contains("use a") || error.contains("use a parameter"),
        "{error}"
    );
}

#[rstest]
#[case("tone(x){return {color:x}}")]
#[case("get tone(){return {color:'red'}}")]
#[serial]
fn stylex_dynamic_rejection_when_namespace_is_method_or_getter(#[case] form: &str) {
    let source =
        format!("import stylex from '@stylexjs/stylex';\nconst styles=stylex.create({{{form}}});");
    let error = extract(&source)
        .expect_err("method/getter rejected")
        .to_string();
    assert!(error.contains("/src/stylex.tsx:2:29:"), "{error}");
    assert!(error.contains("method/getter/setter namespace"), "{error}");
    assert!(error.contains("use a plain namespace"), "{error}");
}

#[rstest]
#[case("[styles.tone(input)]")]
#[case("ok ? styles.tone(input) : null")]
#[case("styles[key](input)")]
#[case("styles.tone(...inputs)")]
#[serial]
fn stylex_dynamic_rejection_when_composition_would_call_string_namespace(#[case] argument: &str) {
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const styles=stylex.create({{tone:(color)=>({{color}})}});\nconst result=stylex.props({argument});"
    );
    let error = extract(&source)
        .expect_err("unlowered call rejected")
        .to_string();
    assert!(error.contains("/src/stylex.tsx:2:"), "{error}");
    assert!(error.contains("cannot be compiled exactly"), "{error}");
    assert!(error.contains("pass"), "{error}");
}
