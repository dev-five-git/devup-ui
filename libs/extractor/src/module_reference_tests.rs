use super::*;

#[rstest::rstest]
#[case("\n")]
#[case("\r")]
#[case("\r\n")]
#[case("\u{2028}")]
#[case("\u{2029}")]
fn locations_when_extensionless_typescript_uses_javascript_line_separators(
    #[case] separator: &str,
) {
    // Given
    let source = format!(
        "type Color = string;{separator}import {{{separator} color{separator}}} from './colors';{separator}const 이름 = '✓🦀'; require('./colors');"
    );
    // When
    let locations = original_module_reference_locations("theme", &source, "./colors");
    // Then
    assert_eq!(
        locations,
        vec![
            ModuleReferenceLocation { line: 4, column: 8 },
            ModuleReferenceLocation {
                line: 5,
                column: 26
            },
        ]
    );
}

#[test]
fn locations_when_unknown_extension_uses_typescript() {
    // Given
    let source = "type Color = string;\r\nexport { color } from './colors';";
    // When
    let locations = original_module_reference_locations("theme.custom", source, "./colors");
    // Then
    assert_eq!(
        locations,
        vec![ModuleReferenceLocation {
            line: 2,
            column: 23
        }]
    );
}

#[test]
fn locations_when_typescript_imports_and_reexports_span_lines() {
    // Given
    let source = "type Color = string;\nimport {\n color\n} from './colors';\nexport { color } from './colors';\nexport * from './colors';";
    // When
    let locations = original_module_reference_locations("theme.ts", source, "./colors");
    // Then
    assert_eq!(
        locations,
        vec![
            ModuleReferenceLocation { line: 4, column: 8 },
            ModuleReferenceLocation {
                line: 5,
                column: 23
            },
            ModuleReferenceLocation {
                line: 6,
                column: 15
            },
        ]
    );
}

#[test]
fn locations_when_only_global_literal_require_is_a_reference() {
    // Given
    let source = "// require('./colors')\nconst text = \"./colors\";\nfunction f(require) { require('./colors'); }\nconst a = require(\n './colors'\n);\nrequire(variable);\nobj.require('./colors');\nrequire('./other');";
    // When
    let locations = original_module_reference_locations("theme.cts", source, "./colors");
    // Then
    assert_eq!(
        locations,
        vec![ModuleReferenceLocation { line: 5, column: 2 }]
    );
}

#[test]
fn locations_when_references_are_absent_are_not_invented() {
    // Given
    let source = "const text = './colors';";
    // When
    let locations = original_module_reference_locations("theme.ts", source, "./colors");
    // Then
    assert_eq!(locations, vec![]);
}

#[test]
fn locations_when_duplicate_specifiers_and_unicode_are_present() {
    // Given
    let source = "const 이름 = '✓'; require('./colors');\nimport './colors';\nrequire('./colors');";
    // When
    let locations = original_module_reference_locations("theme.ts", source, "./colors");
    // Then
    assert_eq!(
        locations,
        vec![
            ModuleReferenceLocation {
                line: 1,
                column: 25
            },
            ModuleReferenceLocation { line: 2, column: 8 },
            ModuleReferenceLocation { line: 3, column: 9 },
        ]
    );
}
