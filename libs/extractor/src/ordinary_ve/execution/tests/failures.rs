use super::*;

#[rstest]
#[case(
    "missing",
    concat!("const ignored=window.document, ", "box: string=style({color:missing});")
)]
#[case(
    "size",
    concat!("const ignored=window.document, ", "box=style({margin:size});\r\nconst size=8;")
)]
#[serial]
fn locates_required_reads_when_declarators_are_disconnected_by_excluded_source(
    #[case] read: &str,
    #[case] body: &str,
) {
    // Given
    css::file_map::reset_file_map();
    let code =
        format!("import {{style}} from '@vanilla-extract/css';\r\nconst 한글='😀';\r\n{body}");
    let expected = crate::locate(
        "/located.ts",
        &code,
        code.find(read)
            .unwrap_or_else(|| panic!("fixture read missing")),
    );
    // When
    let result = run(written("/located.ts", &code), None);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("required read unexpectedly succeeded"));
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains(read), "{error}");
}

#[test]
#[serial]
fn retains_earlier_edit_layers_when_a_selected_read_fails() {
    // Given
    css::file_map::reset_file_map();
    let original = concat!(
        "const removed=window.document;\n",
        "import {style} from '@vanilla-extract/css';const ignored=window.name,box=style({color:missing});"
    );
    let removed = original
        .find("import")
        .unwrap_or_else(|| panic!("fixture import missing"));
    let edits = [(0, removed, 0)];
    let layers: &[&[crate::import_alias_visit::Edit]] = &[&edits];
    let stylesheet = Stylesheet {
        filename: "/edited.ts",
        code: &original[removed..],
        source: original,
        edits: layers,
    };
    let expected = crate::locate(
        "/edited.ts",
        original,
        original
            .find("missing")
            .unwrap_or_else(|| panic!("fixture read missing")),
    );
    // When
    let result = run(stylesheet, None);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("required read unexpectedly succeeded"));
    assert!(error.contains(&expected), "{error}");
}

#[rstest]
#[case("const box=(()=>{style({color:'red'});return ()=>1})();", "box")]
#[case(
    "const box=(()=>{style({color:'red'});return {get value(){return style({color:'blue'})}}})();",
    "box"
)]
#[case(
    "const box=(()=>{style({color:'red'});const value={};value.self=value;return value})();",
    "box"
)]
#[case(
    "const box=(()=>{const value=[];value.extra=style({color:'red'});return value})();",
    "box"
)]
#[serial]
fn fails_located_capture_when_a_native_root_returns_unrepresentable_data(
    #[case] body: &str,
    #[case] name: &str,
) {
    // Given
    css::file_map::reset_file_map();
    let code = format!("import {{style}} from '@vanilla-extract/css';\n{body}");
    let expected = crate::locate(
        "/capture.ts",
        &code,
        code.find(name)
            .unwrap_or_else(|| panic!("fixture binding missing")),
    );
    // When
    let result = run(written("/capture.ts", &code), None);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("unsupported capture unexpectedly succeeded"));
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains("cannot be captured exactly"), "{error}");
}

#[test]
#[serial]
fn diagnoses_an_executed_dynamic_namespace_branch_at_its_original_site() {
    // Given
    css::file_map::reset_file_map();
    let code = "import * as ve from '@vanilla-extract/css';const chosen=false;const name='style';const box=chosen?ve.style({color:'red'}):ve[name]({color:'blue'});";
    let expected = crate::locate(
        "/dynamic.ts",
        code,
        code.find("ve[name]")
            .unwrap_or_else(|| panic!("fixture namespace missing")),
    );
    // When
    let result = run(written("/dynamic.ts", code), None);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("dynamic namespace unexpectedly succeeded"));
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains("static API member"), "{error}");
}

#[test]
#[serial]
fn diagnoses_structural_api_escape_when_the_enabled_api_is_exported() {
    // Given
    css::file_map::reset_file_map();
    let code = "import {style} from '@vanilla-extract/css';export const escaped=style;const box=style({color:'red'});";
    let expected = crate::locate(
        "/escape.ts",
        code,
        code.find("=style;")
            .unwrap_or_else(|| panic!("fixture escape missing"))
            + 1,
    );
    // When
    let result = run(written("/escape.ts", code), None);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("structural escape unexpectedly succeeded"));
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains("escapes"), "{error}");
}

#[rstest]
#[case("tokens.color='blue';", "tokens.color")]
#[case("Object.assign(tokens,{color:'blue'});", "tokens,{color")]
#[case(
    "change(tokens);function change(value){value.color='blue'}",
    "tokens);"
)]
#[serial]
fn diagnoses_required_mutation_when_its_effect_is_outside_the_selected_slice(
    #[case] mutation: &str,
    #[case] site: &str,
) {
    // Given
    css::file_map::reset_file_map();
    let code = format!(
        "import {{style}} from '@vanilla-extract/css';const tokens={{color:'red'}};{mutation}const box=style(tokens);"
    );
    let expected = crate::locate(
        "/mutation.ts",
        &code,
        code.find(site)
            .unwrap_or_else(|| panic!("fixture mutation missing")),
    );
    // When
    let result = run(written("/mutation.ts", &code), None);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("excluded mutation unexpectedly succeeded"));
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains("may be changed"), "{error}");
}

#[test]
#[serial]
fn diagnoses_a_required_rebinding_when_an_excluded_statement_writes_it() {
    // Given
    css::file_map::reset_file_map();
    let code = "import {style} from '@vanilla-extract/css';let color='red';color='blue';const box=style({color});";
    let expected = crate::locate(
        "/write.ts",
        code,
        code.find("color='blue'")
            .unwrap_or_else(|| panic!("fixture write missing")),
    );
    // When
    let result = run(written("/write.ts", code), None);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("excluded write unexpectedly succeeded"));
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains("may be changed"), "{error}");
}
