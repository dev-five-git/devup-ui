use std::fmt;

use super::ReservationInput;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum RowError {
    Arity { row: usize, fields: usize },
    SourceType { filename: String, reason: String },
}

impl fmt::Display for RowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arity { row, fields } => write!(
                f,
                "reservation row {row} must contain 3 or 4 strings, got {fields} fields"
            ),
            Self::SourceType { filename, reason } => write!(f, "{filename}:1:1: {reason}"),
        }
    }
}

impl std::error::Error for RowError {}

pub(super) fn parse_rows(rows: Vec<Vec<String>>) -> Result<Vec<ReservationInput>, RowError> {
    rows.into_iter()
        .enumerate()
        .map(|(index, mut row)| {
            let source_type = if row.len() == 4 { row.pop() } else { None };
            let [filename, code, css_dir]: [String; 3] =
                row.try_into().map_err(|row: Vec<String>| RowError::Arity {
                    row: index,
                    fields: row.len(),
                })?;
            let source_type =
                extractor::parse_source_type(source_type.as_deref()).map_err(|reason| {
                    RowError::SourceType {
                        filename: filename.clone(),
                        reason,
                    }
                })?;
            Ok(ReservationInput {
                filename,
                code,
                css_dir,
                source_type,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reservation::tests::{TestResult, input, setup, transform};
    use serial_test::serial;

    #[test]
    fn exact_three_fields_convert_without_changing_values() -> TestResult {
        // Given
        let row = vec!["a.tsx".into(), "source".into(), "./css".into()];
        // When
        let inputs = parse_rows(vec![row])?;
        // Then
        assert_eq!(inputs.len(), 1);
        assert_eq!(
            (&*inputs[0].filename, &*inputs[0].code, &*inputs[0].css_dir),
            ("a.tsx", "source", "./css")
        );
        assert_eq!(parse_rows(vec![])?.len(), 0);
        Ok(())
    }

    #[test]
    fn two_and_five_fields_are_rejected() {
        // Given / When / Then
        for fields in [2, 5] {
            let error = parse_rows(vec![vec![String::new(); fields]]).err();
            assert_eq!(error, Some(RowError::Arity { row: 0, fields }));
        }
        assert_eq!(
            RowError::Arity { row: 1, fields: 5 }.to_string(),
            "reservation row 1 must contain 3 or 4 strings, got 5 fields"
        );
    }

    #[test]
    #[serial]
    fn later_invalid_row_prevents_entire_packet_mutation() -> TestResult {
        // Given
        let (_reset, option) = setup(true);
        crate::register_theme_internal(serde_json::from_value(
            serde_json::json!({"colors":{"default":{"brand":"red"}}}),
        )?);
        let existing = input(
            "existing.tsx",
            "import {Box} from '@devup-ui/react'; <Box color='red' />;",
        );
        super::super::reserve_class_slots_internal(&[existing], option.clone(), None);
        transform(
            &input(
                "existing.tsx",
                "import {Box} from '@devup-ui/react'; <Box color='red' />;",
            ),
            &option,
        )?;
        let _ = css::file_map::get_file_num_by_filename("existing-virtual");
        let state = || -> Result<_, Box<dyn std::error::Error>> {
            Ok((
                crate::export_class_map_internal()?,
                crate::export_file_map_internal()?,
                crate::export_sheet_internal()?,
                crate::with_style_sheet(|sheet| sheet.create_css(None, false)),
                crate::with_style_sheet(|sheet| serde_json::to_string(&sheet.theme))?,
                super::super::get_unreserved_class_slot_count(),
            ))
        };
        let before = state()?;
        let rows = vec![
            vec![
                "valid.tsx".into(),
                "import {Box} from '@devup-ui/react'; <Box bg='blue' />;".into(),
                "./css".into(),
            ],
            vec!["invalid.tsx".into(), "source".into()],
        ];
        // When
        let result = parse_rows(rows)
            .map(|inputs| super::super::reserve_class_slots_internal(&inputs, option, None));
        // Then
        assert_eq!(result, Err(RowError::Arity { row: 1, fields: 2 }));
        assert_eq!(state()?, before);
        Ok(())
    }

    #[test]
    fn compiled_mode_preserves_filename_and_rejects_unknown_modes() -> TestResult {
        let inputs = parse_rows(vec![vec![
            "page.mdown".into(),
            "source".into(),
            "./css".into(),
            "compiled-mdx".into(),
        ]])?;
        assert_eq!(inputs[0].filename, "page.mdown");
        assert_eq!(
            inputs[0].source_type,
            Some(extractor::ExtractSourceType::CompiledMdx)
        );
        let error = parse_rows(vec![vec![
            "page.mdown".into(),
            "source".into(),
            "./css".into(),
            "tsx".into(),
        ]])
        .err()
        .ok_or("unknown mode was accepted")?;
        assert_eq!(
            error.to_string(),
            "page.mdown:1:1: source type cannot use `tsx` at build time: expected `compiled-mdx`"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn scratch_fault_wins_over_native_fallback_and_valid_suffix_still_runs() -> TestResult {
        use super::super::{extract_scratch_row, reserve_class_slots_with};
        use std::cell::RefCell;

        for (source, native_ok) in [
            (
                "import {Box} from '@devup-ui/react'; import {A} from './a'; import {B} from './b'; <Box color={A} bg={B} />;",
                true,
            ),
            (
                "import {css} from '@devup-ui/react'; import {A} from './a'; css({ color: A });",
                false,
            ),
        ] {
            // Given
            let (_reset, option) = setup(true);
            let bad = input("scratch.tsx", source);
            assert_eq!(
                extractor::extract_with_source_type(
                    &bad.filename,
                    &bad.code,
                    option.clone(),
                    false,
                    None,
                    bad.source_type,
                )
                .is_ok(),
                native_ok
            );
            let calls = RefCell::new(Vec::new());
            let resolve = |specifier: &str, importer: &str| {
                calls
                    .borrow_mut()
                    .push((specifier.to_string(), importer.to_string()));
                match importer {
                    "scratch.tsx" => Err("scratchA".into()),
                    "suffix.mdown" => Ok(Some(extractor::ModuleResolution::Resolved(
                        extractor::ResolvedModule {
                            path: "prepared.mdown".into(),
                            code: "const node=<div/>; export const A='blue';".into(),
                            source_type: Some(extractor::ExtractSourceType::CompiledMdx),
                        },
                    ))),
                    _ => panic!("unexpected importer: {importer}"),
                }
            };
            let suffix = super::super::ReservationInput {
                source_type: Some(extractor::ExtractSourceType::CompiledMdx),
                ..input(
                    "suffix.mdown",
                    "import {Box} from '@devup-ui/react'; import {A} from './ready'; <Box color={A}/>;",
                )
            };
            let mut results = Vec::new();
            let before = crate::export_sheet_internal()?;
            // When
            reserve_class_slots_with(&[bad, suffix], option, |input, option| {
                if input.filename == "suffix.mdown" {
                    assert!(
                        !crate::with_style_sheet(|sheet| sheet.create_css(None, false))
                            .contains("var(--")
                    );
                }
                let result = extract_scratch_row(input, option, &resolve);
                results.push(result.as_ref().map(|_| ()).map_err(Clone::clone));
                result
            });
            // Then
            assert_eq!(
                results,
                vec![
                    Err(
                        "scratch.tsx:1:1: module resolver cannot use `./a` at build time: scratchA"
                            .into()
                    ),
                    Ok(()),
                ]
            );
            assert_eq!(
                *calls.borrow(),
                vec![
                    ("./a".into(), "scratch.tsx".into()),
                    ("./ready".into(), "suffix.mdown".into()),
                    ("./ready".into(), "suffix.mdown".into()),
                ]
            );
            assert_eq!(crate::export_sheet_internal()?, before);
        }
        Ok(())
    }
}
