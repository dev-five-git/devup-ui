use super::{SelectedModule, policy};
use crate::module_loader::Mapped;
use crate::ordinary_ve::selection::plan::{Unit, UnitKind};
use crate::vanilla_extract::capture::Capture;
use crate::vanilla_extract::capture::{Observation, ObservationKind};
use std::collections::BTreeSet;

pub(super) struct Observations {
    pub helper: String,
    pub sites: Vec<Observation>,
}

pub(super) fn before(module: SelectedModule<'_>, unit: &Unit) -> Observation {
    let mutations = module
        .selection
        .mutations
        .iter()
        .filter_map(|mutation| {
            let at = match &mutation.usage {
                crate::mutations::Use::Changes { at, .. }
                | crate::mutations::Use::Calls { at, .. }
                | crate::mutations::Use::Escapes { at, .. } => *at,
            };
            if !(unit.span.start..unit.span.end).contains(&at) {
                return None;
            }
            let binding = module
                .selection
                .units
                .iter()
                .flat_map(|unit| &unit.bindings)
                .chain(
                    module
                        .selection
                        .imports
                        .iter()
                        .map(|import| &import.binding),
                )
                .find(|binding| binding.symbol == mutation.symbol)?;
            Some((binding.name.clone(), policy::place(module.stylesheet, at)))
        })
        .collect();
    Observation {
        span: unit.span,
        place: policy::place(module.stylesheet, unit.span.start),
        kind: ObservationKind::Before,
        reads: Vec::new(),
        mutations,
    }
}

pub(super) fn after(
    module: SelectedModule<'_>,
    unit: &Unit,
    captures: &[Capture],
) -> Option<Observation> {
    let root = module
        .selection
        .roots
        .iter()
        .any(|root| root.owner == unit.node);
    let consumed = module
        .selection
        .consumed
        .iter()
        .any(|consumed| consumed.node == unit.node);
    let (kind, reads) = if root {
        (
            ObservationKind::After,
            captures
                .iter()
                .map(|capture| capture.read.clone())
                .collect(),
        )
    } else if !consumed {
        (
            ObservationKind::Input {
                declarator: matches!(unit.kind, UnitKind::Declarator { .. }),
            },
            unit.bindings
                .iter()
                .map(|binding| binding.name.clone())
                .collect(),
        )
    } else {
        return None;
    };
    Some(Observation {
        span: unit.span,
        place: policy::place(module.stylesheet, unit.span.start),
        kind,
        reads,
        mutations: Vec::new(),
    })
}

impl Observations {
    pub fn new(reserved: &mut BTreeSet<String>) -> Self {
        let mut helper = "__ve_observe__".to_string();
        while reserved.contains(&helper) {
            helper.push('_');
        }
        reserved.insert(helper.clone());
        Self {
            helper,
            sites: Vec::new(),
        }
    }

    pub fn write(&mut self, mapped: &mut Mapped, site: Observation) {
        let index = self.sites.len();
        let arguments = if site.reads.is_empty() {
            String::new()
        } else {
            format!(",{}", site.reads.join(","))
        };
        let at = match site.kind {
            ObservationKind::Before => site.span.start,
            ObservationKind::Input { .. } | ObservationKind::After => site.span.end,
        };
        mapped.synthesize(at, &format!("{}({index}{arguments});\n", self.helper));
        self.sites.push(site);
    }
}
