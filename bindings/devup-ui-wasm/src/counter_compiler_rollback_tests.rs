use super::counter_compiler_tests::*;
use css::counter_names::NameAddress;
use extractor::extract_style::compiler_receipts::WitnessDisposition;
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
        let observed = with_counter_extract(reserved_input(), |_, batch| {
            batch.consume(|v| {
                let child = v
                    .associations
                    .invocations
                    .iter()
                    .find(|i| i.raw == "child.css.ts")
                    .unwrap_or_else(|| panic!("real child invocation"));
                let w = v
                    .witnesses
                    .iter()
                    .find(|w| {
                        w.invocation == child.id
                            && w.disposition == WitnessDisposition::ReservationOnly
                    })
                    .unwrap_or_else(|| panic!("real child reservation"));
                let NameAddress::Counter {
                    namespace,
                    legacy_key,
                    ..
                } = &w.produced.class().allocation.address
                else {
                    panic!("Counter reservation")
                };
                assert!(
                    v.witnesses
                        .iter()
                        .filter(|w| w.disposition == WitnessDisposition::Emission)
                        .all(|w| match &w.produced.class().allocation.address {
                            NameAddress::Counter {
                                namespace: n,
                                legacy_key: k,
                                ..
                            } => n != namespace || k != legacy_key,
                            NameAddress::Baseline { .. } => false,
                        })
                );
                Err::<(), _>(Box::new(w.produced.class().allocation.address.clone()))
            })
        });
        let observed = match observed {
            Err(CounterCompileError::Consumer(observed)) => observed,
            Err(CounterCompileError::Compile(e)) => panic!("{e}"),
            Ok(()) => panic!("observation must abort allocator mutations"),
        };
        let css::counter_names::NameAddress::Counter {
            namespace,
            legacy_key: key,
            slot,
        } = *observed
        else {
            panic!("real Counter reservation")
        };
        let before = authority();
        let restore = cache6_session::Restore::default();
        // When
        let result = after_finish(reserved_input(), || {
            css::class_map::with_class_map_mut(|maps| {
                let mut expected = maps.clone();
                let slots = maps
                    .get_mut(&namespace)
                    .unwrap_or_else(|| panic!("acquired namespace"));
                assert_eq!(slots.get(&key), Some(&slot));
                let next = slot
                    .checked_add(1)
                    .unwrap_or_else(|| panic!("slot overflow"));
                if remove {
                    slots.remove(&key);
                    expected
                        .get_mut(&namespace)
                        .unwrap_or_else(|| panic!("expected namespace"))
                        .remove(&key);
                } else {
                    *slots
                        .get_mut(&key)
                        .unwrap_or_else(|| panic!("acquired key")) = next;
                    *expected
                        .get_mut(&namespace)
                        .unwrap_or_else(|| panic!("expected namespace"))
                        .get_mut(&key)
                        .unwrap_or_else(|| panic!("expected key")) = next;
                }
                assert_eq!(*maps, expected);
            });
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
