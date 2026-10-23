use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use boa_engine::{Context, Source};
use extractor::extract_style::{
    ExtractStyleProperty, extract_style_value::ExtractStyleValue, style_property::StyleProperty,
};
use extractor::{
    ExtractOption, ImportAlias, ResolvedModule, extract_with_modules, extract_without_source_map,
};
use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclarationSpecifier, Statement};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use serde_json::Value;

pub const BASE: &str = "import {styled} from '@devup-ui/react';const trace=[];const selected=true;const Base=styled.article({...((trace.push('controller'),selected)?{color:(trace.push('leaf'),'red')}:{}),color:'green'});export {Base,trace};";

pub fn run(files: &[(&str, &str)], observation: &str) -> Value {
    run_with_reading(files, observation, true)
}

pub fn run_with_reading(files: &[(&str, &str)], observation: &str, readable: bool) -> Value {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let modules: Vec<(String, String)> = files
        .iter()
        .map(|(path, code)| (path.to_string(), code.to_string()))
        .collect();
    let resolver = move |specifier: &str, _: &str| {
        let path = format!("/src/{}", specifier.trim_start_matches("./"));
        modules
            .iter()
            .find(|(file, _)| {
                ["", ".ts", ".tsx"]
                    .iter()
                    .any(|extension| format!("{path}{extension}") == *file)
            })
            .map(|(path, code)| ResolvedModule {
                path: path.clone(),
                code: code.clone(),
            })
    };
    let mut script = String::from(
        r"const modules={};const frames=[];const order=[];
function forwardRef(render){return {render};}
function jsx(target,props){return {target,props};}
function render(element){if(typeof element.target==='string')return element;const next=element.target.render(element.props,element.props.ref);frames.push(next);return render(next);}
function slots(e){return Object.keys(e.props).filter(k=>k.startsWith('__devupCss')||k.startsWith('__devupForwarded'));}
",
    );
    let mut rules = BTreeMap::new();
    for (filename, source) in files {
        let option = ExtractOption {
            single_css: true,
            import_aliases: HashMap::from([(
                "@emotion/react".to_string(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        };
        let output = if readable {
            extract_with_modules(filename, source, option, false, &resolver)
        } else {
            extract_without_source_map(filename, source, option)
        }
        .unwrap_or_else(|error| panic!("{filename}\n{source}\n{error}"));
        for style in &output.styles {
            let rule = match style {
                ExtractStyleValue::Static(style) => {
                    let StyleProperty::ClassName(name) = style.extract(None) else {
                        panic!("static atom must have a class")
                    };
                    Some((
                        name,
                        serde_json::json!([
                            style.property(),
                            style.value(),
                            style.level(),
                            style
                                .selector()
                                .map(ToString::to_string)
                                .unwrap_or_default()
                        ]),
                    ))
                }
                ExtractStyleValue::Dynamic(style) => {
                    let StyleProperty::Variable {
                        class_name,
                        variable_name,
                        ..
                    } = style.extract(None)
                    else {
                        panic!("dynamic atom must have a variable")
                    };
                    Some((
                        class_name,
                        serde_json::json!([
                            style.property(),
                            format!("var({variable_name})"),
                            style.level(),
                            style
                                .selector()
                                .map(ToString::to_string)
                                .unwrap_or_default()
                        ]),
                    ))
                }
                _ => None,
            };
            if let Some((name, rule)) = rule {
                rules.insert(name, rule);
            }
        }
        let arena = Allocator::default();
        let parsed = Parser::new(
            &arena,
            &output.code,
            SourceType::from_path(filename).unwrap_or_else(|error| panic!("{error}")),
        )
        .parse();
        assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
        let mut program = parsed.program;
        let scoping = SemanticBuilder::new()
            .build(&program)
            .semantic
            .into_scoping();
        let transformed = Transformer::new(
            &arena,
            std::path::Path::new(filename),
            &TransformOptions::default(),
        )
        .build_with_scoping(scoping, &mut program);
        assert_eq!(transformed.diagnostics.len(), 0, "{}", output.code);
        let mut imports = String::new();
        let mut exports = Vec::new();
        for statement in &program.body {
            match statement {
                Statement::ImportDeclaration(import) => {
                    for specifier in import.specifiers.iter().flatten() {
                        match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(named) => {
                                let imported = named.imported.name();
                                let local = named.local.name;
                                let helper = match imported.as_str() {
                                    "jsx" | "jsxs" | "jsxDEV" => Some("jsx"),
                                    "forwardRef" => Some("forwardRef"),
                                    _ => None,
                                };
                                if let Some(helper) = helper {
                                    if local != helper {
                                        write!(imports, "const {local}={helper};")
                                            .unwrap_or_else(|error| panic!("{error}"));
                                    }
                                } else if import.source.value.starts_with("./") {
                                    let module = resolver(&import.source.value, filename)
                                        .unwrap_or_else(|| panic!("fixture module must resolve"));
                                    write!(
                                        imports,
                                        "const {local}=modules[{:?}].{imported};",
                                        module.path
                                    )
                                    .unwrap_or_else(|error| panic!("{error}"));
                                }
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                                let module = resolver(&import.source.value, filename)
                                    .unwrap_or_else(|| panic!("fixture module must resolve"));
                                write!(
                                    imports,
                                    "const {}=modules[{:?}];",
                                    namespace.local.name, module.path
                                )
                                .unwrap_or_else(|error| panic!("{error}"));
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                                let module = resolver(&import.source.value, filename)
                                    .unwrap_or_else(|| panic!("fixture module must resolve"));
                                write!(
                                    imports,
                                    "const {}=modules[{:?}].default;",
                                    default.local.name, module.path
                                )
                                .unwrap_or_else(|error| panic!("{error}"));
                            }
                        }
                    }
                }
                Statement::ExportNamedDeclaration(export) => {
                    exports.extend(export.specifiers.iter().map(|specifier| {
                        format!("{}:{}", specifier.exported.name(), specifier.local.name())
                    }));
                }
                Statement::ExportDefaultDeclaration(export) => {
                    let mut codegen = Codegen::new();
                    codegen.print_expression(
                        export
                            .declaration
                            .as_expression()
                            .unwrap_or_else(|| panic!("fixture default must be an expression")),
                    );
                    exports.push(format!("default:({})", codegen.into_source_text()));
                }
                _ => {}
            }
        }
        program.body.retain(|statement| {
            !matches!(
                statement,
                Statement::ImportDeclaration(_)
                    | Statement::ExportNamedDeclaration(_)
                    | Statement::ExportDefaultDeclaration(_)
            )
        });
        write!(
            script,
            "modules[{filename:?}]=(()=>{{{imports}{}return {{{}}};}})();",
            Codegen::new().build(&program).code,
            exports.join(",")
        )
        .unwrap_or_else(|error| panic!("{error}"));
    }
    write!(
        script,
        "const rules={};",
        serde_json::to_string(&rules).unwrap_or_else(|error| panic!("{error}"))
    )
    .unwrap_or_else(|error| panic!("{error}"));
    script.push_str(r"function values(e,property,level=0,selector=''){return (e.props.className||'').split(/\s+/).flatMap(k=>rules[k]?[rules[k]]:[]).filter(r=>r[0]===property&&r[2]===level&&r[3]===selector).map(r=>r[1].startsWith('var(')?e.props.style[r[1].slice(4,-1)]:r[1]).sort();}
function identities(e){return (e.props.className||'').split(/\s+/).filter(k=>k.includes('--'));}
");
    script.push_str(observation);
    let result = Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{error}\n{script}"));
    let json = result
        .as_string()
        .unwrap_or_else(|| panic!("observation must be JSON"))
        .to_std_string_escaped();
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("{error}\n{json}\n{script}"))
}
