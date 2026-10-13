use crate::{ExtractOption, extract};

#[rstest::rstest]
#[case("computed.tsx")]
#[case("computed.css.ts")]
#[serial_test::serial]
fn computed_style_guard_error_names_original_read_location(
    #[case] filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let code = "import { css } from '@devup-ui/react';\nconst compare = Object.getOwnPropertyDescriptor(String.prototype, 'localeCompare').value;\nexport const a = css({ w: compare('a', 'b') });";
    // When
    let error = extract(filename, code, ExtractOption::default())
        .err()
        .ok_or("forbidden style unexpectedly extracted")?;
    // Then
    let error = error.to_string();
    assert!(error.starts_with(&format!("{filename}:2:17:")), "{error}");
    assert!(error.contains("String.prototype.localeCompare"), "{error}");
    assert!(
        error.contains("Fix: use a literal or a CSS variable"),
        "{error}"
    );
    Ok(())
}
