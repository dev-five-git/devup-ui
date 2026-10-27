use super::tests::{TestResult, input, setup, transform};
use super::*;
use serial_test::serial;

const ALLOCATIONS: &str = "import {Box,keyframes,createVar} from '@devup-ui/react'; const token=createVar(); const spin=keyframes({from:{opacity:0},to:{opacity:1}}); export const view=<Box color='salmon' gap={value} animation={`${spin} 1s`}/>;";
const SUFFIX: &str = "import {Box} from '@devup-ui/react'; export const view=<Box color='blue'/>;";

#[test]
#[serial]
fn failed_row_discards_real_allocations_before_valid_suffix() -> TestResult {
    for single_css in [false, true] {
        // Given: identical FILE inventory, but the control skips the faulty extraction.
        let (_reset, option) = setup(single_css);
        let rows = [input("bad.tsx", ALLOCATIONS), input("suffix.tsx", SUFFIX)];
        reserve_class_slots_with(&rows, option.clone(), |row, options| {
            if row.filename == "bad.tsx" {
                return extract_with_source_type(&row.filename, "", options, false, None, None);
            }
            extract_with_source_type(&row.filename, &row.code, options, false, None, None)
        });
        let control = (
            css::class_map::get_class_map(),
            file_map::get_file_map(),
            export_reservation_state_internal()?,
        );
        crate::reset_build_state_internal();
        // When: the real extractor allocates every kind before the row fails.
        reserve_class_slots_with(&rows, option.clone(), |row, options| {
            let output =
                extract_with_source_type(&row.filename, &row.code, options, false, None, None)
                    .map_err(|error| error.to_string())?;
            if row.filename == "bad.tsx" {
                assert!(
                    css::class_map::get_class_map()
                        .values()
                        .map(std::collections::HashMap::len)
                        .sum::<usize>()
                        >= 4
                );
                let _ = file_map::get_file_num_by_filename("extraction-only.css.ts");
                return Err("late row failure".to_string());
            }
            Ok(output)
        });
        // Then
        assert_eq!(css::class_map::get_class_map(), control.0);
        assert_eq!(file_map::get_file_map(), control.1);
        assert_eq!(get_unreserved_class_slot_count(), 0);
        assert_eq!(get_suppressed_reservation_row_count(), 1);
        let mut expected: serde_json::Value = serde_json::from_str(&control.2)?;
        expected["suppressed"] = serde_json::json!(1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&export_reservation_state_internal()?)?,
            expected
        );
        let slots = css::class_map::get_class_map();
        reserve_class_slots_internal(&rows, option.clone(), None);
        assert_eq!(get_suppressed_reservation_row_count(), 0);
        let retried = css::class_map::get_class_map();
        for (file, entries) in slots {
            for (key, number) in entries {
                assert_eq!(retried[&file][&key], number);
            }
        }
        assert!(
            retried
                .values()
                .map(std::collections::HashMap::len)
                .sum::<usize>()
                > control
                    .0
                    .values()
                    .map(std::collections::HashMap::len)
                    .sum::<usize>()
        );
        transform(&rows[0], &option)?;
        assert_eq!(get_unreserved_class_slot_count(), 0);
    }
    Ok(())
}

#[test]
#[serial]
fn native_extraction_error_rolls_back_allocations_and_owner_suppression() -> TestResult {
    for single_css in [false, true] {
        // Given
        let (_reset, option) = setup(single_css);
        let bad = input(
            "bad.tsx",
            "import {Box,css,keyframes} from '@devup-ui/react'; const spin=keyframes({from:{opacity:0},to:{opacity:1}}); const first=<Box color='salmon' gap={value}/>; css({color:window.name});",
        );
        let result =
            extract_with_source_type(&bad.filename, &bad.code, option.clone(), false, None, None);
        assert!(result.is_err());
        assert!(
            css::class_map::get_class_map()
                .values()
                .map(std::collections::HashMap::len)
                .sum::<usize>()
                > 0
        );
        crate::reset_build_state_internal();
        let sheet = crate::export_sheet_internal()?;
        // When
        reserve_class_slots_internal(&[bad], option.clone(), None);
        // Then
        assert_eq!(css::class_map::get_class_map().len(), 0);
        assert_eq!(file_map::get_file_map().len(), 1);
        assert_eq!(crate::export_sheet_internal()?, sheet);
        assert_eq!(get_suppressed_reservation_row_count(), 1);
        let owner = export_reservation_state_internal()?;
        crate::reset_build_state_internal();
        assert_eq!(get_suppressed_reservation_row_count(), 0);
        import_reservation_state_internal(serde_json::from_str(&owner)?);
        assert_eq!(get_suppressed_reservation_row_count(), 1);
        let _ = file_map::get_file_num_by_filename("live.tsx");
        assert_eq!(get_unreserved_class_slot_count(), 1);
        reserve_class_slots_internal(&[], option, None);
        assert_eq!(get_suppressed_reservation_row_count(), 0);
        assert_eq!(get_unreserved_class_slot_count(), 1);
    }
    Ok(())
}

#[test]
#[serial]
fn host_fault_after_imported_allocations_matches_skipped_control() -> TestResult {
    use std::cell::Cell;
    for single_css in [false, true] {
        // Given
        let (_reset, option) = setup(single_css);
        let supplier = "import {style,createVar,keyframes} from '@devup-ui/react'; export const base=style({color:'salmon'}); export const token=createVar(); export const spin=keyframes({from:{opacity:0},to:{opacity:1}});";
        let bad = format!(
            "{ALLOCATIONS} import {{base}} from './supplier.css'; import {{FAULT}} from './fault'; const tail=<Box className={{base}} bg={{FAULT}}/>;"
        );
        let good = "import {style} from '@devup-ui/react'; import {base,token} from './supplier.css'; export const own=style([base,{background:token}]);";
        let rows = [input("bad.tsx", &bad), input("good.css.ts", good)];
        let saw_partial = Cell::new(false);
        let resolver = |specifier: &str, _: &str| match specifier {
            "./supplier.css" => Ok(Some(extractor::ModuleResolution::Resolved(
                extractor::ResolvedModule {
                    path: "supplier.css.ts".into(),
                    code: supplier.into(),
                    source_type: None,
                },
            ))),
            "./fault" => Err("late host fault".into()),
            _ => Ok(None),
        };
        reserve_class_slots_with(&rows, option.clone(), |row, options| {
            if row.filename == "bad.tsx" {
                extract_with_source_type(&row.filename, "", options, false, None, None)
                    .map_err(|error| error.to_string())
            } else {
                let result = extract_scratch_row(row, options, &resolver);
                assert!(
                    result.is_ok(),
                    "valid suffix failed: {:?}",
                    result.as_ref().err()
                );
                result
            }
        });
        let control = (
            css::class_map::get_class_map(),
            file_map::get_file_map(),
            export_reservation_state_internal()?,
        );
        crate::reset_build_state_internal();
        // When
        reserve_class_slots_with(&rows, option.clone(), |row, options| {
            let result = extract_scratch_row(row, options, &resolver);
            if row.filename == "bad.tsx" {
                assert!(result.is_err());
                assert!(
                    css::class_map::get_class_map()
                        .values()
                        .map(std::collections::HashMap::len)
                        .sum::<usize>()
                        >= 4
                );
                saw_partial.set(true);
            }
            result
        });
        // Then: the suffix must regenerate any cached references to rolled-back slots.
        assert!(saw_partial.get());
        assert_eq!(css::class_map::get_class_map(), control.0);
        assert_eq!(file_map::get_file_map(), control.1);
        assert_eq!(get_suppressed_reservation_row_count(), 1);
        assert_eq!(get_unreserved_class_slot_count(), 0);
        let mut expected: serde_json::Value = serde_json::from_str(&control.2)?;
        expected["suppressed"] = serde_json::json!(1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&export_reservation_state_internal()?)?,
            expected
        );
    }
    Ok(())
}
