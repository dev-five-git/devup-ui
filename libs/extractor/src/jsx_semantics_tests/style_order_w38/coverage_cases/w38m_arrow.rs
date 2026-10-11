use super::*;

#[test]
#[serial]
fn keyframes_when_expression_arrow_contains_metadata_reports_original_key() {
    for key in ["styleOrder", "'style-order'"] {
        // Given
        let source = format!(
            "import {{keyframes}} from '@devup-ui/react';\nconst a=keyframes(() => ({{from:{{{key}:2,opacity:0}}}}));"
        );
        let offset = source
            .find(key)
            .required("fixture contains its original key");
        let (_, diagnostic) = crate::style_order::no_effect("keyframes", 0);
        let expected = format!("{}: {diagnostic}", crate::locate("a.tsx", &source, offset));
        // When
        let actual = error(&source);
        // Then
        assert_eq!(
            actual.lines().filter(|line| *line == expected).count(),
            1,
            "{actual}"
        );
    }
}

#[test]
#[serial]
fn keyframes_when_block_arrow_contains_metadata_remains_unsupported() {
    // Given
    let source = concat!(
        "import {keyframes} from '@devup-ui/react';\n",
        "const a=keyframes(() => {return {from:{styleOrder:2,opacity:0}};});"
    );
    // When
    let actual = error(source);
    // Then
    assert!(
        actual.starts_with("a.tsx:2:9: `keyframes()` cannot use `"),
        "{actual}"
    );
    assert!(
        actual.contains(
            "its values must be literals, theme tokens or constants, or be computed from them"
        ),
        "{actual}"
    );
    assert!(!actual.contains("has no effect"), "{actual}");
}

#[test]
#[serial]
fn keyframes_when_expression_arrow_has_only_declarations_remains_unsupported() {
    // Given
    let source = concat!(
        "import {keyframes} from '@devup-ui/react';\n",
        "const a=keyframes(() => ({from:{opacity:0}}));"
    );
    // When
    let actual = error(source);
    // Then
    assert!(
        actual.starts_with("a.tsx:2:9: `keyframes()` cannot use `"),
        "{actual}"
    );
    assert!(
        actual.contains(
            "its values must be literals, theme tokens or constants, or be computed from them"
        ),
        "{actual}"
    );
    assert!(!actual.contains("has no effect"), "{actual}");
}
