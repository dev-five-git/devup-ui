use super::*;
use rstest::rstest;

fn compile_file(filename: &str, code: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    extract(
        filename,
        code,
        ExtractOption {
            import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string())
}

#[rstest]
#[case("a.js", "const r = require; export const a = r('@emotion/css').css({ padding: 8 });")]
#[case("a.cjs", "const load = module.require; const E = load('@emotion/css'); E.css({ padding: 8 });")]
#[case("a.cjs", "const E = module.require('@emotion/css'); E['cx']('a');")]
#[case("a.js", "const name = '@emotion/css'; require(name).keyframes({});")]
#[case("a.js", "const E = await import('@emotion/css'); E.css({ padding: 8 });")]
#[case("a.js", "export const a = (await import('@emotion/css')).css({ padding: 8 });")]
#[case("a.js", "const { css } = await import('@emotion/css'); css({ padding: 8 });")]
#[case(
    "a.mjs",
    "import { createRequire } from 'node:module'; const E = createRequire(import.meta.url)('@emotion/css'); E.cx('a');"
)]
#[case(
    "a.mjs",
    "import * as m from 'module'; const req = m.createRequire(import.meta.url); req('@emotion/css').css({ padding: 8 });"
)]
#[case(
    "a.mjs",
    "import mod from 'module'; const { css } = mod.createRequire(import.meta.url)('@emotion/css'); css({});"
)]
#[case("a.ts", "import E = require('@emotion/css'); E.css({ padding: 8 });")]
#[case("a.jsx", "const E = require('@emotion/css'); export const a = E.merge('a b');")]
#[serial]
fn styling_through_a_runtime_loader_is_a_located_error(#[case] filename: &str, #[case] code: &str) {
    // Given: the styling API would stay an Emotion call at runtime
    // When
    let error = compile_file(filename, code).unwrap_err();

    // Then
    assert!(error.starts_with(&format!("{filename}:1:")), "{error}");
    assert!(error.contains("cannot use"), "{error}");
    assert!(error.contains("require(), import-equals or dynamic import"), "{error}");
}

#[rstest]
#[case("import('@emotion/css').then((m) => m.css({ padding: 8 }));", "import('@emotion/css')")]
#[case("const p = import('@emotion/css'); consume(p);", "import('@emotion/css')")]
#[case("export const p = (import('@emotion/css'));", "import('@emotion/css')")]
#[case("const E = await import('@emotion/css'); consume(E);", "E")]
#[serial]
fn escaping_dynamic_imports_are_located_errors(#[case] code: &str, #[case] shown: &str) {
    // Given
    // When
    let error = compile_file("a.js", code).unwrap_err();

    // Then
    assert!(error.starts_with("a.js:1:"), "{error}");
    assert!(error.contains(&format!("cannot use `{shown}")), "{error}");
}

#[rstest]
#[case("a.cjs", "const r = require; const E = r('@emotion/css'); E.flush(); console.log(E.cache);")]
#[case("a.js", "const E = await import('@emotion/css'); E.flush(); E?.sheet;")]
#[case("a.js", "const { flush, cache: { key } } = await import('@emotion/css'); flush(key);")]
#[case(
    "a.mjs",
    "import { createRequire } from 'module'; createRequire(import.meta.url)('@emotion/css').flush();"
)]
#[case("a.cjs", "module.require('@emotion/css').hydrate([]);")]
#[serial]
fn runtime_only_loader_use_is_kept_as_written(#[case] filename: &str, #[case] code: &str) {
    // Given
    // When
    let output = compile_file(filename, code).unwrap();

    // Then
    assert_eq!(output.code, code);
}

#[rstest]
#[case("a.cjs", "function f(require) { return require('@emotion/css').css({ padding: 8 }); }")]
#[case("a.cjs", "function g(module) { return module.require('@emotion/css').css({}); }")]
#[case("a.js", "const load = (n) => n; export const a = load('@emotion/css').css({});")]
#[case("a.js", "const require = (n) => n; export const a = require('@emotion/css').css({});")]
#[case(
    "a.mjs",
    "import { createRequire } from './own.js'; export const a = createRequire(1)('@emotion/css').css({});"
)]
#[case(
    "a.mjs",
    "import { createRequire } from 'module'; function f(createRequire) { return createRequire(1)('@emotion/css').css({}); }"
)]
#[case("a.js", "const name = other; export const a = import(name);")]
#[serial]
fn shadowed_or_foreign_loaders_are_not_rewritten(#[case] filename: &str, #[case] code: &str) {
    // Given: the spelling matches but no binding is the module system's
    // When
    let output = compile_file(filename, code).unwrap();

    // Then
    assert_eq!(output.code, code);
}

#[rstest]
#[case("a.js", "const E = await import('@emotion/css'); E.css({ padding: 8 });")]
#[case("a.cjs", "const E = module.require('@emotion/css'); E.css({ padding: 8 });")]
#[case("a.ts", "import E = require('@emotion/css'); E.css({ padding: 8 });")]
#[case("a.js", "import('@emotion/css').then((m) => m.css({}));")]
#[serial]
fn disabled_alias_leaves_acquisition_code_untouched(#[case] filename: &str, #[case] code: &str) {
    // Given
    reset_class_map();
    reset_file_map();

    // When
    let output = extract(filename, code, ExtractOption::default()).unwrap();

    // Then
    assert_eq!(output.code, code);
}

#[rstest]
#[case("a.js")]
#[case("a.jsx")]
#[case("a.mjs")]
#[case("a.ts")]
#[serial]
fn static_namespace_imports_compile_in_every_file_type(#[case] filename: &str) {
    // Given
    let code = "import * as E from '@emotion/css'; export const a = E.css({ padding: 8 });";

    // When
    let output = compile_file(filename, code).unwrap();

    // Then
    assert!(format!("{:?}", output.styles).contains("8px"), "{}", output.code);
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}

#[test]
#[serial]
fn unitless_values_from_boa_stay_bare_when_namespace_is_normalized() {
    // Given: `double(1)` is computed by Boa, so the value is rewritten after the
    // namespace pass, and `opacity` takes no unit
    let code = "import * as E from '@emotion/css'; function double(n) { return n * 2 } export const a = E.css({ opacity: double(1) });";

    // When
    let output = compile(code).unwrap();

    // Then
    let styles = format!("{:?}", output.styles);
    assert!(styles.contains("opacity"), "{styles}");
    assert!(!styles.contains("2px"), "{styles}");
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}
