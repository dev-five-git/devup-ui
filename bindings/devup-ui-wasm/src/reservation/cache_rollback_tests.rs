use super::tests::{TestResult, input, setup};
use super::*;
use serial_test::serial;

#[test]
#[serial]
fn failed_imported_evaluation_does_not_reuse_rolled_back_references() -> TestResult {
    for single_css in [false, true] {
        // Given
        let (_reset, option) = setup(single_css);
        let root = "import {style} from '@devup-ui/react'; import {base,token} from './supplier.css'; export const own=style([base,{background:token}]);";
        let rows = [input("bad.css.ts", root), input("good.css.ts", root)];
        let resolver = |_: &str, _: &str| {
            Some(extractor::ModuleResolution::Resolved(extractor::ResolvedModule {
            path: "supplier.css.ts".into(),
            code: "import {style,createVar,createContainer,layer} from '@devup-ui/react'; export const token=createVar(); export const container=createContainer(); export const namedLayer=layer(); export const base=style({color:'red'});".into(),
            source_type: None,
        }))
        };
        let mut control_code = String::new();
        reserve_class_slots_with(&rows, option.clone(), |row, options| {
            let code = if row.filename == "bad.css.ts" {
                ""
            } else {
                &row.code
            };
            let result = extract_with_source_type(
                &row.filename,
                code,
                options,
                false,
                Some(&resolver),
                None,
            );
            if row.filename == "good.css.ts" {
                control_code = result.as_ref().map_err(ToString::to_string)?.code.clone();
            }
            result.map_err(|error| error.to_string())
        });
        assert_eq!(get_suppressed_reservation_row_count(), 0);
        let control = (css::class_map::get_class_map(), file_map::get_file_map());
        crate::reset_build_state_internal();
        let mut actual_code = String::new();
        // When
        reserve_class_slots_with(&rows, option, |row, options| {
            let result = extract_with_source_type(
                &row.filename,
                &row.code,
                options,
                false,
                Some(&resolver),
                None,
            )
            .map_err(|error| error.to_string());
            let output = result?;
            if row.filename == "bad.css.ts" {
                assert!(
                    file_map::get_file_map()
                        .get_by_left("supplier.css.ts")
                        .is_some()
                );
                return Err("failure after imported stylesheet evaluation".to_string());
            }
            actual_code = output.code.clone();
            Ok(output)
        });
        // Then
        assert_eq!(get_suppressed_reservation_row_count(), 1);
        assert_eq!(get_unreserved_class_slot_count(), 0);
        assert_eq!(css::class_map::get_class_map(), control.0);
        assert_eq!(file_map::get_file_map(), control.1);
        assert_eq!(actual_code, control_code);
    }
    Ok(())
}
