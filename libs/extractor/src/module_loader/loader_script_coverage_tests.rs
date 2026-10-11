use rstest::rstest;
use serial_test::serial;

use super::super::{ModuleLoader, Unit, module_script};
use crate::{ExtractOption, ExtractStyleValue, ImportAlias, ResolvedModule};

#[test]
#[serial]
fn imported_bare_tag_is_unbound_when_the_strict_helper_checks_its_receiver()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    let source = "import {style} from '@devup-ui/react';import {tag} from './tag';export const box=style({color:tag`blue`});";
    let helper = "export function tag(parts){'use strict';if(this!==undefined)throw new Error('bound tag receiver');return parts[0];}";
    let resolver = move |specifier: &str, importer: &str| {
        ((specifier, importer) == ("./tag", "/tag.css.ts")).then(|| ResolvedModule {
            path: "/tag.ts".into(),
            code: helper.into(),
        })
    };
    let option = ExtractOption {
        single_css: true,
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".into(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    // When
    let output = crate::extract_with_modules("/tag.css.ts", source, option, false, &resolver)?;
    // Then
    let colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(value) if value.property == "color" => Some(value.value()),
            _ => None,
        })
        .collect();
    assert_eq!(colors, ["blue"]);
    Ok(())
}

#[rstest]
#[case(
    "export default function paint(){return paint.name==='paint'?'blue':'red';}export {paint};",
    "import value,{paint as named} from './default';const result=[value===named,value(),value.name];",
    "[true,\"blue\",\"paint\"]"
)]
#[case(
    "export default class Palette{static color(){return Palette.name==='Palette'?'blue':'red';}}export {Palette};",
    "import Value,{Palette as Named} from './default';const result=[Value===Named,Value.color(),Value.name];",
    "[true,\"blue\",\"Palette\"]"
)]
fn named_default_keeps_local_identity_when_the_written_module_is_loaded(
    #[case] module: &'static str,
    #[case] source: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given
    let resolver = move |specifier: &str, importer: &str| {
        ((specifier, importer) == ("./default", "/script-entry.ts")).then(|| ResolvedModule {
            path: "/default.ts".into(),
            code: module.into(),
        })
    };
    let option = ExtractOption::default();
    let mut loader = ModuleLoader::new(Some(&resolver), &option);
    let unit = Unit::written("/script-entry.ts", source, source, &[])?;
    let entry = module_script(&unit, &mut loader, true)?;
    let script = loader.script(&entry);
    let mut context = boa_engine::Context::default();
    // When
    let result = context
        .eval(boa_engine::Source::from_bytes(&format!(
            "{}\nJSON.stringify(result)",
            script.text
        )))
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(
        result
            .to_string(&mut context)
            .map_err(|error| error.to_string())?
            .to_std_string_escaped(),
        expected
    );
    Ok(())
}
