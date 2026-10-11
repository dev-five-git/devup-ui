use oxc_ast::AstKind;
use oxc_semantic::Semantic;
use oxc_span::GetSpan;

use super::{edits::Replacement, execution::Executed};
use crate::{imported_constants::consumer::ReadPlan, vanilla_extract::Stylesheet};

pub(super) fn replace(
    source: (Stylesheet<'_>, &Semantic<'_>),
    plan: &ReadPlan,
    output: (&Executed, &mut Vec<Replacement>),
) -> Result<(), String> {
    let (stylesheet, semantic) = source;
    let (result, replacements) = output;
    for span in &plan.slots {
        let capture = result.readback.iter().find(|(root, _)| root == span).map(|(_, expression)| expression)
            .ok_or_else(|| format!("{}: required consumer read has no finalized capture. Fix: report this extraction error", super::execution::policy::place(stylesheet, span.start)))?;
        let shorthand = semantic.nodes().iter().find_map(|node| match node.kind() {
            AstKind::ObjectProperty(property)
                if property.shorthand && property.value.span() == *span =>
            {
                property.key.static_name()
            }
            _ => None,
        });
        let text = match shorthand {
            Some(key) => format!("{key}: {capture}"),
            None => capture.clone(),
        };
        replacements.push(Replacement { span: *span, text });
    }
    Ok(())
}
