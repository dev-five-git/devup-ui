use crate::class_selection_expectations::{Declaration, Meaning, State};
use crate::extract_style::{extract_style_value::ExtractStyleValue, style_property::StyleProperty};
use crate::styled_source_tests::namespace_module_execution;
use crate::{ExtractOption, ExtractOutput, ImportAlias, ResolvedModule};
use std::collections::HashMap;

pub type Modules = Vec<(String, ExtractOutput)>;

pub fn compile(files: &[(&str, &str)], option: ExtractOption) -> Modules {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::debug::set_debug(true);
    let sources: Vec<_> = files
        .iter()
        .map(|(name, code)| ((*name).to_string(), (*code).to_string()))
        .collect();
    let resolver = move |specifier: &str, importer: &str| {
        let directory = importer
            .rsplit_once('/')
            .unwrap_or_else(|| panic!("absolute fixture path"))
            .0;
        let path = format!("{directory}/{}", specifier.trim_start_matches("./"));
        sources
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(name, code)| ResolvedModule {
                path: name.clone(),
                code: code.clone(),
            })
    };
    files
        .iter()
        .map(|(name, code)| {
            (
                (*name).into(),
                crate::extract_with_modules(name, code, option.clone(), false, &resolver)
                    .unwrap_or_else(|error| panic!("{name}: {error}")),
            )
        })
        .collect()
}

pub fn option(single_css: bool, package: &str) -> ExtractOption {
    ExtractOption {
        single_css,
        package: package.into(),
        css_dir: package.into(),
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    }
}

pub fn known(case: usize, single_css: bool) -> Modules {
    let source = format!(
        "import './environment.js';\n{}",
        crate::class_selection_fixtures::source(case)
    );
    compile(
        &[("/namespace/known.ts", &source)],
        option(single_css, "@devup-ui/core"),
    )
}

pub fn observe(outputs: &Modules, observer: &str, globals: &str) -> serde_json::Value {
    let environment = crate::extract(
        "/namespace/environment.js",
        globals,
        ExtractOption::default(),
    )
    .unwrap_or_else(|error| panic!("environment fixture: {error}"));
    let mut modules = vec![("/namespace/environment.js".into(), environment)];
    modules.extend(outputs.iter().map(|(name, output)| {
        (
            name.clone(),
            ExtractOutput {
                styles: output.styles.clone(),
                code: output.code.clone(),
                map: output.map.clone(),
                css_file: output.css_file.clone(),
                dependencies: output.dependencies.clone(),
            },
        )
    }));
    namespace_module_execution::execute(&modules, observer)
}

pub fn observe_known(outputs: &Modules, state: State) -> String {
    let globals = format!(
        "globalThis.on={};globalThis.other={};globalThis.ext={};globalThis.size={:?};globalThis.idx={};",
        state.on.javascript(),
        state.other,
        state.ext.javascript(),
        state.size,
        state.idx
    );
    observe(
        outputs,
        "import {result} from './known.ts';globalThis.observed=result;",
        &globals,
    )
    .as_str()
    .unwrap_or_else(|| panic!("selected result must be a string"))
    .into()
}

pub fn class(style: &ExtractStyleValue, filename: Option<&str>) -> Option<String> {
    match style.extract(filename) {
        Some(StyleProperty::ClassName(name)) => Some(name),
        Some(StyleProperty::Variable { .. }) => panic!("unexpected dynamic declaration: {style:?}"),
        None => None,
    }
}

pub fn meaning(style: &ExtractStyleValue, name: &str) -> Meaning {
    match style {
        ExtractStyleValue::Static(value) => Meaning::Declaration(Declaration {
            property: value.property.clone(),
            value: value.value.clone(),
            level: value.level,
            selector: value.selector.as_ref().map(ToString::to_string),
            layer: value.layer.clone(),
            order: value.style_order.unwrap_or(255),
        }),
        ExtractStyleValue::Typography(value) => {
            assert_eq!(name, format!("typo-{value}"));
            Meaning::Typography(value.clone())
        }
        ExtractStyleValue::Keyframes(value) => {
            assert_eq!(name, "k-525370705245237976");
            assert_eq!(value.keyframes.len(), 1);
            let from = &value.keyframes["from"];
            assert_eq!(from.len(), 1);
            assert_eq!(from[0].property, "opacity");
            assert_eq!(from[0].value, "0");
            assert_eq!(
                (
                    from[0].level,
                    &from[0].selector,
                    &from[0].layer,
                    from[0].style_order
                ),
                (0, &None, &None, None)
            );
            Meaning::Keyframes(name.into())
        }
        ExtractStyleValue::Dynamic(_)
        | ExtractStyleValue::Css(_)
        | ExtractStyleValue::Import(_)
        | ExtractStyleValue::FontFace(_)
        | ExtractStyleValue::LayerOrder(_) => panic!("unexpected selected metadata: {style:?}"),
    }
}

pub fn selected<'a>(
    outputs: &'a Modules,
    tokens: &str,
    single_css: bool,
) -> Vec<(String, &'a ExtractStyleValue)> {
    tokens
        .split_whitespace()
        .filter_map(|token| {
            let mut matching = outputs.iter().flat_map(|(name, output)| {
                output.styles.iter().filter(move |style| {
                    class(style, (!single_css).then_some(name.as_str())).as_deref() == Some(token)
                })
            });
            matching.next().map(|style| (token.into(), style))
        })
        .collect()
}

pub fn meanings(outputs: &Modules, tokens: &str, single_css: bool) -> Vec<Meaning> {
    let selected = selected(outputs, tokens, single_css);
    let mut result: Vec<_> = tokens
        .split_whitespace()
        .map(|token| {
            selected.iter().find(|(name, _)| name == token).map_or_else(
                || Meaning::Opaque(token.into()),
                |(name, style)| meaning(style, name),
            )
        })
        .collect();
    result.sort();
    result
}
