use crate::dead_properties_test_utils::{failure, position};
use crate::{ExtractOption, ImportAlias, ResolvedModule, extract, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

fn options() -> ExtractOption {
    ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    }
}

#[rstest]
#[case(
    "export const c = style(rules);",
    "style",
    "selectors -> &:hover -> boxOrient"
)]
#[case(
    "globalStyle('body', rules);",
    "globalStyle",
    "selectors -> &:hover -> boxOrient"
)]
#[case(
    "export const k = keyframes({ from: rules });",
    "keyframes",
    "from -> selectors -> &:hover -> boxOrient"
)]
#[case(
    "export const v = styleVariants({ a: 1 }, () => rules);",
    "styleVariants",
    "selectors -> &:hover -> boxOrient"
)]
#[case(
    "export const f = fontFace(rules);",
    "fontFace",
    "selectors -> &:hover -> boxOrient"
)]
#[case(
    "globalFontFace('Inter', rules);",
    "globalFontFace",
    "selectors -> &:hover -> boxOrient"
)]
#[serial]
fn evaluated_rules_report_authored_call_and_path(
    #[case] statement: &str,
    #[case] api: &str,
    #[case] path: &str,
) {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ style, globalStyle, keyframes, styleVariants, fontFace, globalFontFace }} from '@vanilla-extract/css';\nconst key = 'box' + 'Orient';\nconst rules = {{ selectors: {{ '&:hover': {{ [key]: 'vertical' }} }} }};\n{statement}"
    );
    let column = position(statement, &format!("{api}(")) + 1;

    let error = failure(extract("origin.css.ts", &source, options()));

    assert!(
        error.starts_with(&format!("origin.css.ts:4:{column}:")),
        "{error}"
    );
    assert!(error.contains(path), "{error}");
    assert!(error.contains("flex-direction"), "{error}");
}

#[rstest]
#[case("export const c = style({ boxOrient: 'vertical' });")]
#[case("export const k = keyframes({ from: { boxOrient: 'vertical' } });")]
#[case("globalStyle('body', { boxOrient: 'vertical' });")]
#[serial]
fn literal_stylesheet_declarations_keep_exact_key_locations(#[case] statement: &str) {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ style, globalStyle, keyframes }} from '@vanilla-extract/css';\n{statement}"
    );
    let column = position(statement, "boxOrient") + 1;

    let error = failure(extract("literal.css.ts", &source, options()));

    assert!(
        error.starts_with(&format!("literal.css.ts:2:{column}:")),
        "{error}"
    );
}

#[test]
#[serial]
fn imported_helper_call_keeps_its_authored_filename() {
    reset_class_map();
    reset_file_map();
    let source = "import { style } from '@vanilla-extract/css';\nimport { make } from './helper';\nexport const c = style(make());";
    let resolver = |specifier: &str, _: &str| {
        match specifier {
        "./helper" => Some(ResolvedModule {
            path: "helper.ts".to_string(),
            code: "import { style } from '@vanilla-extract/css';\nconst key = 'box' + 'Orient';\nexport const make = () => style({ [key]: 'vertical' });".to_string(),
        }),
        _ => None,
    }
    };
    let error = failure(extract_with_modules(
        "importer.css.ts",
        source,
        options(),
        false,
        &resolver,
    ));

    assert!(error.starts_with("helper.ts:3:27:"), "{error}");
    assert!(error.contains("boxOrient"), "{error}");
}

#[test]
#[serial]
fn imported_stylesheet_call_keeps_its_authored_filename() {
    reset_class_map();
    reset_file_map();
    let source = "import { style } from '@vanilla-extract/css';\nimport { c } from './other.css';\nexport const result = style([c, { color: 'red' }]);";
    let resolver = |specifier: &str, _: &str| {
        match specifier {
        "./other.css" => Some(ResolvedModule {
            path: "other.css.ts".to_string(),
            code: "import { style } from '@vanilla-extract/css';\nconst key = 'box' + 'Orient';\nexport const c = style({ [key]: 'vertical' });".to_string(),
        }),
        _ => None,
    }
    };
    let error = failure(extract_with_modules(
        "importer.css.ts",
        source,
        options(),
        false,
        &resolver,
    ));

    assert!(error.starts_with("other.css.ts:3:18:"), "{error}");
}

#[rstest]
#[case("const c = css(rules);", "css(")]
#[case("const k = keyframes({ from: rules });", "keyframes(")]
#[case("globalCss({ body: rules });", "globalCss(")]
#[case("const S = styled.div(rules);", "styled.div(")]
#[case("const e = <Box _hover={rules} />;", "<Box")]
#[serial]
fn imported_objects_report_the_authored_styling_call(#[case] statement: &str, #[case] call: &str) {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ Box, css, keyframes, globalCss, styled }} from '@devup-ui/react';\nimport {{ rules }} from './rules';\n{statement}"
    );
    let resolver = |specifier: &str, _: &str| match specifier {
        "./rules" => Some(ResolvedModule {
            path: "rules.ts".to_string(),
            code: "export const rules = { selectors: { '&:hover': { boxOrient: 'vertical' } } };"
                .to_string(),
        }),
        _ => None,
    };

    let error = failure(extract_with_modules(
        "imported.tsx",
        &source,
        ExtractOption::default(),
        false,
        &resolver,
    ));

    assert!(
        error.starts_with(&format!(
            "imported.tsx:3:{}:",
            position(statement, call) + 1
        )),
        "{error}"
    );
    assert!(
        error.contains("selectors -> &:hover -> boxOrient"),
        "{error}"
    );
}

#[test]
#[serial]
fn variant_mapper_data_is_not_a_declaration() {
    reset_class_map();
    reset_file_map();
    let source = "import { styleVariants } from '@devup-ui/react';\nexport const c = styleVariants({ a: { boxOrient: 'red' } }, data => ({ color: data.boxOrient }));";

    let result = extract("variants.css.ts", source, ExtractOption::default());

    assert!(result.is_ok(), "{result:?}");
}

#[test]
#[serial]
fn computed_object_reports_its_authored_css_call() {
    reset_class_map();
    reset_file_map();
    let source = "import { css } from '@devup-ui/react';\nfunction makeRules() { return { selectors: { '&:hover': { boxOrient: 'vertical' } } }; }\nconst c = css(makeRules());";

    let error = failure(extract("computed.tsx", source, ExtractOption::default()));

    assert!(error.starts_with("computed.tsx:3:11:"), "{error}");
    assert!(
        error.contains("selectors -> &:hover -> boxOrient"),
        "{error}"
    );
}

#[rstest]
#[case("import { rules } from ", "; export { rules };", 26)]
#[case("const { rules } = require(", "); module.exports = { rules };", 30)]
#[case("export { rules } from ", ";", 26)]
#[case("export * from ", ";", 18)]
#[serial]
fn required_module_specifiers_keep_authored_locations(
    #[case] before: &str,
    #[case] after: &str,
    #[case] column: usize,
    #[values("\\uD800", "\\uDC00")] specifier: &str,
) {
    // Given: styles read the required module after TypeScript and instrumentation edits.
    reset_class_map();
    reset_file_map();
    let source = "import { style } from '@vanilla-extract/css';\nimport { rules } from './required';\nexport const result = style(rules);";
    let required = format!(
        "// Authored module, not generated JavaScript.\nconst unused: number = 1; const notModule = '{specifier}'; Math.max(1, 2); Number(0); function ignored(require: (x: string) => void) {{ require('{specifier}'); }} function optional() {{ require(0); }}\n   {before}'{specifier}'{after}"
    );
    let resolver = move |specifier: &str, _: &str| match specifier {
        "./required" => Some(ResolvedModule {
            path: "required.ts".to_string(),
            code: required.clone(),
        }),
        _ => None,
    };

    // When: the public module-aware extraction must load the rules.
    let error = failure(extract_with_modules(
        "required-origin.css.ts",
        source,
        options(),
        false,
        &resolver,
    ));

    // Then: the offending literal owns the error, not the entry's style() call.
    assert!(
        error.starts_with(&format!("required.ts:3:{column}:")),
        "{error}"
    );
    assert!(error.contains(specifier), "{error}");
}

#[rstest]
#[case("import { rules } from ", "; export { rules };")]
#[case("const { rules } = require(", "); module.exports = { rules };")]
#[case("export { rules } from ", ";")]
#[case("export * from ", ";")]
#[serial]
fn required_module_specifiers_accept_complete_surrogate_pairs(
    #[case] before: &str,
    #[case] after: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: the same required reads name a representable astral module.
    reset_class_map();
    reset_file_map();
    let required = format!("\nconst unused: number = 1;\n   {before}'\\uD83D\\uDE00'{after}");
    let resolver = move |specifier: &str, _: &str| match specifier {
        "./required" => Some(ResolvedModule {
            path: "required.ts".to_string(),
            code: required.clone(),
        }),
        "😀" => Some(ResolvedModule {
            path: "rules.ts".to_string(),
            code: "export const rules = { color: 'red' };".to_string(),
        }),
        _ => None,
    };

    // When: the evaluator consumes the required rules.
    let output = extract_with_modules(
        "required-control.css.ts",
        "import { style } from '@vanilla-extract/css';\nimport { rules } from './required';\nexport const result = style(rules);",
        options(),
        false,
        &resolver,
    )?;

    // Then: real rules, not an unused-function fixture, were extracted.
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(style) => Some((style.property(), style.value())),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![("color", "red")]);
    Ok(())
}

#[rstest]
#[serial]
fn required_module_specifiers_in_entry_refuse_plain_extraction_fallback(
    #[values("\\uD800", "\\uDC00")] specifier: &str,
) {
    // Given: a required import follows a styling call that instrumentation expands.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ style }} from '@vanilla-extract/css';\nexport const first = style({{ color: 'blue' }});\n   import {{ rules }} from '{specifier}';\nexport const result = style(rules);"
    );

    // When: evaluation cannot represent that module name.
    let error = failure(extract("entry.css.ts", &source, options()));

    // Then: the original literal wins over a fallback style-value diagnostic.
    assert!(error.starts_with("entry.css.ts:3:26:"), "{error}");
    assert!(error.contains(specifier), "{error}");
}

#[rstest]
#[case("import type { Gone } from ")]
#[case("import { type Gone } from ")]
#[case("export type { Gone } from ")]
#[case("export { type Gone } from ")]
#[case("export type * from ")]
#[serial]
fn type_only_module_refusal_reports_required_site(
    #[case] erased: &str,
    #[values("\\uD800", "\\uDC00")] specifier: &str,
) {
    // Given: an erased declaration and a required ESM import share the same name.
    reset_class_map();
    reset_file_map();
    let required = format!(
        "// Original helper source.\n{erased}'{specifier}';\nconst unused: number = 1;\n   import {{ rules }} from '{specifier}';\nexport {{ rules }};"
    );
    let resolver = move |specifier: &str, _: &str| match specifier {
        "./required" => Some(ResolvedModule {
            path: "required-types.ts".to_string(),
            code: required.clone(),
        }),
        _ => None,
    };

    // When: stylesheet extraction actually needs the imported rules.
    let error = failure(extract_with_modules(
        "type-only-origin.css.ts",
        "import { style } from '@vanilla-extract/css';\nimport { rules } from './required';\nexport const result = style(rules);",
        options(),
        false,
        &resolver,
    ));

    // Then: only the actual read's opening quote is reported, never the erased site.
    assert!(error.starts_with("required-types.ts:4:26:"), "{error}");
    assert_eq!(error.lines().count(), 1, "{error}");
    assert!(!error.contains("required-types.ts:2:"), "{error}");
}

#[rstest]
#[case("import type { Gone } from ")]
#[case("import { type Gone } from ")]
#[case("export type { Gone } from ")]
#[case("export { type Gone } from ")]
#[case("export type * from ")]
#[serial]
fn type_only_module_declarations_allow_required_rules(
    #[case] erased: &str,
    #[values("\\uD800", "\\uDC00")] specifier: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: the same ESM shape has a representable required module name.
    reset_class_map();
    reset_file_map();
    let required = format!(
        "// Original helper source.\n{erased}'{specifier}';\nconst unused: number = 1;\n   import {{ rules }} from '\\uD83D\\uDE00';\nexport {{ rules }};"
    );
    let resolver = move |specifier: &str, _: &str| match specifier {
        "./required" => Some(ResolvedModule {
            path: "required-types.ts".to_string(),
            code: required.clone(),
        }),
        "😀" => Some(ResolvedModule {
            path: "rules.ts".to_string(),
            code: "export const rules = { color: 'red' };".to_string(),
        }),
        _ => None,
    };

    // When: the public evaluator consumes the helper's real runtime export.
    let output = extract_with_modules(
        "type-only-control.css.ts",
        "import { style } from '@vanilla-extract/css';\nimport { rules } from './required';\nexport const result = style(rules);",
        options(),
        false,
        &resolver,
    )?;

    // Then: the required value produced actual static styles, not an unused fixture.
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(style) => Some((style.property(), style.value())),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![("color", "red")]);
    Ok(())
}

#[rstest]
#[case(
    "import { style as make } from '@devup-ui/react';",
    "const c = make(rules);",
    "make("
)]
#[case(
    "import * as ve from '@devup-ui/react';",
    "const c = ve.style(rules);",
    "ve.style("
)]
#[case(
    "import { style } from '@devup-ui/react'; const make = style;",
    "const c = make(rules);",
    "make("
)]
#[case(
    "import { style } from '@devup-ui/react'; const fns = { make: style };",
    "const c = fns.make(rules);",
    "fns.make("
)]
#[case(
    "import { style } from '@devup-ui/react'; const fns = [style]; const i = 0;",
    "const c = fns[i](rules);",
    "fns[i]("
)]
#[serial]
fn stylesheet_api_aliases_preserve_call_origins(
    #[case] imports: &str,
    #[case] statement: &str,
    #[case] call: &str,
) {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "{imports}\nconst key = 'box' + 'Orient'; const rules = {{ [key]: 'vertical' }};\n{statement}"
    );

    let error = failure(extract("aliases.css.ts", &source, ExtractOption::default()));

    assert!(
        error.starts_with(&format!(
            "aliases.css.ts:3:{}:",
            position(statement, call) + 1
        )),
        "{error}"
    );
    assert!(error.contains("boxOrient"), "{error}");
}
