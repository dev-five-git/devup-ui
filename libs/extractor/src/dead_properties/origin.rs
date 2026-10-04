use crate::{
    ExtractOption,
    import_alias_visit::{source_offset, transform_import_aliases_with_edits},
};

pub(crate) fn map_evaluated(
    errors: &mut [(u32, String)],
    edits: &[&[crate::import_alias_visit::Edit]],
    calls: &[(usize, usize)],
) -> Result<(), std::num::TryFromIntError> {
    for (offset, message) in errors {
        let authored = edits
            .iter()
            .fold(usize::try_from(*offset)?, |offset, edits| {
                source_offset(edits, offset)
            });
        let authored = if super::terminal_error(message) {
            calls
                .iter()
                .filter(|(start, end)| *start <= authored && authored <= *end)
                .min_by_key(|(start, end)| end - start)
                .map_or(authored, |(start, _)| *start)
        } else {
            authored
        };
        *offset = u32::try_from(authored)?;
    }
    Ok(())
}

pub(crate) fn evaluated_calls(
    source: &str,
    filename: &str,
    option: &ExtractOption,
) -> Vec<(usize, usize)> {
    let (aliased, edits) = transform_import_aliases_with_edits(
        source,
        filename,
        &option.package,
        &option.import_aliases,
    );
    match super::instrument(&aliased, filename, &option.package) {
        Ok(instrumented) => instrumented
            .calls
            .into_iter()
            .filter_map(|(start, end)| {
                Some((
                    source_offset(&edits, usize::try_from(start).ok()?),
                    source_offset(&edits, usize::try_from(end).ok()?),
                ))
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}
