use crate::{import_alias_visit::source_offset, vanilla_extract::Stylesheet};

pub(super) struct Analysis {
    pub insertions: Vec<(u32, String)>,
    pub errors: Vec<(u32, String)>,
    pub ranges: Vec<(u32, u32)>,
}

impl Analysis {
    pub fn diagnostics(
        &self,
        input: Stylesheet<'_>,
        evaluated: Option<&[(usize, usize)]>,
    ) -> Result<String, String> {
        let authored = |offset| {
            input
                .edits
                .iter()
                .fold(offset, |offset, edits| source_offset(edits, offset))
        };
        self.errors
            .iter()
            .map(|(offset, message)| {
                let offset = usize::try_from(*offset).map_err(|error| error.to_string())?;
                let key = authored(offset);
                let call = self
                    .ranges
                    .iter()
                    .filter(|(start, end)| {
                        usize::try_from(*start).is_ok_and(|start| start <= offset)
                            && usize::try_from(*end).is_ok_and(|end| offset <= end)
                    })
                    .min_by_key(|(start, end)| end - start)
                    .map(|(start, _)| usize::try_from(*start).map(authored))
                    .transpose()
                    .map_err(|error| error.to_string())?;
                let owner = evaluated.and_then(|ranges| {
                    ranges
                        .iter()
                        .filter(|(start, end)| {
                            call.is_some_and(|call| *start <= call && call <= *end)
                                && *start <= key
                                && key <= *end
                        })
                        .min_by_key(|(start, end)| end - start)
                });
                let at = owner.map_or(key, |(start, _)| *start);
                Ok(format!(
                    "{}: {message}",
                    crate::locate(input.filename, input.source, at)
                ))
            })
            .collect::<Result<Vec<_>, String>>()
            .map(|errors| errors.join("\n"))
    }
}
