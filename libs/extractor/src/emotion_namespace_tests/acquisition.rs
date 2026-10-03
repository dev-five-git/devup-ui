use super::*;
use rstest::rstest;

const IMPORT: &str = "import * as E from '@emotion/css';\n";

#[rstest]
#[case("import E = require('@emotion/css');", "export const a=E.css({ padding: 8 });")]
#[case("const E = require('@emotion/css');", "export const a=E.css({ padding: 8 });")]
#[case("var E = require('@emotion/css');", "export const a=E['cx']('a');")]
#[case("const E = require('@emotion/css');", "const c=E.css; export const a=c({});")]
#[case("export {};", "const { css } = require('@emotion/css'); css({ padding: 8 });")]
#[case("export {};", "export const a=require('@emotion/css').css({ padding: 8 });")]
#[case("export {};", "const c=require('@emotion/css').css; c({ padding: 8 });")]
#[case("export {};", "const {cache, ...rest} = require('@emotion/css');")]
#[case("export {};", "consume(require('@emotion/css'));")]
#[serial]
fn require_acquired_styling_reports_a_located_error(#[case] head: &str, #[case] body: &str) {
    // Given: the styling API would otherwise stay a runtime Emotion call
    // When
    let error = compile(&format!("{head}\n{body}")).unwrap_err();

    // Then
    assert!(error.starts_with("namespace.tsx:2:"), "{error}");
    assert!(error.contains("cannot use"), "{error}");
}

#[rstest]
#[case("import E = require('@emotion/css');", "E.css")]
#[case("export {};", "require('@emotion/css').cx")]
#[serial]
fn require_acquired_styling_asks_for_an_es_import(#[case] head: &str, #[case] read: &str) {
    // Given
    let code = format!("{head}\nexport const a={read}({{padding:8}});");

    // When
    let error = compile(&code).unwrap_err();

    // Then
    assert!(error.contains(&format!("cannot use `{read}`")), "{error}");
    assert!(error.contains("require(), import-equals"), "{error}");
    assert!(error.contains("import * as ns from '@emotion/css'"), "{error}");
}

#[test]
#[serial]
fn exported_import_equals_reports_the_namespace_re_export() {
    // Given
    let code = "export import E = require('@emotion/css');";

    // When
    let error = compile(code).unwrap_err();

    // Then
    assert!(error.starts_with("namespace.tsx:1:1:"), "{error}");
    assert!(error.contains("cannot be exported"), "{error}");
}

#[rstest]
#[case("const { flush, cache: { key } } = require('@emotion/css'); flush(); console.log(key);")]
#[case("const E = require('@emotion/css'); E.cache; E['flush'](); E?.sheet;")]
#[case("var E = require('@emotion/css'), N = E; N.hydrate();")]
#[case("import E = require('@emotion/css'); console.log(E.sheet, E?.cache);")]
#[case("function require(){} require('@emotion/css').css({ padding: 8 });")]
#[case("import type E = require('@emotion/css'); type Api = typeof E;")]
#[case("import fs = require('fs'); export { fs };")]
#[serial]
fn runtime_only_require_use_stays_untouched(#[case] body: &str) {
    // Given
    // When
    let output = compile(body).unwrap();

    // Then
    assert_eq!(output.code, body);
}

#[test]
#[serial]
fn runtime_destructuring_keeps_nested_patterns_defaults_and_optional_reads() {
    // Given
    let code = format!(
        "{IMPORT}const {{css, cache: {{key}}, flush = () => {{}}}} = E;\nexport const a = css({{padding: 8}});\nexport const b = [key, flush, E?.cache, E?.['sheet']];"
    );

    // When
    let output = compile(&code).unwrap();

    // Then
    assert!(format!("{:?}", output.styles).contains("8px"), "{}", output.code);
    assert!(output.code.contains("@emotion/css"), "{}", output.code);
    assert!(output.code.contains("E?.cache"), "{}", output.code);
    assert!(output.code.contains("cache: { key }"), "{}", output.code);
    assert!(!output.code.contains("css("), "{}", output.code);
}

#[rstest]
#[case("const {cache, ...rest} = E;")]
#[case("const {css = other, cache} = E;")]
#[case("const { css: { call }, cache } = E;")]
#[case("export const a = E?.css({});")]
#[case("export const a = E?.['cx']('a');")]
#[case("import N = E; N.css({ padding: 8 });")]
#[case("import c = E.css; c({ padding: 8 });")]
#[serial]
fn macro_shapes_without_a_build_time_meaning_stay_errors(#[case] body: &str) {
    // Given
    // When
    let error = compile(&format!("{IMPORT}{body}")).unwrap_err();

    // Then
    assert!(error.starts_with("namespace.tsx:2:"), "{error}");
}

#[test]
#[serial]
fn string_literal_export_names_compile_by_their_value() {
    // Given
    let code = "import {'css' as c, 'cx' as join} from '@emotion/css'; const a=c({ padding: 8 }); export const b=join(a,{picked:8});";

    // When
    let output = compile(code).unwrap();

    // Then
    assert!(format!("{:?}", output.styles).contains("8px"), "{}", output.code);
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}

#[test]
fn string_literal_export_names_are_quoted_only_when_retained() {
    // Given
    let code = "import {'css' as c, 'cx' as join, 'weird-name' as w} from '@emotion/css'";
    let aliases = HashMap::from([("@emotion/css".to_string(), ImportAlias::NamedToNamed)]);

    // When
    let output = import_alias_visit::transform_import_aliases(
        code,
        "namespace.tsx",
        "@devup-ui/react",
        &aliases,
    );

    // Then
    assert_eq!(
        output,
        "import { css as c } from '@devup-ui/react'; import { cx as join } from '@devup-ui/react/compat'; import { \"weird-name\" as w } from '@emotion/css';"
    );
}
