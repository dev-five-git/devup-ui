use super::counter_compiler_tests::*;
use sheet::counter_kernel::KernelError;

#[test]
#[serial]
fn consumer_error_is_transported_when_genuine_prior_records_are_missing() {
    with_admission(|| {
        // Given: retain genuine compiler authority but remove its materialized records.
        let _guard = Guard::new();
        required(code_extract_counter(reserved_input(), false));
        with_style_sheet_mut(|s| s.properties.clear());
        let before = authority();
        // When: real compilation precedes checked publication of the invalid BASE.
        let error = code_extract_counter(reserved_input(), false)
            .err()
            .unwrap_or_else(|| panic!("missing prior records must fail"));
        // Then: preserve the actual kernel category and complete pre-attempt state.
        assert_eq!(error, "dormant counter kernel: Coverage");
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &Default::default()));
    });
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn final_rejects_when_only_reservation_key_or_slot_is_damaged_after_finish(#[case] remove: bool) {
    with_admission(|| {
        // Given: independent genuine parser observation aborts without granting batch authority.
        let _guard = Guard::new();
        let address = reservation();
        let before = authority();
        let restore = cache6_session::Restore::default();
        // When
        let result = after_finish(reserved_input(), || {
            damage_reservation(&address, remove);
            Ok::<_, String>(())
        });
        // Then
        assert!(matches!(
            result,
            Err(CounterCompileError::Consumer(UpdateError::Kernel(
                KernelError::Authority
            )))
        ));
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|state| assert_eq!(state, &restore));
        let retry = code_extract_counter(reserved_input(), false).unwrap_or_else(|e| panic!("{e}"));
        let retry = ObservedOutput::capture(&retry, "parent.css.ts");
        let retry_maps = cache6_session::Maps::capture();
        fresh();
        let clean = code_extract_counter(reserved_input(), false).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(ObservedOutput::capture(&clean, "parent.css.ts"), retry);
        assert_eq!(cache6_session::Maps::capture(), retry_maps);
        with_style_sheet(|sheet| {
            assert!(!sheet.create_css(None, false).contains("magenta"));
            assert!(
                !sheet
                    .create_css(Some("parent.css.ts"), false)
                    .contains("magenta")
            );
        });
    });
}
