//! Test-only measurements under immutable, predeclared module reach.

mod error_tests;
mod report;
mod snapshot;

use snapshot::Snapshot;

use super::{
    code_extract_internal, register_theme_internal, reset_build_state_internal, seed_file_map,
    set_atom_hoist, with_style_sheet,
};
use std::collections::{HashMap, HashSet};

const ROUTE_IDS: [u32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
const ROUTES: usize = ROUTE_IDS.len();
const SHARED_MODULES: usize = 32;
const SHARED_ATOMS: usize = 80;
const PRIVATE_ATOMS: usize = 25;
const PROPS: [(&str, &str); 8] = [
    ("w", "width"),
    ("h", "height"),
    ("p", "padding"),
    ("m", "margin"),
    ("minW", "min-width"),
    ("minH", "min-height"),
    ("maxW", "max-width"),
    ("maxH", "max-height"),
];

struct Module {
    path: String,
    atoms: Vec<Atom>,
}

struct Atom {
    property: (&'static str, &'static str),
    value: String,
}

impl Module {
    fn source(&self) -> String {
        let mut elements = String::new();
        for atom in &self.atoms {
            elements.push_str("<Box ");
            elements.push_str(atom.property.0);
            elements.push_str("=\"");
            elements.push_str(&atom.value);
            elements.push_str("\" />");
        }
        format!("import {{ Box }} from '@devup-ui/react'; const x = <>{elements}</>;")
    }

    fn declarations(&self) -> Vec<String> {
        self.atoms
            .iter()
            .map(|atom| format!("{}:{}", atom.property.1, atom.value))
            .collect()
    }
}

fn fixture() -> Vec<Module> {
    let atoms = |count, start| {
        (0..count)
            .map(|i| Atom {
                property: PROPS[i % PROPS.len()],
                value: format!("{}px", start + i),
            })
            .collect()
    };
    let mut modules: Vec<Module> = (0..SHARED_MODULES)
        .map(|i| Module {
            path: format!("src/shared/s{i:02}.tsx"),
            atoms: atoms(SHARED_ATOMS, 100_000),
        })
        .collect();
    modules.extend((0..ROUTES).map(|route| Module {
        path: format!("src/routes/r{route}.tsx"),
        atoms: atoms(PRIVATE_ATOMS, 1_000_000 + route * PRIVATE_ATOMS),
    }));
    modules.push(Module {
        path: "src/unknown.tsx".to_string(),
        atoms: atoms(1, 2_000_000),
    });
    modules
}

#[derive(Debug)]
pub(super) struct ExtractionError {
    file: String,
    message: String,
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.file, self.message)
    }
}

impl std::error::Error for ExtractionError {}

struct BuildState;

impl Drop for BuildState {
    fn drop(&mut self) {
        reset_build_state_internal();
    }
}

struct Measurement {
    before: Snapshot,
    after: Snapshot,
}

fn build(modules: &[Module], hoist: bool) -> Result<Snapshot, ExtractionError> {
    reset_build_state_internal();
    let _state = BuildState;
    register_theme_internal(sheet::theme::Theme::default());
    seed_file_map(modules.iter().map(|m| m.path.clone()).collect());
    let all_routes: HashSet<u32> = ROUTE_IDS.into_iter().collect();
    let mut reach: HashMap<String, HashSet<u32>> = modules[..SHARED_MODULES]
        .iter()
        .map(|m| (m.path.clone(), all_routes.clone()))
        .collect();
    reach.extend(ROUTE_IDS.iter().enumerate().map(|(r, id)| {
        (
            modules[SHARED_MODULES + r].path.clone(),
            HashSet::from([*id]),
        )
    }));
    super::import_file_routes_internal(reach);
    set_atom_hoist(hoist.then_some(2));
    let code = modules
        .iter()
        .map(|module| {
            code_extract_internal(
                &module.path,
                &module.source(),
                "@devup-ui/react",
                "df".to_string(),
                false,
                false,
                true,
                HashMap::new(),
            )
            .map(|output| output.code())
            .map_err(|message| ExtractionError {
                file: module.path.clone(),
                message,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Snapshot::capture(modules, &code))
}

fn measure(hoist: bool) -> Result<Measurement, ExtractionError> {
    // Given: every shared module reaches all routes before any extraction.
    let mut modules = fixture();
    let before = build(&modules, hoist)?;
    before.assert_loaded_definitions(&modules);
    let copies = if hoist { 1 } else { SHARED_MODULES };
    for declaration in modules[0].declarations() {
        let needle = format!("{{{declaration}}}");
        assert_eq!(
            before
                .sheets
                .iter()
                .map(|s| s.matches(&needle).count())
                .sum::<usize>(),
            copies
        );
        assert_eq!(
            before.sheets[0].matches(&needle).count(),
            usize::from(hoist)
        );
    }
    // When: deploy a fresh complete build after changing one upstream constant.
    for module in &mut modules[..SHARED_MODULES] {
        module.atoms[0].value = "900000px".to_string();
    }
    let after = build(&modules, hoist)?;
    // Then: fresh transformed code still loads the exact new declarations.
    after.assert_loaded_definitions(&modules);
    assert_eq!(after.changed(&before).0, copies);
    assert_eq!(after.sheets[0] != before.sheets[0], hoist);
    assert_eq!(
        after
            .sheets
            .iter()
            .map(|s| s.matches("{width:900000px}").count())
            .sum::<usize>(),
        copies
    );
    assert_eq!(
        after
            .sheets
            .iter()
            .map(|s| s.matches("{width:100000px}").count())
            .sum::<usize>(),
        0
    );
    for (i, module) in modules.iter().enumerate().skip(SHARED_MODULES) {
        for declaration in module.declarations() {
            assert!(after.sheets[i + 1].contains(&format!("{{{declaration}}}")));
            assert!(!after.sheets[0].contains(&format!("{{{declaration}}}")));
        }
    }
    Ok(Measurement { before, after })
}

fn checked_measurements() -> Result<(Measurement, Measurement), ExtractionError> {
    let baseline = measure(false)?;
    let atom = measure(true)?;
    let redundant_payload = (SHARED_MODULES - 1)
        * fixture()[0]
            .declarations()
            .iter()
            .map(String::len)
            .sum::<usize>();
    // Retain at least one route's share of eliminated declaration payload after
    // paying the ACTUAL lossless-name/header/import costs. No fixed percentage.
    assert!(baseline.before.raw_bytes() >= atom.before.raw_bytes() + redundant_payload / ROUTES);
    let baseline_changed = baseline.after.changed(&baseline.before);
    let atom_changed = atom.after.changed(&atom.before);
    assert_eq!(baseline_changed.0 - atom_changed.0, SHARED_MODULES - 1);
    assert!(baseline_changed.1 >= atom_changed.1 + (SHARED_MODULES - 1) * "width:900000px".len());
    Ok((baseline, atom))
}

/// Callable replacement body; the caller must retain its serial-test guard.
pub(super) fn assert_predeclared_shared_savings() -> Result<(), ExtractionError> {
    let artifacts = build_predeclared_measurement_artifacts()?;
    report::assert_artifacts(&artifacts);
    println!("{}", artifacts["report"]);
    Ok(())
}

/// Build complete artifacts independently of the caller's stdout/env policy.
/// The caller must retain its serial-test guard.
pub(super) fn build_predeclared_measurement_artifacts() -> Result<serde_json::Value, ExtractionError>
{
    let (baseline, atom) = checked_measurements()?;
    Ok(report::artifacts(&baseline, &atom))
}
