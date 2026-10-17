use super::value_to_code;
use boa_engine::{Context, Source};
use rstest::rstest;
use rustc_hash::FxHashMap;

#[rstest]
#[case(
    "({get first(){reads++;return ()=>1},get later(){throw new Error('must remain skipped')}})",
    1
)]
#[case(
    "[{get first(){reads++;return ()=>1}}, {get later(){throw new Error('must remain skipped')}}]",
    1
)]
#[case(
    "({first:()=>1,nested:{get later(){throw new Error('must remain skipped')}}})",
    0
)]
#[case(
    "({get first(){reads++;return 1},get next(){reads++;return ()=>1},last:{value:1}})",
    2
)]
fn gate_boundaries_export_fallback_preserves_getter_short_circuit(
    #[case] expression: &str,
    #[case] expected_reads: u32,
) -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes(&format!("var reads=0;{expression}")))
        .map_err(|error| error.to_string())?;
    // When
    let code = value_to_code(&value, &mut context, &FxHashMap::default(), &mut Vec::new())
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(code, None);
    let reads = context
        .eval(Source::from_bytes("reads"))
        .map_err(|error| error.to_string())?;
    assert_eq!(reads.as_number(), Some(f64::from(expected_reads)));
    Ok(())
}
