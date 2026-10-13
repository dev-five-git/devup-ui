use oxc_span::Span;

pub(super) struct Replacement {
    pub span: Span,
    pub text: String,
}

pub(super) struct Overlap {
    pub at: u32,
    pub message: String,
}

pub(super) fn apply(
    source: &str,
    mut replacements: Vec<Replacement>,
) -> Result<(String, Vec<crate::import_alias_visit::Edit>), Overlap> {
    replacements.sort_by_key(|edit| (edit.span.start, edit.span.end));
    let mut code = String::new();
    let mut edits = Vec::new();
    let mut copied = 0;
    for edit in replacements {
        let start = Span::new(0, edit.span.start).source_text(source).len();
        let end = start + edit.span.source_text(source).len();
        if start < copied {
            return Err(Overlap {
                at: edit.span.start,
                message: "native source replacements overlap. Fix: report this extraction error with the original module".into(),
            });
        }
        code.push_str(&source[copied..start]);
        code.push_str(&edit.text);
        edits.push((start, end, edit.text.len()));
        copied = end;
    }
    code.push_str(&source[copied..]);
    Ok((code, edits))
}
