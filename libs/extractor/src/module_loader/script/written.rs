use std::rc::Rc;

use super::{Unit, Written};
use crate::{
    import_alias_visit::Edit,
    source_map::Trace,
    vanilla_extract::{Stylesheet, strip_typescript_marked},
};

impl Unit {
    pub(crate) fn written(
        filename: &str,
        code: &str,
        source: &str,
        edits: &[&[Edit]],
    ) -> Result<Rc<Self>, String> {
        Self::instrumented(
            Stylesheet {
                filename,
                code,
                source,
                edits,
            },
            |_| false,
        )
    }

    pub(crate) fn instrumented(
        input: Stylesheet<'_>,
        generated: impl Fn(usize) -> bool,
    ) -> Result<Rc<Self>, String> {
        crate::module_loader::validate(input)?;
        let stripped = strip_typescript_marked(input.code, input.filename);
        let mut instrumentation = Vec::new();
        let mut previous = false;
        for &(at, original) in &stripped.marks {
            let current = generated(original);
            if current != previous {
                instrumentation.push((at, current));
                previous = current;
            }
        }
        Ok(Rc::new(Self {
            filename: input.filename.to_string(),
            script: stripped.code.clone(),
            written: Some(Written {
                source: input.source.to_string(),
                trace: Trace::new(&stripped.marks, input.edits),
                retained: None,
                instrumentation,
            }),
        }))
    }
}
