use super::mixed_canonical::assert_preserved;
use super::mixed_review::emitted_predicate;
use super::mixed_support::{assert_consumed, extract};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("const before=input;", "before.value===1")]
#[case(
    "const alias=input;const before=alias;",
    "before===alias && before.value===1"
)]
#[serial]
fn readonly_alias_keeps_old_identity_when_a_selected_root_rebinds_its_source(
    #[case] aliases: &str,
    #[case] original: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';let input={{value:1}};{aliases}export const result=(()=>{{input={{value:2}};style({{}});return input}})();const browser=window.document;"
    );
    // When
    let output = extract("ts", &source)?;
    // Then
    assert_consumed(&output);
    assert_preserved(&output.code, aliases);
    assert!(
        emitted_predicate(
            &output.code,
            &format!("{original} && before!==input && result===input && input.value===2")
        ),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case("const alias=input;alias.value=3;", "alias.value=")]
#[case("const alias=input;const next=alias;next.value=3;", "next.value=")]
#[case("const alias=input;change(alias);", "alias);")]
#[serial]
fn mutable_alias_is_located_when_its_effect_is_outside_selected_initialization(
    #[case] observation: &str,
    #[case] site: &str,
) {
    // Given
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';const input={{value:1}};{observation}export const result=(()=>{{style({{}});return input}})();const browser=window.document;"
    );
    let offset = source
        .find(site)
        .unwrap_or_else(|| panic!("fixture mutation site missing"));
    let expected = crate::locate("/mixed.ts", &source, offset);
    // When
    let result = extract("ts", &source);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("mutable alias was incorrectly allowed"))
        .to_string();
    assert!(error.starts_with(&expected), "{error}");
    assert!(error.contains("may be changed"), "{error}");
}
