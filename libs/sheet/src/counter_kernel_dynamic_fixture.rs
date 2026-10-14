use super::{Candidate, FrozenAuthority, VariableLineage, fixtures::*};
use crate::emission_seed::{EmissionInput, NumericSite};
use css::allocation_input::{AllocationContext, LegacyDeclaration, LegacyInput, NameMode};

pub(super) fn dynamic(mode: NameMode, site: bool) -> (Candidate, FrozenAuthority) {
    let declaration = declaration("padding", "identifier");
    let selector = match mode {
        NameMode::AtomHoist => Some(css::atom_name::selector_key(None, None)),
        NameMode::Counter | NameMode::Debug => None,
    };
    let variable_input = LegacyInput::Variable(css::allocation_input::LegacyVariable {
        property: "padding".into(),
        level: 0,
        selector: selector.clone(),
    });
    let variable_context = AllocationContext {
        config: css::allocation_input::CapturedNameConfig {
            prefix: "p".into(),
            mode,
        },
        file: None,
        delivery: None,
    };
    let variable_evidence = envelope(variable_input, variable_context, 2);
    let variable = if site {
        "---pSj-c-b".into()
    } else {
        variable_evidence.allocation.name.clone()
    };
    let value = match mode {
        NameMode::Counter | NameMode::Debug if !site => Some("!important".into()),
        NameMode::Counter | NameMode::Debug | NameMode::AtomHoist => {
            Some(format!("var({variable}) !important"))
        }
    };
    let (mut candidate, mut authority) = fixture(
        seed(EmissionInput::Dynamic {
            declaration,
            variable,
            site: site.then_some(NumericSite {
                source: 9,
                at: 2,
                role: 1,
            }),
            important: true,
        }),
        LegacyInput::Declaration(LegacyDeclaration {
            property: "padding".into(),
            level: 0,
            value,
            selector,
            order: None,
        }),
        mode,
    );
    if !site {
        retain_map(&mut authority, &variable_evidence);
        candidate.lineage.variable = Some(VariableLineage {
            original: 7,
            evidence: variable_evidence,
        });
    }
    (candidate, authority)
}
