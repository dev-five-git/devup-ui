use crate::{import_alias_visit::source_offset, vanilla_extract::Stylesheet};

pub(super) struct Analysis {
    pub insertions: Vec<(u32, String)>,
    pub errors: Vec<(u32, String)>,
    pub ranges: Vec<(u32, u32)>,
}

/// An offset as an index, which it always fits on the targets the build runs on
fn wide(offset: u32) -> Result<usize, String> {
    usize::try_from(offset).map_err(|error| error.to_string())
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
                let offset = wide(*offset)?;
                let key = authored(offset);
                let call = self
                    .ranges
                    .iter()
                    .filter(|(start, end)| {
                        usize::try_from(*start).is_ok_and(|start| start <= offset)
                            && usize::try_from(*end).is_ok_and(|end| offset <= end)
                    })
                    .min_by_key(|(start, end)| end - start)
                    .map(|(start, _)| *start);
                let call = call.map(wide).transpose()?.map(authored);
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
