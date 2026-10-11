use super::{ActiveExactAttempt, check_administration_allowed, with_admission};

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn typed_administration_rejects_when_exact_scope_is_active(#[case] nested: bool) {
    // Given
    let check = || {
        if nested {
            crate::exact_attempt::with_exclusive_attempt(|| {
                Ok::<_, ()>(check_administration_allowed())
            })
        } else {
            Ok(check_administration_allowed())
        }
    };
    // When
    let result = with_admission(|| crate::exact_attempt::with_exclusive_attempt(check));
    // Then
    assert_eq!(result, Ok(Err(ActiveExactAttempt)));
}

#[test]
#[serial_test::serial]
fn typed_administration_accepts_when_exact_scope_has_finished() {
    // Given
    let finished = crate::exact_attempt::with_exclusive_attempt(|| Ok::<_, ()>(()));
    assert_eq!(finished, Ok(()));
    // When
    let result = with_admission(check_administration_allowed);
    // Then
    assert_eq!(result, Ok(()));
}

#[test]
fn active_exact_error_has_a_typed_diagnostic() {
    // Given
    let error: &dyn std::error::Error = &ActiveExactAttempt;
    // When
    let diagnostic = error.to_string();
    // Then
    assert_eq!(diagnostic, "an exact attempt is active");
}
