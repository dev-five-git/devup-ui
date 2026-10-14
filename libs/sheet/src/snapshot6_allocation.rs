use super::{
    EvidenceError,
    raw::{Object, Raw},
    wire::{Wire, tagged, variant, wire_struct},
};
use crate::counter_evidence::AllocationEvidence;
use css::{
    allocation_input::{
        AllocationContext, AllocationFile, CapturedDelivery, CapturedNameConfig, LegacyDeclaration,
        LegacyInput, LegacyVariable, NameMode,
    },
    counter_names::{AllocatedName, NameAddress},
};

wire_struct!(LegacyDeclaration { property => "property", level => "level", value => "value", selector => "selector", order => "order" });
wire_struct!(LegacyVariable { property => "property", level => "level", selector => "selector" });
wire_struct!(CapturedNameConfig { prefix => "prefix", mode => "mode" });
wire_struct!(CapturedDelivery { canonical => "canonical", hoisted => "hoisted" });
wire_struct!(AllocationContext { config => "config", file => "file", delivery => "delivery" });
wire_struct!(AllocatedName { name => "name", address => "address" });
wire_struct!(AllocationEvidence { input => "input", context => "context", allocation => "allocation" });

impl Wire for NameMode {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        match String::read(raw)?.as_str() {
            "Counter" => Ok(Self::Counter),
            "Debug" => Ok(Self::Debug),
            "AtomHoist" => Ok(Self::AtomHoist),
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::Counter => "Counter",
            Self::Debug => "Debug",
            Self::AtomHoist => "AtomHoist",
        }
        .to_string()
        .write()
    }
}
impl Wire for LegacyInput {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let (name, payload) = variant(raw)?;
        match name.as_str() {
            "Declaration" => LegacyDeclaration::read(payload).map(Self::Declaration),
            "Keyframes" => String::read(payload).map(Self::Keyframes),
            "Variable" => LegacyVariable::read(payload).map(Self::Variable),
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::Declaration(value) => tagged("Declaration", value.write()),
            Self::Keyframes(value) => tagged("Keyframes", value.write()),
            Self::Variable(value) => tagged("Variable", value.write()),
        }
    }
}
impl Wire for AllocationFile {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let (name, payload) = variant(raw)?;
        match name.as_str() {
            "Original" => u32::read(payload).map(Self::Original),
            "Legacy" => {
                let mut object = Object::read(payload)?;
                let value = Self::Legacy {
                    filename: object.take("filename")?,
                    ordinal: object.take("ordinal")?,
                };
                object.finish()?;
                Ok(value)
            }
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::Original(id) => tagged("Original", id.write()),
            Self::Legacy { filename, ordinal } => tagged(
                "Legacy",
                Raw::Object(Object(vec![
                    ("filename".into(), filename.write()),
                    ("ordinal".into(), ordinal.write()),
                ])),
            ),
        }
    }
}
impl Wire for NameAddress {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let (name, payload) = variant(raw)?;
        let mut object = Object::read(payload)?;
        let value = match name.as_str() {
            "Counter" => Self::Counter {
                namespace: object.take("namespace")?,
                legacy_key: object.take("legacy_key")?,
                slot: object.take("slot")?,
            },
            "Baseline" => Self::Baseline {
                mode: object.take("mode")?,
                input: object.take("input")?,
                context: object.take("context")?,
            },
            _ => return Err(EvidenceError::Schema),
        };
        object.finish()?;
        Ok(value)
    }
    fn write(&self) -> Raw {
        match self {
            Self::Counter {
                namespace,
                legacy_key,
                slot,
            } => tagged(
                "Counter",
                Raw::Object(Object(vec![
                    ("namespace".into(), namespace.write()),
                    ("legacy_key".into(), legacy_key.write()),
                    ("slot".into(), slot.write()),
                ])),
            ),
            Self::Baseline {
                mode,
                input,
                context,
            } => tagged(
                "Baseline",
                Raw::Object(Object(vec![
                    ("mode".into(), mode.write()),
                    ("input".into(), input.write()),
                    ("context".into(), context.write()),
                ])),
            ),
        }
    }
}
