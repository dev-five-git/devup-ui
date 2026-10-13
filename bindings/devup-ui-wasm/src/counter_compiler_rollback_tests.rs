use super::counter_compiler_tests::*;
use css::counter_names::NameAddress;
use extractor::extract_style::compiler_receipts::WitnessDisposition;
use sheet::counter_kernel::KernelError;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[rstest]
#[serial]
fn installed_session_is_restored_when_real_resolver_reentry_then_outer_fails(
    #[values(false, true)] shadow: bool,
    #[values(false, true)] inner_error: bool,
    #[values(false, true)] unwind: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        configure_typography();
        compile(
            "cached.css.ts",
            "import {css} from '@devup-ui/react'; const a=css({color:'green'});",
        );
        let bytes = encoded();
        fresh();
        configure_typography();
        let (maps, sheet) = cache6_session::Restore::before();
        assert_eq!(cache6_restore::import(&bytes), Ok(()));
        let restore = std::rc::Rc::new(cache6_session::Restore {
            session: Some(cache6_session::Session::installed(maps, sheet)),
            ..Default::default()
        });
        let before = authority();
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let observed_calls = calls.clone();
        let expected_restore = restore.clone();
        let resolver = move |_: &str, _: &str| {
            calls.set(calls.get() + 1);
            let depth_one = authority();
            let nested_source = "import {Box} from '@devup-ui/react'; const a=<Box color='blue'/>;";
            if inner_error {
                let nested = after_finish(input("nested.tsx", nested_source), || Err("depth two"));
                assert!(matches!(
                    nested,
                    Err(CounterCompileError::Consumer(UpdateError::Output(
                        "depth two"
                    )))
                ));
                assert_eq!(authority(), depth_one);
            } else if shadow {
                let shadow = required(extractor::extract(
                    "shadow.tsx",
                    nested_source,
                    ExtractOption::default(),
                ));
                assert!(shadow.code.contains("className"));
                assert_eq!(css::file_map::original_id("shadow.tsx"), None);
            } else {
                compile("nested.tsx", nested_source);
            }
            cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, restore.as_ref()));
            Some(ResolvedModule {
                path: "values.ts".into(),
                code: "export const tone='red';".into(),
            })
        };
        let source = "import { tone } from './values'; import { css } from '@devup-ui/react'; const a=css({color:tone});";
        let mut request = input("outer.css.ts", source);
        request.resolver = Some(&resolver);
        // When
        let result = catch_unwind(AssertUnwindSafe(|| {
            after_finish(request, || {
                assert!(!unwind, "outer unwind");
                Err("outer error")
            })
        }));
        // Then
        match result {
            Ok(result) => assert!(matches!(
                result,
                Err(CounterCompileError::Consumer(UpdateError::Output(
                    "outer error"
                )))
            )),
            Err(payload) => assert_eq!(payload.downcast_ref::<&str>(), Some(&"outer unwind")),
        }
        assert_ne!(observed_calls.get(), 0);
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, expected_restore.as_ref()));
        let retry = compile(
            "retry.css.ts",
            "import {css} from '@devup-ui/react';const a=css({color:'purple'});",
        );
        let retry_maps = cache6_session::Maps::capture();
        fresh();
        configure_typography();
        compile(
            "cached.css.ts",
            "import {css} from '@devup-ui/react'; const a=css({color:'green'});",
        );
        assert_eq!(
            compile(
                "retry.css.ts",
                "import {css} from '@devup-ui/react';const a=css({color:'purple'});"
            ),
            retry
        );
        assert_eq!(cache6_session::Maps::capture(), retry_maps);
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
