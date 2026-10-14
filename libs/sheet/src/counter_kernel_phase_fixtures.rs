use super::{
    Candidate, FrozenAuthority, LinkedBatch, cleanup_tests::global_dynamic, fixtures::*,
    scratch::ScratchUpdate,
};
use crate::{StyleSheet, StyleSheetCss, emission_seed::EmissionInput};
use css::allocation_input::{LegacyInput, NameMode};

pub(super) fn replacement() -> (Candidate, FrozenAuthority) {
    let (candidate, _) = global_dynamic(false);
    let mut seed = candidate.proof.emission.seed;
    let EmissionInput::Dynamic { variable, site, .. } = &mut seed.body else {
        panic!("dynamic")
    };
    site.as_mut().unwrap_or_else(|| panic!("site")).at = 3;
    *variable = "---pSj-d-b".into();
    let mut input = candidate.proof.allocation.input;
    let LegacyInput::Declaration(declaration) = &mut input else {
        panic!("declaration")
    };
    declaration.value = Some("var(---pSj-d-b) !important".into());
    let (mut candidate, mut authority) = fixture(seed, input, NameMode::Counter);
    candidate.proof.allocation = envelope(
        candidate.proof.allocation.input.clone(),
        candidate.proof.allocation.context.clone(),
        5,
    );
    candidate.proof.emission.name = candidate.proof.allocation.allocation.name.clone();
    candidate.proof.emission.expansion = require_ok(
        candidate
            .proof
            .emission
            .seed
            .replay(&candidate.proof.emission.name),
    );
    retain_map(&mut authority, &candidate.proof.allocation);
    (candidate, authority)
}

pub(super) fn combine(base: &FrozenAuthority, incoming: &FrozenAuthority) -> FrozenAuthority {
    assert_eq!(base.placements, incoming.placements);
    let mut authority = base.clone();
    for (namespace, keys) in &incoming.classes {
        authority
            .classes
            .entry(namespace.clone())
            .or_default()
            .extend(keys.clone());
    }
    authority.originals.extend(incoming.originals.clone());
    authority.files.extend(incoming.files.clone());
    authority.deliveries.extend(incoming.deliveries.clone());
    authority
}

pub(super) fn relink(output: &ScratchUpdate, authority: &FrozenAuthority) -> LinkedBatch {
    let mut authority = authority.clone();
    authority.authored = output.evidence.authored.clone();
    authority.cleanups = output.cleanups.clone();
    authority.phase = output.phase.clone();
    require_ok(LinkedBatch::link_captured_batch(
        &output.candidates,
        &output.evidence,
        &authority,
    ))
}

pub(super) fn output_sheet(output: &ScratchUpdate) -> StyleSheet {
    StyleSheet {
        properties: output.properties.clone(),
        keyframes: output.keyframes.clone(),
        css: output
            .css
            .iter()
            .map(|(source, rules)| {
                (
                    source.clone(),
                    rules
                        .iter()
                        .map(|rule| StyleSheetCss {
                            css: rule.css.clone(),
                        })
                        .collect(),
                )
            })
            .collect(),
        global_css_files: output.global_css_files.clone(),
        imports: output.imports.clone(),
        font_faces: output.font_faces.clone(),
        ..StyleSheet::default()
    }
}
