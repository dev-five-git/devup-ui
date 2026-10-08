use super::{Measurement, PRIVATE_ATOMS, ROUTES, SHARED_ATOMS, SHARED_MODULES, Snapshot, fixture};
use serde_json::{Value, json};

fn report(baseline: &Measurement, atom: &Measurement) -> Value {
    json!({
        "invalidationModel": "fresh deployment builds; exact stylesheet content comparison",
        "fixture": {"routes": ROUTES, "sharedModules": SHARED_MODULES,
            "sharedAtoms": SHARED_ATOMS, "privateAtomsPerRoute": PRIVATE_ATOMS,
            "unknownLocalModules": 1, "threshold": 2},
        "sessionRaw": {"baseline": baseline.before.raw_bytes(), "atom": atom.before.raw_bytes()},
        "invalidationRaw": {"baseline": baseline.after.changed(&baseline.before).1,
            "atom": atom.after.changed(&atom.before).1},
        "changedSheets": {"baseline": baseline.after.changed(&baseline.before).0,
            "atom": atom.after.changed(&atom.before).0},
        "sheetCountSavings": {"numerator": SHARED_MODULES - 1, "denominator": SHARED_MODULES},
        "jsxClassNameRawBytes": {"baseline": baseline.before.name_bytes(), "atom": atom.before.name_bytes()},
        "cssSelectorNameRawBytes": {"baseline": baseline.before.selector_name_bytes(),
            "atom": atom.before.selector_name_bytes()},
        "gzip": "external measurement required; no compression dependency added"
    })
}

pub(super) fn artifacts(baseline: &Measurement, atom: &Measurement) -> Value {
    json!({
        "report": report(baseline, atom),
        "baseline": {"beforeSheets": baseline.before.sheets, "afterSheets": baseline.after.sheets,
            "beforeCode": baseline.before.code, "afterCode": baseline.after.code},
        "atom": {"beforeSheets": atom.before.sheets, "afterSheets": atom.after.sheets,
            "beforeCode": atom.before.code, "afterCode": atom.after.code},
        "paths": fixture().iter().map(|m| &m.path).collect::<Vec<_>>()
    })
}

pub(super) fn assert_artifacts(artifacts: &Value) {
    // Given: the original fixture, independently of the exported JSON.
    let modules = fixture();
    // When: consume the complete stdout representation as an external tool does.
    let serialized = artifacts.to_string();
    let decoded: Value =
        serde_json::from_str(&serialized).unwrap_or_else(|error| panic!("{error}"));
    // Then: paths, code and sheets retain their fixture identity and deployment.
    let paths: Vec<String> =
        serde_json::from_value(decoded["paths"].clone()).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        paths,
        modules.iter().map(|m| m.path.clone()).collect::<Vec<_>>()
    );
    let summary = &decoded["report"];
    assert_eq!(
        summary["fixture"],
        json!({"routes": ROUTES, "sharedModules": SHARED_MODULES,
        "sharedAtoms": SHARED_ATOMS, "privateAtomsPerRoute": PRIVATE_ATOMS,
        "unknownLocalModules": 1, "threshold": 2})
    );
    assert_eq!(
        summary["sheetCountSavings"],
        json!({"numerator": SHARED_MODULES - 1,
        "denominator": SHARED_MODULES})
    );
    for (name, copies) in [("baseline", SHARED_MODULES), ("atom", 1)] {
        let mut modules = fixture();
        let before = Snapshot {
            sheets: serde_json::from_value(decoded[name]["beforeSheets"].clone())
                .unwrap_or_else(|error| panic!("{error}")),
            code: serde_json::from_value(decoded[name]["beforeCode"].clone())
                .unwrap_or_else(|error| panic!("{error}")),
        };
        let after = Snapshot {
            sheets: serde_json::from_value(decoded[name]["afterSheets"].clone())
                .unwrap_or_else(|error| panic!("{error}")),
            code: serde_json::from_value(decoded[name]["afterCode"].clone())
                .unwrap_or_else(|error| panic!("{error}")),
        };
        assert_eq!(before.sheets.len(), modules.len() + 1);
        assert_eq!(before.code.len(), modules.len());
        before.assert_loaded_definitions(&modules);
        for module in &mut modules[..SHARED_MODULES] {
            module.atoms[0].value = "900000px".to_string();
        }
        assert_eq!(after.sheets.len(), modules.len() + 1);
        assert_eq!(after.code.len(), modules.len());
        after.assert_loaded_definitions(&modules);
        assert_eq!(
            before.sheets[0].contains("{width:100000px}"),
            name == "atom"
        );
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
        assert_eq!(after.changed(&before).0, copies);
        assert_eq!(summary["sessionRaw"][name], json!(before.raw_bytes()));
        assert_eq!(
            summary["invalidationRaw"][name],
            json!(after.changed(&before).1)
        );
        assert_eq!(summary["changedSheets"][name], json!(copies));
        assert_eq!(
            summary["jsxClassNameRawBytes"][name],
            json!(before.name_bytes())
        );
        assert_eq!(
            summary["cssSelectorNameRawBytes"][name],
            json!(before.selector_name_bytes())
        );
    }
}
