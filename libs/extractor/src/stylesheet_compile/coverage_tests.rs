use crate::{ExtractOption, vanilla_extract::Stylesheet};
use serial_test::serial;

#[test]
fn malformed_authored_source_when_compiling_is_left_for_the_original_parser() -> Result<(), String>
{
    let source = "import {css} from '@devup-ui/react'; export const box=css({ color: ; });";
    let stylesheet = Stylesheet {
        filename: "/broken.css.ts",
        code: source,
        source,
        edits: &[],
    };
    let compiled = super::compile(stylesheet, &ExtractOption::default(), None)?;
    assert_eq!(compiled.code, None);
    assert_eq!(compiled.edits, vec![]);
    assert_eq!(compiled.styles.into_iter().collect::<Vec<_>>(), vec![]);
    Ok(())
}

#[test]
#[serial]
fn unrecorded_macro_when_inside_an_uninvoked_throw_keeps_original_compilation() -> Result<(), String>
{
    // Given
    const SOURCE: &str = "import {css} from '@devup-ui/react';function unused(){throw css({p:2});}";
    let stylesheet = Stylesheet {
        filename: "/unused-throw.css.ts",
        code: SOURCE,
        source: SOURCE,
        edits: &[],
    };
    // When
    let compiled = super::compile(stylesheet, &ExtractOption::default(), None)?;
    // Then
    assert_eq!(compiled.code, None);
    assert_eq!(compiled.edits, vec![]);
    Ok(())
}

#[test]
#[serial]
fn uninvoked_styled_factory_when_precompiled_still_tracks_generated_import_provenance()
-> Result<(), String> {
    // Given
    const SOURCE: &str =
        "import {styled} from '@devup-ui/react';function unused(){return styled('section',{p:2});}";
    let stylesheet = Stylesheet {
        filename: "/unused-styled.css.ts",
        code: SOURCE,
        source: SOURCE,
        edits: &[],
    };
    // When
    let compiled = super::compile(stylesheet, &ExtractOption::default(), None)?;
    // Then
    assert!(compiled.evaluation.is_some());
    assert!(
        compiled
            .scaffolding
            .iter()
            .any(|import| import.contains("forwardRef"))
    );
    assert!(compiled.code.as_ref().is_some_and(
        |code| code.contains("__devupForwardRef") && code.contains("__devupCreateElement")
    ));
    Ok(())
}

#[test]
#[serial]
fn lexical_style_read_when_precompile_inlining_rejects_it_reports_original_location()
-> Result<(), String> {
    // Given
    let source =
        "import {css} from '@devup-ui/react';\nconst base=css({color});\nconst color='sienna';";
    let stylesheet = Stylesheet {
        filename: "/macro-tdz.css.ts",
        code: source,
        source,
        edits: &[],
    };
    // When
    let error = super::compile(stylesheet, &ExtractOption::default(), None)
        .err()
        .ok_or("TDZ style read was accepted")?;
    // Then
    assert!(error.starts_with("/macro-tdz.css.ts:2:17:"), "{error}");
    assert!(
        error.contains("`color`") && error.contains("ReferenceError"),
        "{error}"
    );
    Ok(())
}
