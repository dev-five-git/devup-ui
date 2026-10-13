pub(super) use super::cache6_test_support::*;
pub(super) use super::counter_compiler::{after_finish, code_extract_counter};
use css::counter_names::NameAddress;
pub(super) use extractor::compiler_policy::{
    CompilerInput, CounterCompileError, with_counter_extract,
};
use extractor::extract_style::compiler_receipts::WitnessDisposition;
pub(super) use std::panic::{AssertUnwindSafe, catch_unwind};

#[cfg(test)]
mod cold;
#[cfg(test)]
mod dynamic;
#[cfg(test)]
mod nested;
#[cfg(test)]
mod surfaces;

pub(super) fn required<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("{error:?}"))
}

pub(super) fn input<'a>(file: &'a str, code: &'a str) -> CompilerInput<'a> {
    CompilerInput {
        filename: file,
        code,
        option: ExtractOption::default(),
        source_map: true,
        resolver: None,
    }
}
pub(super) fn compile(file: &str, code: &str) -> ObservedOutput {
    let output = code_extract_counter(input(file, code), false).unwrap_or_else(|e| panic!("{e}"));
    ObservedOutput::capture(&output, file)
}
pub(super) const PARENT: &str = "import {tone} from './child.css'; import {style} from '@devup-ui/react'; export const root=style({color:tone});";
pub(super) const RESERVED_CHILD: &ModuleResolver = &|_, _| {
    Some(ResolvedModule { path: "child.css.ts".into(), code: "import {style} from '@devup-ui/react'; export const tone='red'; export const unused=style({color:'magenta'});".into() })
};
pub(super) fn reserved_input() -> CompilerInput<'static> {
    let mut source = input("parent.css.ts", PARENT);
    source.resolver = Some(RESERVED_CHILD);
    source
}

pub(super) fn exported() -> (Vec<u8>, cache6_session::Maps, LiveCheckpoint) {
    let bytes = encoded();
    let maps = cache6_session::Maps::capture();
    with_style_sheet_mut(|s| s.source_ids = css::file_map::get_original_ids());
    (bytes, maps, with_style_sheet(LiveCheckpoint::capture))
}

pub(super) fn install(
    proof: &(Vec<u8>, cache6_session::Maps, LiveCheckpoint),
) -> cache6_session::Restore {
    let (maps, sheet) = cache6_session::Restore::before();
    assert_eq!(cache6_restore::import(&proof.0), Ok(()));
    assert_eq!(cache6_session::Maps::capture(), proof.1);
    assert_eq!(with_style_sheet(LiveCheckpoint::capture), proof.2);
    let restore = cache6_session::Restore {
        session: Some(cache6_session::Session::installed(maps, sheet)),
        ..Default::default()
    };
    cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &restore));
    restore
}

pub(super) fn reservation() -> NameAddress {
    let observed = with_counter_extract(reserved_input(), |_, batch| {
        batch.consume(|v| {
            let child = v
                .associations
                .invocations
                .iter()
                .find(|i| i.raw == "child.css.ts")
                .unwrap_or_else(|| panic!("child invocation"));
            let witness = v
                .witnesses
                .iter()
                .find(|w| {
                    w.invocation == child.id && w.disposition == WitnessDisposition::ReservationOnly
                })
                .unwrap_or_else(|| panic!("reservation"));
            let address = &witness.produced.class().allocation.address;
            assert!(
                v.witnesses
                    .iter()
                    .filter(|w| w.disposition == WitnessDisposition::Emission)
                    .all(
                        |w| match (address, &w.produced.class().allocation.address) {
                            (
                                NameAddress::Counter {
                                    namespace,
                                    legacy_key,
                                    ..
                                },
                                NameAddress::Counter {
                                    namespace: other,
                                    legacy_key: key,
                                    ..
                                },
                            ) => namespace != other || legacy_key != key,
                            _ => false,
                        }
                    )
            );
            Err::<(), _>(Box::new(address.clone()))
        })
    });
    match observed {
        Err(CounterCompileError::Consumer(address)) => *address,
        Err(CounterCompileError::Compile(error)) => panic!("{error}"),
        Ok(()) => panic!("probe must abort"),
    }
}

pub(super) fn damage_reservation(address: &NameAddress, remove: bool) {
    match address {
        NameAddress::Counter {
            namespace,
            legacy_key,
            slot,
        } => {
            css::class_map::with_class_map_mut(|maps| {
                let mut expected = maps.clone();
                for map in [&mut *maps, &mut expected] {
                    let slots = map
                        .get_mut(namespace)
                        .unwrap_or_else(|| panic!("namespace"));
                    assert_eq!(slots.get(legacy_key), Some(slot));
                    if remove {
                        slots.remove(legacy_key);
                    } else {
                        slots.insert(
                            legacy_key.clone(),
                            slot.checked_add(1).unwrap_or_else(|| panic!("overflow")),
                        );
                    }
                }
                assert_eq!(*maps, expected);
            });
        }
        NameAddress::Baseline { .. } => panic!("Counter address"),
    }
}
