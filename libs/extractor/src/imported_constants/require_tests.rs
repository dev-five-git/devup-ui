use crate::{ExtractOption, ExtractStyleValue, ResolvedModule};
use rstest::rstest;
use serial_test::serial;
use std::cell::RefCell;
use std::rc::Rc;

fn require_probe(module: &str) -> (Vec<String>, Result<crate::ExtractOutput, String>) {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let requested = Rc::new(RefCell::new(Vec::new()));
    let recorded = requested.clone();
    let module = module.to_string();
    let resolver = move |specifier: &str, _: &str| {
        recorded.borrow_mut().push(specifier.to_string());
        let source = match specifier {
            "./values" => module.as_str(),
            "./trap" => "export const color = 'green';",
            "./local" => "export const require = () => ({ color: 'red' });",
            _ => return None,
        };
        Some(ResolvedModule {
            path: format!("/src/{specifier}.ts"),
            code: source.to_string(),
        })
    };
    let output = crate::extract_with_modules("/src/App.tsx", "import {Box} from '@devup-ui/react';import {COLOR} from './values';export const a=<Box color={COLOR}/>;", ExtractOption::default(),false,&resolver).map_err(|error|error.to_string());
    (requested.borrow().clone(), output)
}

#[rstest]
#[case(
    "const require=()=>({color:'red'});const values=require('./trap');export const COLOR=values.color;"
)]
#[case(
    "function require(){return {color:'red'};}const values=require('./trap');export const COLOR=values.color;"
)]
#[case("let require=load;const values=require('./trap');export const COLOR=values.color;")]
#[case("class require{}const values=require('./trap');export const COLOR=values.color;")]
#[case(
    "import {require} from './local';const values=require('./trap');export const COLOR=values.color;"
)]
#[case("const values=require('./trap');const require=load;export const COLOR=values.color;")]
#[serial]
fn shadowed_require_when_reading_module_constants_never_resolves_a_module(#[case] module: &str) {
    let (requested, output) = require_probe(module);
    assert!(
        !requested.iter().any(|specifier| specifier == "./trap"),
        "{requested:?}"
    );
    if let Ok(output) = output {
        assert!(
            !output.styles.iter().any(
                |style| matches!(style,ExtractStyleValue::Static(style) if style.value=="green")
            ),
            "{}",
            output.code
        );
    }
}

#[rstest]
#[case(
    "export function f(require){const values=require('./trap');return values.color;}export const COLOR=f(local);"
)]
#[case(
    "try{throw local;}catch(require){const values=require('./trap');observe(values);}export const COLOR=dynamic;"
)]
#[case(
    "const require=local;{const values=require('./trap');observe(values);}export const COLOR=dynamic;"
)]
#[serial]
fn enclosing_require_when_nested_calls_are_read_never_loads_the_named_module(#[case] module: &str) {
    let (requested, _) = require_probe(module);
    assert!(
        !requested.iter().any(|specifier| specifier == "./trap"),
        "{requested:?}"
    );
}

#[test]
#[serial]
fn unbound_require_when_reading_module_constants_still_loads() {
    let (requested, output) =
        require_probe("const values=require('./trap');export const COLOR=values.color;");
    let output = output.unwrap_or_else(|error| panic!("{error}"));
    assert!(
        requested.iter().any(|specifier| specifier == "./trap"),
        "{requested:?}"
    );
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style,ExtractStyleValue::Static(style) if style.value=="green")),
        "{}",
        output.code
    );
}
