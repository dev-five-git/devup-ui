use boa_engine::{
    Context, JsValue, Module, Source, builtins::promise::PromiseState, module::MapModuleLoader,
};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_codegen::Codegen;
use oxc_semantic::SemanticBuilder;
use oxc_transformer::{TransformOptions, Transformer};
use std::{path::Path, rc::Rc};

pub(in super::super) fn compile(
    files: &[(&str, &str)],
    single_css: bool,
) -> Vec<(String, crate::ExtractOutput)> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let sources: Vec<_> = files
        .iter()
        .map(|(name, code)| ((*name).to_string(), (*code).to_string()))
        .collect();
    let resolver = move |specifier: &str, importer: &str| {
        let directory = importer
            .rsplit_once('/')
            .unwrap_or_else(|| panic!("absolute module fixture path"))
            .0;
        let path = format!("{directory}/{}", specifier.trim_start_matches("./"));
        sources
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(name, code)| crate::ResolvedModule {
                path: name.clone(),
                code: code.clone(),
            })
    };
    let option = crate::ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".into(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..crate::ExtractOption::default()
    };
    files
        .iter()
        .map(|(name, code)| {
            let output = crate::extract_with_modules(name, code, option.clone(), false, &resolver)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            ((*name).into(), output)
        })
        .collect()
}

pub(in super::super) fn execute(
    outputs: &[(String, crate::ExtractOutput)],
    observation: &str,
) -> serde_json::Value {
    let loader = Rc::new(MapModuleLoader::new());
    let mut context = Context::builder()
        .module_loader(loader.clone())
        .build()
        .unwrap_or_else(|error| panic!("context fixture: {error:?}"));
    context.eval(Source::from_bytes(r"
function forwardRef(render){return {render};}
function jsx(target,props){return {target,props};}
function require(name){if(name==='react')return {forwardRef};if(name==='react/jsx-runtime')return {jsx,jsxs:jsx};throw new Error(name);}
function render(element){return typeof element.target==='string'?element:typeof element.target==='function'?render((0,element.target)(element.props)):render(element.target.render(element.props,element.props.ref));}
function slots(element){return Object.keys(element.props).filter(key=>key.startsWith('__devupCss'));}
")).unwrap_or_else(|error| panic!("runtime fixture: {error:?}"));
    for (name, code) in [
        ("react", "export const forwardRef=globalThis.forwardRef;"),
        (
            "react/jsx-runtime",
            "export const jsx=globalThis.jsx;export const jsxs=jsx;",
        ),
        ("@emotion/react", "export const jsx=globalThis.jsx;"),
        (
            "@devup-ui/react/compat",
            "export const jsx=globalThis.jsx;export const jsxs=jsx;",
        ),
    ] {
        let module = Module::parse(Source::from_bytes(code), None, &mut context)
            .unwrap_or_else(|error| panic!("shim fixture: {error:?}"));
        let key = Path::new(name).components().collect::<std::path::PathBuf>();
        assert_eq!(
            loader.insert(
                key.to_str().unwrap_or_else(|| panic!("UTF-8 module path")),
                module
            ),
            None
        );
    }
    for (name, output) in outputs {
        let arena = Allocator::default();
        let parsed = oxc_parser::Parser::new(
            &arena,
            &output.code,
            oxc_span::SourceType::from_path(name)
                .unwrap_or_else(|error| panic!("source type fixture: {error:?}")),
        )
        .parse();
        assert_eq!(
            parsed.diagnostics.len(),
            0,
            "{}\n{:?}",
            output.code,
            parsed.diagnostics
        );
        let mut program = parsed.program;
        let scoping = SemanticBuilder::new()
            .build(&program)
            .semantic
            .into_scoping();
        let transformed = Transformer::new(&arena, Path::new(name), &TransformOptions::default())
            .build_with_scoping(scoping, &mut program);
        assert_eq!(transformed.diagnostics.len(), 0, "{}", output.code);
        program.body.retain(|statement| {
            !matches!(statement,
            Statement::ImportDeclaration(import) if import.source.value.ends_with(".css"))
        });
        let code = Codegen::new().build(&program).code;
        let module = Module::parse(
            Source::from_bytes(&code).with_path(Path::new(name)),
            None,
            &mut context,
        )
        .unwrap_or_else(|error| panic!("{error}\n{code}"));
        let key = Path::new(name).components().collect::<std::path::PathBuf>();
        assert_eq!(
            loader.insert(
                key.to_str().unwrap_or_else(|| panic!("UTF-8 module path")),
                module
            ),
            None
        );
    }
    let module = Module::parse(
        Source::from_bytes(observation).with_path(Path::new("/namespace/observe.js")),
        None,
        &mut context,
    )
    .unwrap_or_else(|error| panic!("observer module fixture: {error:?}"));
    let promise = module.load_link_evaluate(&mut context);
    context
        .run_jobs()
        .unwrap_or_else(|error| panic!("module jobs: {error:?}"));
    if let PromiseState::Rejected(error) = promise.state() {
        panic!(
            "module evaluation: {}\n{outputs:?}",
            error
                .to_string(&mut context)
                .unwrap_or_else(|error| panic!("error string conversion: {error:?}"))
                .to_std_string_escaped()
        );
    }
    assert_eq!(
        promise.state(),
        PromiseState::Fulfilled(JsValue::undefined()),
        "{observation}"
    );
    let result = context
        .eval(Source::from_bytes("JSON.stringify(globalThis.observed)"))
        .unwrap_or_else(|error| panic!("JSON observation: {error:?}"));
    serde_json::from_str(
        &result
            .as_string()
            .unwrap_or_else(|| panic!("JSON string observation"))
            .to_std_string_escaped(),
    )
    .unwrap_or_else(|error| panic!("JSON observation parse: {error:?}"))
}
