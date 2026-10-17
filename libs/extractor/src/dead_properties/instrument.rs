use crate::{import_alias_visit::Edit, vanilla_extract::Stylesheet};

pub(crate) struct Instrumented {
    pub code: String,
    pub calls: Vec<(u32, u32)>,
    pub edits: Vec<Edit>,
}

impl Instrumented {
    pub(crate) fn generated(&self, offset: usize) -> bool {
        let mut inserted = 0;
        self.edits.iter().any(|&(at, _, length)| {
            let start = at + inserted;
            inserted += length;
            (start..start + length).contains(&offset)
        })
    }
}

pub(crate) fn instrument(
    code: &str,
    filename: &str,
    package: &str,
) -> Result<Instrumented, String> {
    instrument_located(
        Stylesheet {
            code,
            filename,
            source: code,
            edits: &[],
        },
        package,
    )
}

pub(crate) fn instrument_located(
    input: Stylesheet<'_>,
    package: &str,
) -> Result<Instrumented, String> {
    instrument_evaluated(input, package, None)
}

pub(crate) fn instrument_evaluated(
    input: Stylesheet<'_>,
    package: &str,
    evaluated: Option<&[(usize, usize)]>,
) -> Result<Instrumented, String> {
    let mut analysis = super::calls::collect(input, package);
    if !analysis.errors.is_empty() {
        if evaluated.is_some() {
            let authored = Stylesheet {
                code: input.source,
                edits: &[],
                ..input
            };
            let original = super::calls::collect(authored, package);
            if !original.errors.is_empty() {
                return Err(original.diagnostics(authored, None)?);
            }
        }
        return Err(analysis.diagnostics(input, evaluated)?);
    }
    analysis.insertions.sort_by_key(|(offset, _)| *offset);
    let mut code = String::new();
    let mut copied = 0;
    let mut edits = Vec::new();
    for (offset, insertion) in analysis.insertions {
        let offset = usize::try_from(offset).map_err(|error| error.to_string())?;
        code.push_str(&input.code[copied..offset]);
        code.push_str(&insertion);
        edits.push((offset, offset, insertion.len()));
        copied = offset;
    }
    code.push_str(&input.code[copied..]);
    Ok(Instrumented {
        code,
        calls: analysis.ranges,
        edits,
    })
}
