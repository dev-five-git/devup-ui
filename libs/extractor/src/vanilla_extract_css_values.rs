use super::StyleCollector;
use crate::own_css_plan::Plan;
use boa_engine::JsObject;
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "vanilla_extract_css_capture.rs"]
mod capture;

pub(super) struct Values<'a> {
    pub names: &'a FxHashMap<String, String>,
    pub collector: &'a StyleCollector,
    pub active: FxHashSet<JsObject>,
    pub plan: Plan,
}

impl Values<'_> {
    pub(super) fn declarations(&self, root: usize) {
        let errors = capture::validate(&self.plan, root);
        let mut collector = self.collector.borrow_mut();
        if let Some((location, api)) = collector.origin.clone() {
            collector
                .declaration_errors
                .extend(errors.into_iter().map(|(path, requirement)| {
                    format!(
                        "{location}: {}",
                        crate::utils::build_time_error(&api, &path, requirement)
                    )
                }));
        }
    }
}
