use css::resident_owner::OwnerFault;
use rstest::rstest;
use serial_test::serial;

use super::{checkpoint::HostCheckpoint, error_native_tests::Value, owner_native_tests::*};

#[test]
#[serial]
fn rich_checkpoint_restores_every_nested_css_record_and_owned_family()
-> Result<(), Box<dyn std::error::Error>> {
    // Given a parsed rich checkpoint, independent of the later engine state.
    let _fixture = Fixture::new()?;
    let expected = crate::owner_codec_remaining_tests::install_owner("original")?;
    let checkpoint = HostCheckpoint::parse(expected.each_ref().map(String::as_str))?;
    crate::owner_codec_remaining_tests::install_owner("replacement")?;
    // When the actual production restore runs.
    checkpoint.restore();
    // Then CSS records, atom plan, allocators, inventory and reservation counters match.
    assert_eq!(parts()?, expected);
    Ok(())
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[serial]
fn invalid_checkpoint_family_never_publishes_partial_state(
    #[case] index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given all five valid exported families and a later invalid family.
    let _fixture = Fixture::new()?;
    let expected = crate::owner_codec_remaining_tests::install_owner("original")?;
    let mut wire = expected.clone();
    wire[index] = "invalid".into();
    // When production parses the complete checkpoint.
    let result = HostCheckpoint::parse(wire.each_ref().map(String::as_str));
    // Then it rejects without installing any earlier parsed family.
    assert!(result.is_err());
    assert_eq!(parts()?, expected);
    Ok(())
}

#[rstest]
#[case("sheet", 1)]
#[case("classes", 2)]
#[case("files", 3)]
#[case("canonical", 4)]
#[case("reservation", 5)]
#[serial]
fn checkpoint_decoder_keeps_exact_getter_failure_and_order(
    #[case] field: &str,
    #[case] reads: usize,
) -> Result<(), OwnerFault> {
    // Given a checkpoint object whose selected getter throws an exact reference.
    let _fixture = Fixture::new()?;
    let wire = ok(snapshot());
    let thrown = Value::new();
    FIELD_FAILURE.with_borrow_mut(|failure| *failure = Some((field.into(), thrown.clone())));
    // When the same production decoder used by capture reads it.
    let result = OWNER.decode(wire);
    // Then ordered reads stop at that field and retain the exact thrown reference.
    assert!(matches!(result, Err(actual) if actual == thrown));
    assert_eq!(EVENTS.with_borrow(Vec::len), reads);
    assert_eq!(
        EVENTS.with_borrow(|events| events.last().cloned()),
        Some(field.into())
    );
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn checkpoint_decoder_distinguishes_nonstring_fields_from_invalid_json(
    #[case] malformed_json: bool,
) -> Result<(), OwnerFault> {
    // Given an invalid sheet representation.
    let _fixture = Fixture::new()?;
    let wire = ok(snapshot());
    wire.set_property(
        "sheet",
        if malformed_json {
            Value::text("invalid")
        } else {
            Value::Undefined
        },
    );
    // When the production decoder validates and parses it.
    let Err(error) = OWNER.decode(wire) else {
        panic!("invalid checkpoint accepted")
    };
    // Then type validation precedes parsing and no partially decoded state is installed.
    if malformed_json {
        assert_eq!(EVENTS.with_borrow(Vec::len), 5);
        assert_ne!(
            error.get("message"),
            Value::text("checkpoint fields must be strings")
        );
    } else {
        assert_eq!(EVENTS.with_borrow(Vec::len), 1);
        assert_eq!(error.get("name"), Value::text("TypeError"));
        assert_eq!(
            error.get("message"),
            Value::text("checkpoint fields must be strings")
        );
    }
    Ok(())
}
