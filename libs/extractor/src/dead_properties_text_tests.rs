use crate::dead_properties_test_utils::{failure, position};
use crate::{ExtractOption, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(r"const c = css('color:red;\nbox-align:center');", "box-align")]
#[case(r"const c = css('content:\x61; box-align:center');", "box-align")]
#[case(r"const c = css('content:\u2192; box-align:center');", "box-align")]
#[case(r"const c = css('content:\u{2192}; box-align:center');", "box-align")]
#[case(
    r"const c = css('content:\uD83D\uDE00; box-align:center');",
    "box-align"
)]
#[case("const c = css('color:red;\\\n box-align:center');", "box-align")]
#[case("const c = css('color:red;\\\r\n box-align:center');", "box-align")]
#[case(
    r#"const c = css('content:"\\u2192"; box-align:center');"#,
    "box-align"
)]
#[case("const c = css`color:${value};\nbox-align:center`;", "box-align")]
#[case("const c = css`color:${'red'};\nbox-align:center`;", "box-align")]
#[case("const c = css`${'box-align'}:center`;", "box-align")]
#[case(
    "const c = css`color:red; /* comment */ box-align /* gap */:center`;",
    "box-align"
)]
#[case(
    "globalCss({ fontFaces: [`src:url(a); box-align:center`] });",
    "box-align"
)]
#[case("const c = css({ 'BOX-ALIGN': 'center' });", "'BOX-ALIGN'")]
#[serial]
fn text_mapping_keeps_authored_declaration_location(#[case] statement: &str, #[case] key: &str) {
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ css, globalCss }} from '@devup-ui/react';\n{statement}");
    let offset = position(&source, key);
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;

    let error = failure(extract("text.tsx", &source, ExtractOption::default()));

    assert!(
        error.starts_with(&format!("text.tsx:{line}:{column}:")),
        "{error}"
    );
}

#[rstest]
#[case("const c = cAlias({ boxOrient: 'vertical' });", "boxOrient")]
#[case("const c = cAlias`box-orient:vertical`;", "box-orient")]
#[case("const k = kAlias({ from: { boxOrient: 'vertical' } });", "boxOrient")]
#[case("gAlias`body { box-orient:vertical }`;", "box-orient")]
#[serial]
fn import_aliases_use_the_same_declaration_boundary(#[case] statement: &str, #[case] key: &str) {
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ css as cAlias, keyframes as kAlias }} from '@emotion/react'; import {{ globalCss as gAlias }} from '@devup-ui/react';\n{statement}"
    );
    let options = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };

    let error = failure(extract("alias.tsx", &source, options));

    assert!(
        error.starts_with(&format!("alias.tsx:2:{}:", position(statement, key) + 1)),
        "{error}"
    );
}

#[rstest]
#[case("const c = css`content:'box-align: center';--rules:{box-align:center};color:red`; ")]
#[case("const c = css`&[data-value=\"box-align: center\"] { color:red }`; ")]
#[case("const c = css`/* unterminated box-align:center`; ")]
#[serial]
fn scanner_ignores_balanced_custom_values_and_selector_strings(#[case] statement: &str) {
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ css }} from '@devup-ui/react';\n{statement}");

    let result = extract("controls.tsx", &source, ExtractOption::default());

    assert!(result.is_ok(), "{result:?}");
}

#[rstest]
#[case("keyframes({ from: { p: [[1, 2]] } });")]
#[case("globalCss({ body: { p: [[1, 2]] } });")]
#[serial]
fn unreadable_arrays_keep_the_stored_location(#[case] statement: &str) {
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ keyframes, globalCss }} from '@devup-ui/react';\n{statement}");

    let error = failure(extract("arrays.tsx", &source, ExtractOption::default()));

    assert!(
        error.starts_with(&format!(
            "arrays.tsx:2:{}:",
            position(statement, "[1, 2]") + 1
        )),
        "{error}"
    );
}
