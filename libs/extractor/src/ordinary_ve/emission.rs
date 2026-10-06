use super::edits::Replacement;
use crate::vanilla_extract::capture::Emission;
use oxc_span::Span;

pub(super) fn apply(emission: &Emission, replacements: &mut Vec<Replacement>) {
    for (span, declarator, code) in &emission.inputs {
        replacements.push(Replacement {
            span: Span::new(span.end, span.end),
            text: if *declarator {
                format!(", []=((()=>{{{code}}})(),[])")
            } else {
                format!("\n{code}\n")
            },
        });
    }
}
