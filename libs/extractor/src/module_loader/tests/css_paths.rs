use crate::module_loader::retained_css::{is_generated, relative_specifier};

#[rstest::rstest]
#[case("/src/card.css.ts", "/src/styles.css", "./styles.css")]
#[case(
    "/src/pages/card.css.ts",
    "/styles/global.css",
    "../../styles/global.css"
)]
#[case(
    "C:\\app\\src\\card.css.ts",
    "c:/app/src/../styles/global.css?inline",
    "../styles/global.css"
)]
#[case(
    "src/pages/card.css.ts",
    "src/styles/global.css",
    "../styles/global.css"
)]
#[case("card.css.ts", "global.css", "./global.css")]
fn css_retention_relativizes_when_paths_have_compatible_roots(
    #[case] root: &str,
    #[case] target: &str,
    #[case] expected: &str,
) -> Result<(), crate::module_loader::retained_css::PathError> {
    // Given / When
    let relative = relative_specifier(root, target)?;
    // Then
    assert_eq!(relative, expected);
    assert!(relative.starts_with("./") || relative.starts_with("../"));
    assert!(!relative.contains('\\'));
    Ok(())
}

#[rstest::rstest]
#[case("C:/app/card.css.ts", "D:/styles/global.css")]
#[case("src/card.css.ts", "/styles/global.css")]
fn css_retention_rejects_when_paths_have_incompatible_roots(
    #[case] root: &str,
    #[case] target: &str,
) -> Result<(), String> {
    // Given / When
    let error = relative_specifier(root, target)
        .err()
        .ok_or("expected incompatible roots")?;
    // Then
    assert!(error.describe().contains(root) && error.describe().contains(target));
    assert!(error.describe().contains("Fix:"));
    Ok(())
}

#[rstest::rstest]
#[case("df/styles/devup-ui.css", true)]
#[case("df/styles/devup-ui-17.css?inline", true)]
#[case("df/styles/my.module.css", false)]
#[case("df/styles/sub/user.css", false)]
#[case("df/styles-other/devup-ui.css", false)]
fn css_retention_exempts_only_generated_loads_when_css_directory_contains_user_files(
    #[case] specifier: &str,
    #[case] expected: bool,
) {
    // Given
    let option = crate::ExtractOption {
        css_dir: "df/styles".to_string(),
        ..crate::ExtractOption::default()
    };
    // When
    let generated = is_generated(specifier, &option);
    // Then
    assert_eq!(generated, expected);
}
