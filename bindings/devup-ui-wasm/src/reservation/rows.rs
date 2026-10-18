use std::fmt;

use super::ReservationInput;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RowArityError {
    row: usize,
    fields: usize,
}

impl fmt::Display for RowArityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "reservation row {} must contain exactly 3 strings, got {} fields",
            self.row, self.fields
        )
    }
}

impl std::error::Error for RowArityError {}

pub(super) fn parse_rows(rows: Vec<Vec<String>>) -> Result<Vec<ReservationInput>, RowArityError> {
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            let [filename, code, css_dir]: [String; 3] =
                row.try_into().map_err(|row: Vec<String>| RowArityError {
                    row: index,
                    fields: row.len(),
                })?;
            Ok(ReservationInput {
                filename,
                code,
                css_dir,
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
    fn two_and_four_fields_are_rejected() {
        // Given / When / Then
        for fields in [2, 4] {
            let error = parse_rows(vec![vec![String::new(); fields]]).err();
            assert_eq!(error, Some(RowArityError { row: 0, fields }));
        }
        assert_eq!(
            RowArityError { row: 1, fields: 4 }.to_string(),
            "reservation row 1 must contain exactly 3 strings, got 4 fields"
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
        assert_eq!(result, Err(RowArityError { row: 1, fields: 2 }));
        assert_eq!(state()?, before);
        Ok(())
    }
}
