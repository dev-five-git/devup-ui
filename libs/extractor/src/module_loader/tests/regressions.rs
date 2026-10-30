use serial_test::serial;

use super::{error_of, parts};

#[rstest::rstest]
#[case("globalStyle('body', { get width() { throw new Error('serialize'); } })")]
#[case("globalFontFace('font', [{ get src() { throw new Error('serialize'); } }])")]
#[case("styleVariants({ a: { toJSON() { throw new Error('serialize'); } } })")]
#[case("style({ toJSON() { throw new Error('serialize'); } })")]
#[case("keyframes({ toJSON() { throw new Error('serialize'); } })")]
#[case("fontFace({ toJSON() { throw new Error('serialize'); } })")]
#[case("({ get width() { throw new Error('serialize'); } })")]
#[case("[1, { get width() { throw new Error('serialize'); } }]")]
#[serial]
fn serialization_exceptions_keep_the_original_throw_location(
    #[case] expression: &str,
) -> Result<(), String> {
    let source = format!(
        "import {{ style, globalStyle, globalFontFace, styleVariants, keyframes, fontFace }} from '@devup-ui/react';\nexport const result = {expression};\n"
    );
    let expected = crate::locate(
        "/a.css.ts",
        &source,
        source.find("new Error").ok_or("missing throw fixture")?,
    );

    let error = error_of("/a.css.ts", &source, &[]);

    assert!(
        error.starts_with(&format!(
            "{expected}: JS execution error: Error: serialize. Fix: "
        )),
        "{error}"
    );
    Ok(())
}

#[test]
#[serial]
fn fatal_entry_syntax_is_rejected_before_stripping() {
    let source = "import { style } from '@devup-ui/react';\nexport const broken = ;\n";

    let error = error_of("/a.css.ts", source, &[]);

    assert!(
        error.starts_with("/a.css.ts:2:23: JS execution error: SyntaxError:"),
        "{error}"
    );
    assert!(error.contains("Fix: correct the syntax"), "{error}");
}

#[rstest::rstest]
#[case(&[("/b.ts", "export const broken = ;\n")], "./b")]
#[case(&[("/b.css.ts", "export const broken = ;\n")], "./b.css")]
#[serial]
fn fatal_import_syntax_is_located_in_the_imported_file(
    #[case] files: super::Files,
    #[case] specifier: &str,
) {
    let filename = files[0].0;
    let entry = format!("import {{ broken }} from '{specifier}';\nexport const x = broken;\n");

    let error = error_of("/a.css.ts", &entry, files);

    assert!(
        error.starts_with(&format!(
            "{filename}:1:23: JS execution error: SyntaxError:"
        )),
        "{error}"
    );
}

#[test]
#[serial]
fn alias_edits_are_applied_to_parser_diagnostic_labels() -> Result<(), String> {
    use crate::import_alias_visit::transform_import_aliases_with_edits;
    use crate::vanilla_extract::{Stylesheet, execute_located};
    let source = "import { style } from '@vanilla-extract/css'; export const broken = ;";
    let aliases = std::collections::HashMap::from([(
        "@vanilla-extract/css".to_string(),
        crate::ImportAlias::NamedToNamed,
    )]);
    let aliased =
        transform_import_aliases_with_edits(source, "/a.css.ts", super::PACKAGE, &aliases);
    let expected = crate::locate(
        "/a.css.ts",
        source,
        source.rfind(';').ok_or("missing semicolon fixture")?,
    );

    let error = execute_located(
        Stylesheet {
            filename: "/a.css.ts",
            code: &aliased.code,
            source,
            edits: &[&aliased.edits],
        },
        &crate::ExtractOption::default(),
        None,
    )
    .err()
    .ok_or("expected invalid syntax to fail")?;

    assert!(
        error.starts_with(&format!("{expected}: JS execution error: SyntaxError:")),
        "{error}"
    );
    Ok(())
}

#[test]
#[serial]
fn typed_retained_nested_functions_keep_original_coordinates() -> Result<(), String> {
    const IMPORTED: &str = "import { style } from '@devup-ui/react';\nexport const s = style({});\nexport const invoke = (): string => {\n  const inner = (): string => { throw new Error('nested retained'); };\n  return inner();\n};";
    let source = "import { invoke } from './b.css';\nexport const x = invoke();";
    let imported = IMPORTED;
    let expected = crate::locate(
        "/b.css.ts",
        imported,
        imported.find("new Error").ok_or("missing throw fixture")?,
    );

    let error = error_of("/a.css.ts", source, &[("/b.css.ts", IMPORTED)]);

    let (lead, _, calls) = parts(&error);
    assert_eq!(
        lead,
        format!("{expected}: JS execution error: Error: nested retained")
    );
    assert!(
        calls.starts_with(&format!("    at inner ({expected})")),
        "{error}"
    );
    assert!(calls.contains("at invoke (/b.css.ts:5:"), "{error}");
    assert!(calls.ends_with("at <main> (/a.css.ts:2:24)"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn retained_function_frames_survive_style_extraction_inside_the_function() -> Result<(), String> {
    const IMPORTED: &str = "import { style, css } from '@devup-ui/react';\nexport const s = style({});\nexport const fail = () => { const c = css({ color: 'red' }); throw new Error('compiled retained'); };";
    let expected = crate::locate(
        "/b.css.ts",
        IMPORTED,
        IMPORTED.find("new Error").ok_or("missing throw fixture")?,
    );

    let error = error_of(
        "/a.css.ts",
        "import { fail } from './b.css';\nexport const x = fail();",
        &[("/b.css.ts", IMPORTED)],
    );

    assert!(
        error.starts_with(&format!(
            "{expected}: JS execution error: Error: compiled retained. Fix:"
        )),
        "{error}"
    );
    Ok(())
}

#[rstest::rstest]
#[case("style")]
#[case("fontFace")]
#[case("keyframes")]
#[serial]
fn cyclic_json_values_report_the_serialization_exception(#[case] api: &str) -> Result<(), String> {
    let source = format!(
        "import {{ {api} }} from '@devup-ui/react';\nconst o = {{}}; o.self = o;\nexport const result = {api}(o);"
    );
    let expected = crate::locate(
        "/a.css.ts",
        &source,
        source.find("(o)").ok_or("missing call fixture")?,
    );

    let error = error_of("/a.css.ts", &source, &[]);

    assert!(
        error.starts_with(&format!(
            "{expected}: JS execution error: TypeError: cyclic object value. Fix:"
        )),
        "{error}"
    );
    Ok(())
}
