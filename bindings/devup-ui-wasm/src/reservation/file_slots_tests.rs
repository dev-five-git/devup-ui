use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::tests::{TestResult, input, setup, transform};
use super::*;
use crate::{export_class_map_internal, export_file_map_internal, seed_file_map, with_style_sheet};
use serial_test::serial;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const VARIABLE_CONTAINER: &str = "import {createVar,createContainer} from '@devup-ui/react'; export const variable=createVar(); export const container=createContainer();";
const VARIABLE_LAYER: &str = "import {createVar,layer} from '@devup-ui/react'; export const variable=createVar(); export const namedLayer=layer();";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> io::Result<Self> {
        loop {
            let path = std::env::temp_dir().join(format!(
                "devup-raw-slots-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
    }

    fn sources(&self) -> io::Result<Vec<ReservationInput>> {
        [
            ("A.css.ts", VARIABLE_CONTAINER),
            ("B.css.ts", VARIABLE_LAYER),
        ]
        .into_iter()
        .map(|(name, code)| {
            let path = self.0.join(name);
            fs::write(&path, code)?;
            Ok(input(&path.to_string_lossy(), &fs::read_to_string(&path)?))
        })
        .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let removed = fs::remove_dir_all(&self.0);
        assert!(removed.is_ok(), "fixture cleanup failed: {removed:?}");
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Built {
    sources: BTreeMap<String, (String, Option<String>)>,
    css: Vec<String>,
    files: String,
}

fn build(
    inputs: &[ReservationInput],
    order: [usize; 2],
) -> Result<Built, Box<dyn std::error::Error>> {
    let (_reset, option) = setup(false);
    file_map::set_canonical_map(
        inputs
            .iter()
            .map(|input| (input.filename.clone(), "bucket.tsx".into()))
            .collect(),
    );
    seed_file_map(inputs.iter().map(|input| input.filename.clone()).collect());
    reserve_class_slots_internal(inputs, option.clone(), None);
    let mut sources = BTreeMap::new();
    for index in order {
        let output = transform(&inputs[index], &option)?;
        assert!(output.code.contains("var(--"));
        assert!(!output.code.contains("createVar("));
        sources.insert(inputs[index].filename.clone(), (output.code, output.map));
    }
    assert_eq!(export_class_map_internal()?, "{}");
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(Built {
        sources,
        css: with_style_sheet(|sheet| {
            vec![
                sheet.create_css(None, false),
                sheet.create_css(Some("bucket.tsx"), false),
            ]
        }),
        files: export_file_map_internal()?,
    })
}

#[test]
#[serial]
fn physical_raw_only_outputs_match_when_callbacks_reverse() -> TestResult {
    // Given: identical owned physical files, both collapsed into one canonical bucket.
    let fixture = Fixture::new()?;
    let inputs = fixture.sources()?;
    // When: complete reservation, followed by genuine extraction in opposite orders.
    let ab = build(&inputs, [0, 1])?;
    let ba = build(&inputs, [1, 0])?;
    // Then: actual source/maps/CSS/file assignments match, without class allocations.
    assert_eq!(ab, ba);
    Ok(())
}

#[test]
#[serial]
fn raw_virtual_slots_are_counted_once_with_legacy_spelling() -> TestResult {
    // Given
    let (_reset, option) = setup(false);
    seed_file_map(vec!["canonical.tsx".into()]);
    assert_eq!(file_map::get_file_num_by_filename("legacy.css.ts"), 1);
    assert_eq!(get_unreserved_class_slot_count(), 0);
    reserve_class_slots_internal(&[], option.clone(), None);
    seed_file_map(vec!["watch-bucket.tsx".into()]);
    assert_eq!(get_unreserved_class_slot_count(), 0);
    let original = file_map::get_file_map();
    let virtual_input = input(
        "virtual.css.ts",
        "import {createVar} from '@devup-ui/react'; export const variable=createVar();",
    );
    // When
    let first = transform(&virtual_input, &option)?;
    let repeated = transform(&virtual_input, &option)?;
    // Then
    assert_eq!(first.code, repeated.code);
    assert_eq!(get_unreserved_class_slot_count(), 1);
    assert_eq!(export_class_map_internal()?, "{}");
    reserve_class_slots_internal(
        &[input("watch.css.ts", VARIABLE_CONTAINER)],
        option.clone(),
        None,
    );
    assert_eq!(get_unreserved_class_slot_count(), 1);
    assert_eq!(css::sheet_to_variable_name("gap", 0, None), "--a");
    assert_eq!(get_unreserved_class_slot_count(), 2);
    crate::reset_build_state_internal();
    file_map::set_file_map(original);
    assert_eq!(transform(&virtual_input, &option)?.code, first.code);
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(())
}

fn watch_build(
    fixture: &Fixture,
    initial: &[ReservationInput],
    order: [usize; 3],
) -> Result<Built, Box<dyn std::error::Error>> {
    let (_reset, option) = setup(false);
    file_map::set_canonical_map(
        initial
            .iter()
            .map(|input| (input.filename.clone(), "bucket.tsx".into()))
            .collect(),
    );
    reserve_class_slots_internal(initial, option.clone(), None);
    for input in initial {
        transform(input, &option)?;
    }
    let previous = file_map::get_file_map();
    let mut revised = Vec::new();
    for (name, code) in [
        ("0-C.css.ts", VARIABLE_CONTAINER),
        ("0-D.css.ts", VARIABLE_LAYER),
    ] {
        let path = fixture.0.join(name);
        fs::write(&path, code)?;
        revised.push(input(&path.to_string_lossy(), &fs::read_to_string(&path)?));
    }
    fs::write(
        &initial[0].filename,
        format!("{VARIABLE_CONTAINER} export const added=createVar();"),
    )?;
    revised.push(input(
        &initial[0].filename,
        &fs::read_to_string(&initial[0].filename)?,
    ));
    let mut canonical = file_map::get_canonical_map();
    for input in revised.iter().take(2) {
        canonical.insert(input.filename.clone(), "watch-bucket.tsx".into());
    }
    file_map::set_canonical_map(canonical);
    seed_file_map(revised.iter().map(|input| input.filename.clone()).collect());
    assert_eq!(get_unreserved_class_slot_count(), 0);
    reserve_class_slots_internal(&revised, option.clone(), None);
    for (filename, ordinal) in &previous {
        assert_eq!(
            file_map::get_file_map().get_by_left(filename),
            Some(ordinal)
        );
    }
    let mut sources = BTreeMap::new();
    for index in order {
        let output = transform(&revised[index], &option)?;
        sources.insert(revised[index].filename.clone(), (output.code, output.map));
    }
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(Built {
        sources,
        files: export_file_map_internal()?,
        css: with_style_sheet(|sheet| {
            vec![
                sheet.create_css(None, false),
                sheet.create_css(Some("bucket.tsx"), false),
            ]
        }),
    })
}

#[test]
#[serial]
fn path_sorted_watch_append_preserves_old_raw_and_canonical_ordinals() -> TestResult {
    // Given
    let fixture = Fixture::new()?;
    let initial = fixture.sources()?;
    // When
    let forward = watch_build(&fixture, &initial, [0, 1, 2])?;
    let reverse = watch_build(&fixture, &initial, [2, 1, 0])?;
    // Then
    assert_eq!(forward, reverse);
    Ok(())
}

#[test]
#[serial]
fn unknown_scratch_supplier_file_slots_do_not_persist() -> TestResult {
    // Given
    let (_reset, option) = setup(false);
    let resolver = |_: &str, _: &str| {
        Some(extractor::ResolvedModule {
            path: "supplier.css.ts".into(),
            code: "import {createVar} from '@devup-ui/react'; export const token=createVar();"
                .into(),
        })
    };
    let root = input(
        "root.css.ts",
        "import {style} from '@devup-ui/react'; import {token} from './supplier.css'; export const own=style({ color: token });",
    );
    // When
    reserve_class_slots_internal(&[root], option, Some(&resolver));
    // Then
    assert_eq!(
        file_map::get_file_map().get_by_left("root.css.ts"),
        Some(&0)
    );
    assert_eq!(
        file_map::get_file_map().get_by_left("supplier.css.ts"),
        None
    );
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(())
}

#[test]
#[serial]
fn unwind_rolls_back_inventory_raw_slots_and_previous_residual_count() -> TestResult {
    // Given
    let (_reset, option) = setup(false);
    reserve_class_slots_internal(&[], option.clone(), None);
    transform(&input("virtual.css.ts", VARIABLE_CONTAINER), &option)?;
    let before = (
        export_file_map_internal()?,
        export_class_map_internal()?,
        crate::export_sheet_internal()?,
        get_unreserved_class_slot_count(),
    );
    let resolver =
        |_: &str, _: &str| -> Option<extractor::ResolvedModule> { panic!("supplier failure") };
    // When
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reserve_class_slots_internal(
            &[
                input("new.css.ts", VARIABLE_LAYER),
                input(
                    "root.css.ts",
                    "import {style} from '@devup-ui/react'; import {base} from './supplier.css'; export const own=style(base);",
                ),
            ],
            option,
            Some(&resolver),
        );
    }));
    // Then
    assert!(result.is_err());
    assert_eq!(
        (
            export_file_map_internal()?,
            export_class_map_internal()?,
            crate::export_sheet_internal()?,
            get_unreserved_class_slot_count()
        ),
        before
    );
    Ok(())
}
