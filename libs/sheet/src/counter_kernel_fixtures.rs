use super::{BatchPhase, Candidate, FrozenAuthority, Lineage, LinkedBatch};
pub(super) use crate::emission_seed_test_helpers::{declaration, preset, require_ok, seed};
use crate::{
    counter_evidence::{
        AllocationEvidence, CounterEvidence, EmissionWitness, ExpansionProof, Materialization,
    },
    emission_seed::{EmissionInput, EmissionSeed},
};
use css::{
    allocation_input::{
        AllocationContext, AllocationFile, CapturedDelivery, CapturedNameConfig, LegacyInput,
        NameMode,
    },
    counter_names::{AllocatedName, NameAddress, allocation_key, render_name},
};

pub(super) fn envelope(
    input: LegacyInput,
    context: AllocationContext,
    slot: usize,
) -> AllocationEvidence {
    let address = match allocation_key(&input, &context) {
        Some((namespace, legacy_key)) => NameAddress::Counter {
            namespace,
            legacy_key,
            slot,
        },
        None => NameAddress::Baseline {
            mode: context.config.mode,
            input: input.clone(),
            context: context.clone(),
        },
    };
    AllocationEvidence {
        allocation: AllocatedName {
            name: render_name(&input, &context, slot),
            address,
        },
        input,
        context,
    }
}

pub(super) fn fixture(
    seed: EmissionSeed,
    input: LegacyInput,
    mode: NameMode,
) -> (Candidate, FrozenAuthority) {
    let config = CapturedNameConfig {
        prefix: "p".into(),
        mode,
    };
    let shared = seed.placement.single_css
        || matches!(&input, LegacyInput::Declaration(declaration) if declaration.order == Some(0));
    let delivery = CapturedDelivery {
        canonical: seed.placement.bucket.clone(),
        hoisted: seed.placement.hoisted,
    };
    let context = AllocationContext {
        config: config.clone(),
        file: (!shared).then_some(AllocationFile::Original(7)),
        delivery: (!shared).then_some(delivery.clone()),
    };
    let allocation = envelope(input, context, 4);
    let name = allocation.allocation.name.clone();
    let children = match &seed.body {
        EmissionInput::Keyframes { steps } => vec![9; steps.values().map(Vec::len).sum()],
        EmissionInput::Static(_) | EmissionInput::Typography(_) | EmissionInput::Dynamic { .. } => {
            vec![]
        }
    };
    let candidate = Candidate {
        proof: ExpansionProof {
            allocation,
            emission: EmissionWitness {
                expansion: require_ok(seed.replay(&name)),
                seed: seed.clone(),
                name,
                materialization: Materialization::Complete,
            },
        },
        lineage: Lineage {
            parent: 7,
            children,
            variable: None,
        },
    };
    let mut authority = FrozenAuthority {
        config,
        originals: [(seed.placement.source_file.clone(), 7), ("child".into(), 9)].into(),
        files: [(seed.placement.bucket.clone(), 23)].into(),
        classes: Default::default(),
        placements: vec![seed.placement.clone()],
        deliveries: [(seed.placement.source_file, delivery)].into(),
        authored: vec![],
        cleanups: vec![],
        phase: BatchPhase::Fresh,
    };
    retain_map(&mut authority, &candidate.proof.allocation);
    (candidate, authority)
}

pub(super) fn retain_map(authority: &mut FrozenAuthority, evidence: &AllocationEvidence) {
    match &evidence.allocation.address {
        NameAddress::Counter {
            namespace,
            legacy_key,
            slot,
        } => {
            authority
                .classes
                .entry(namespace.clone())
                .or_default()
                .insert(legacy_key.clone(), *slot);
        }
        NameAddress::Baseline { .. } => {}
    }
}

pub(super) fn evidence(candidates: &[Candidate], authority: &FrozenAuthority) -> CounterEvidence {
    let mut evidence = CounterEvidence {
        authored: authority.authored.clone(),
        ..CounterEvidence::default()
    };
    for candidate in candidates {
        require_ok(evidence.insert(candidate.proof.clone()));
    }
    evidence
}

pub(super) fn linked(candidates: &[Candidate], authority: &FrozenAuthority) -> LinkedBatch {
    require_ok(LinkedBatch::link_captured_batch(
        candidates,
        &evidence(candidates, authority),
        authority,
    ))
}

pub(super) fn static_fixture(mode: NameMode) -> (Candidate, FrozenAuthority) {
    fixture(
        seed(EmissionInput::Static(declaration("font-size", "14px"))),
        LegacyInput::Declaration(css::allocation_input::LegacyDeclaration {
            property: "font-size".into(),
            level: 0,
            value: Some("14px".into()),
            selector: match mode {
                NameMode::AtomHoist => Some(css::atom_name::selector_key(None, None)),
                NameMode::Counter | NameMode::Debug => None,
            },
            order: None,
        }),
        mode,
    )
}

pub(super) fn sheet(batch: &LinkedBatch) -> crate::StyleSheet {
    let mut sheet = crate::StyleSheet::default();
    for record in &batch.records {
        super::emission::insert(&mut sheet, record);
    }
    sheet
}

pub(super) fn empty(authority: &FrozenAuthority) -> LinkedBatch {
    let mut authority = authority.clone();
    authority.authored.clear();
    authority.cleanups.clear();
    authority.phase = BatchPhase::Fresh;
    linked(&[], &authority)
}

pub(super) fn request<'a>(
    incoming: &'a LinkedBatch,
    cleanup: Option<&'a super::Cleanup>,
) -> super::scratch::ScratchRequest<'a> {
    require_ok(super::scratch::ScratchRequest::new(incoming, cleanup))
}
