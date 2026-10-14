use super::super::{BatchPhase, Candidate, Cleanup, Lineage, VariableLineage, state::CounterState};
use super::{
    EvidenceError,
    raw::{Object, Raw},
    wire::{Wire, tagged, unique, variant, wire_struct},
};
use crate::counter_evidence::{EmissionWitness, Expansion, ExpansionProof, Materialization};

wire_struct!(EmissionWitness { name => "name", seed => "seed", expansion => "expansion", materialization => "materialization" });
wire_struct!(ExpansionProof { allocation => "allocation", emission => "emission" });
wire_struct!(Candidate { proof => "proof", lineage => "lineage" });
wire_struct!(Lineage { parent => "parent", children => "children", variable => "variable" });
wire_struct!(VariableLineage { original => "original", evidence => "evidence" });
wire_struct!(Cleanup { source => "source", bucket => "bucket", single_css => "single_css" });

impl Wire for Materialization {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let (name, payload) = variant(raw)?;
        match (name.as_str(), payload) {
            ("Complete", Raw::Null) => Ok(Self::Complete),
            ("AfterGlobalCleanup", payload) => {
                let mut object = Object::read(payload)?;
                let value = Self::AfterGlobalCleanup {
                    source: object.take("source")?,
                    bucket: object.take("bucket")?,
                };
                object.finish()?;
                Ok(value)
            }
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::Complete => "Complete".to_string().write(),
            Self::AfterGlobalCleanup { source, bucket } => tagged(
                "AfterGlobalCleanup",
                Raw::Object(Object(vec![
                    ("source".into(), source.write()),
                    ("bucket".into(), bucket.write()),
                ])),
            ),
        }
    }
}
impl Wire for Expansion {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let (name, payload) = variant(raw)?;
        if name == "Static" {
            return unique(Vec::read(payload)?).map(Self::Static);
        }
        let mut object = Object::read(payload)?;
        let value = match name.as_str() {
            "Dynamic" => Self::Dynamic {
                variable: object.take("variable")?,
                site: object.take("site")?,
                important: object.take("important")?,
                consumer: object.take("consumer")?,
                reset: object.take("reset")?,
            },
            "Typography" => Self::Typography {
                preset: object.take("preset")?,
                yielded: object.take("yielded")?,
                members: unique(object.take("members")?)?,
            },
            "Keyframes" => Self::Keyframes {
                steps: object.take("steps")?,
                record: object.take("record")?,
            },
            _ => return Err(EvidenceError::Schema),
        };
        object.finish()?;
        unique(value.records())?;
        Ok(value)
    }
    fn write(&self) -> Raw {
        match self {
            Self::Static(members) => tagged("Static", members.write()),
            Self::Dynamic {
                variable,
                site,
                important,
                consumer,
                reset,
            } => tagged(
                "Dynamic",
                Raw::Object(Object(vec![
                    ("variable".into(), variable.write()),
                    ("site".into(), site.write()),
                    ("important".into(), important.write()),
                    ("consumer".into(), consumer.write()),
                    ("reset".into(), reset.write()),
                ])),
            ),
            Self::Typography {
                preset,
                yielded,
                members,
            } => tagged(
                "Typography",
                Raw::Object(Object(vec![
                    ("preset".into(), preset.write()),
                    ("yielded".into(), yielded.write()),
                    ("members".into(), members.write()),
                ])),
            ),
            Self::Keyframes { steps, record } => tagged(
                "Keyframes",
                Raw::Object(Object(vec![
                    ("steps".into(), steps.write()),
                    ("record".into(), record.write()),
                ])),
            ),
        }
    }
}
impl Wire for BatchPhase {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let (name, payload) = variant(raw)?;
        match (name.as_str(), payload) {
            ("Fresh", Raw::Null) => Ok(Self::Fresh),
            ("Retained", payload) => Wire::read(payload).map(Self::Retained),
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::Fresh => "Fresh".to_string().write(),
            Self::Retained(owners) => tagged("Retained", owners.write()),
        }
    }
}
impl Wire for CounterState {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let mut object = Object::read(raw)?;
        let state = Self {
            counters: object.take("counters")?,
            baseline: unique(object.take("baseline")?)?,
            authored: unique(object.take("authored")?)?,
            config: object.take("config")?,
            threshold: object.take("threshold")?,
            originals: object.take("originals")?,
            files: object.take("files")?,
            placements: unique(object.take("placements")?)?,
            deliveries: object.take("deliveries")?,
            cleanups: object.take("cleanups")?,
            phase: object.take("phase")?,
            rejection: None,
        };
        object.finish()?;
        unique(state.candidates().collect())?;
        Ok(state)
    }
    fn write(&self) -> Raw {
        Raw::Object(Object(vec![
            ("counters".into(), self.counters.write()),
            ("baseline".into(), self.baseline.write()),
            ("authored".into(), self.authored.write()),
            ("config".into(), self.config.write()),
            ("threshold".into(), self.threshold.write()),
            ("originals".into(), self.originals.write()),
            ("files".into(), self.files.write()),
            ("placements".into(), self.placements.write()),
            ("deliveries".into(), self.deliveries.write()),
            ("cleanups".into(), self.cleanups.write()),
            ("phase".into(), self.phase.write()),
        ]))
    }
}
