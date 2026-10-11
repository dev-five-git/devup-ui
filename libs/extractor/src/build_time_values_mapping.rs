//! Original-source tracing for generated build-time helper statements.

use crate::evaluation_sandbox::{Violation, instrument};
use crate::source_map::{Lines, Trace};

#[derive(Default)]
pub(super) struct Generated {
    pub(super) text: String,
    copied: Vec<(usize, usize, usize)>,
}

impl Generated {
    pub(super) fn copy(&mut self, source: &str, from: usize, to: usize) {
        let start = self.text.len();
        self.text.push_str(&source[from..to]);
        self.copied.push((start, self.text.len(), from));
    }

    fn locate(&self, offset: usize) -> Option<usize> {
        self.copied
            .iter()
            .find(|(start, end, _)| (*start..*end).contains(&offset))
            .map(|(start, _, original)| original + offset - start)
    }
}

pub(super) struct Mapped {
    generated: Generated,
    stripped: std::rc::Rc<crate::vanilla_extract::Stripped>,
    trace: Trace,
    path: String,
    pub(super) instrumented: crate::evaluation_sandbox::Instrumented,
}

impl Mapped {
    pub(super) fn new(generated: Generated, filename: &str, path: &str) -> Self {
        let stripped = crate::vanilla_extract::strip_typescript_marked(&generated.text, filename);
        let trace = Trace::new(&stripped.marks, &[]);
        let instrumented = instrument(&stripped.code, path);
        Self {
            generated,
            stripped,
            trace,
            path: path.to_string(),
            instrumented,
        }
    }

    pub(super) fn locate(&self, violation: &Violation) -> Option<usize> {
        let offset = match violation.site() {
            Some((place, offset)) if place.starts_with(&format!("{}:", self.path)) => offset,
            Some(_) => return None,
            None => {
                let error = self.instrumented.explain(&violation.error().to_string());
                let frame = error.lines().find_map(|line| {
                    line.split_once(&format!("({}:", self.path))
                        .map(|(_, position)| position)
                })?;
                let (line, column) = frame.trim_end_matches(')').split_once(':')?;
                Lines::new(&self.stripped.code).code_point_offset(
                    line.parse::<u32>().ok()?.saturating_sub(1),
                    column.parse::<u32>().ok()?.saturating_sub(1),
                )
            }
        };
        self.generated.locate(
            self.trace
                .resolve(&self.stripped.code, &self.generated.text, offset),
        )
    }
}

#[cfg(test)]
mod build_time_values_mapping_coverage_tests;
