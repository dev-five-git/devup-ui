use oxc_semantic::Semantic;
use oxc_span::GetSpan;

use super::super::{edits::Replacement, execution::SelectedModule};

pub(super) fn check(
    module: SelectedModule<'_>,
    semantic: &Semantic<'_>,
    replacements: &[Replacement],
) -> Result<(), String> {
    let SelectedModule {
        stylesheet,
        selection,
    } = module;
    for import in selection
        .imports
        .iter()
        .filter(|import| import.native.is_some() && !import.preserved)
    {
        for reference in semantic
            .scoping()
            .get_resolved_reference_ids(import.binding.symbol)
        {
            let reference = semantic.scoping().get_reference(*reference);
            let span = semantic.nodes().kind(reference.node_id()).span();
            if reference.is_value()
                && !replacements
                    .iter()
                    .any(|edit| edit.span.contains_inclusive(span))
            {
                return Err(crate::located_errors(
                    stylesheet.filename,
                    stylesheet.source,
                    stylesheet.edits,
                    vec![(
                        span.start,
                        format!(
                            "native binding `{}` escapes preserved source. Fix: use the API only in exact initializers and retain computed values",
                            import.binding.name
                        ),
                    )],
                ));
            }
        }
    }
    for call in &selection.native_calls {
        if !selection
            .roots
            .iter()
            .any(|root| root.native_calls.contains(&call.node))
            || !replacements
                .iter()
                .any(|edit| edit.span.contains_inclusive(call.span))
        {
            return Err(crate::located_errors(
                stylesheet.filename,
                stylesheet.source,
                stylesheet.edits,
                vec![(
                    call.callee.start,
                    format!(
                        "native `{}` call cannot remain in preserved source. Fix: report this extraction error with the original module",
                        call.api
                    ),
                )],
            ));
        }
    }
    Ok(())
}
