use crate::dead_properties_test_utils::failure;
use crate::{
    ExtractOption, ExtractSourceType, ImportAlias, ModuleResolution, ResolvedModule,
    extract_with_source_type,
};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use serial_test::serial;

#[test]
#[serial]
fn stylesheet_allows_dead_declaration_lookalikes_when_they_are_compiled_jsx_text()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: styling-looking JSX text is not an authored JavaScript call.
    reset_class_map();
    reset_file_map();
    let source = "import {style} from '@devup-ui/react';\nconst boxOrient = 'vertical'; export function view() { return <div>style({boxOrient})</div>; }\nexport const card = style({color: 'red'});";
    let resolver = |_: &str, _: &str| Some(ModuleResolution::Ignored);
    // When: an explicit compiled mode overrides the stylesheet's TS extension.
    let output = extract_with_source_type(
        "text.css.ts",
        source,
        ExtractOption::default(),
        false,
        Some(&resolver),
        Some(ExtractSourceType::CompiledMdx),
    );
    // Then: only the actual style call is evaluated and extracted.
    let output = output?;
    assert!(
        output.styles.iter().any(|style| matches!(
            style, crate::extract_style::extract_style_value::ExtractStyleValue::Static(style)
                if style.property == "color" && style.value == "red"
        )),
        "{output:?}"
    );
    Ok(())
}

#[test]
#[serial]
fn stylesheet_dead_declaration_keeps_call_origin_when_compiler_mode_overrides_ts() {
    // Given: compiled JSX precedes a computed declaration under a TS filename.
    reset_class_map();
    reset_file_map();
    let source = "import {style} from '@vanilla-extract/css';\nexport function view() { return <div/>; }\nconst key = 'box' + 'Orient';\nexport const bad = style({[key]: 'vertical'});";
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    let resolver = |_: &str, _: &str| Some(ModuleResolution::Ignored);
    // When: the stylesheet is instrumented and evaluated as compiled JavaScript.
    let error = failure(extract_with_source_type(
        "compiled.css.ts",
        source,
        option,
        false,
        Some(&resolver),
        Some(ExtractSourceType::CompiledMdx),
    ));
    // Then: validation reports the authored call rather than generated css().
    assert!(error.starts_with("compiled.css.ts:4:20:"), "{error}");
    assert!(
        error.contains("boxOrient") && error.contains("flex-direction"),
        "{error}"
    );
}

#[test]
#[serial]
fn imported_dead_declaration_keeps_origin_when_helper_is_compiled_mdx() {
    // Given: an imported compiled helper has JSX before its computed style call.
    reset_class_map();
    reset_file_map();
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(ResolvedModule {
        path: "helper.mdown".to_string(),
        code: "import {style} from '@vanilla-extract/css';\nexport function view() { return <div/>; }\nconst key = 'box' + 'Orient';\nexport const make = () => style({[key]: 'vertical'});".to_string(),
        source_type: Some(ExtractSourceType::CompiledMdx),
    }))
    };
    // When: a stylesheet invokes that helper through the real module loader.
    let error = failure(extract_with_source_type(
        "importer.css.ts",
        "import {style} from '@devup-ui/react'; import {make} from './helper'; export const bad = style(make());",
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
    ));
    // Then: the helper's authored styling call owns the error.
    assert!(error.starts_with("helper.mdown:4:27:"), "{error}");
    assert!(
        error.contains("boxOrient") && error.contains("flex-direction"),
        "{error}"
    );
}

#[test]
#[serial]
fn evaluated_dead_declaration_keeps_origin_when_compiled_jsx_precedes_css() {
    // Given: an aliased computed css call follows JSX in compiler output.
    reset_class_map();
    reset_file_map();
    let source = "import {css} from '@emotion/react';\nexport const view = <div/>;\nfunction makeRules() { return {selectors: {'&:hover': {boxOrient: 'vertical'}}}; }\nexport const bad = css(makeRules());";
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    // When: evaluation remaps generated declarations using the explicit grammar.
    let error = failure(extract_with_source_type(
        "page.mdown",
        source,
        option,
        true,
        None,
        Some(ExtractSourceType::CompiledMdx),
    ));
    // Then: the real file and authored css call survive alias and evaluation edits.
    assert!(error.starts_with("page.mdown:4:20:"), "{error}");
    assert!(
        error.contains("selectors -> &:hover -> boxOrient"),
        "{error}"
    );
}
