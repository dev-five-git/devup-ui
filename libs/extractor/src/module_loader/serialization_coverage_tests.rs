use boa_engine::{Context, Source};
use rstest::rstest;
use rustc_hash::FxHashMap;

use super::value_to_code;

#[rstest]
#[case("new Map([['color', 'red']])")]
#[case("new Set(['red'])")]
#[case("new (class Color { constructor() { this.value = 'red'; } })()")]
#[case("[() => 'red']")]
fn serialization_declines_unsupported_exports_without_reporting_an_execution_error(
    #[case] script: &str,
) -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes(script))
        .map_err(|error| error.to_string())?;
    // When
    let code = value_to_code(&value, &mut context, &FxHashMap::default(), &mut Vec::new())
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(code, None);
    Ok(())
}

#[test]
fn serialization_propagates_a_thrown_getter_instead_of_declining_the_export() -> Result<(), String>
{
    // Given
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes(
            "({ get color() { throw new TypeError('getter failed'); } })",
        ))
        .map_err(|error| error.to_string())?;
    // When
    let error = value_to_code(&value, &mut context, &FxHashMap::default(), &mut Vec::new())
        .err()
        .ok_or("getter exception swallowed")?;
    // Then
    assert!(error.to_string().starts_with("TypeError: getter failed"));
    Ok(())
}
