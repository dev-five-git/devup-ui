use crate::allocation_input::{
    AllocationContext, AllocationFile, CapturedNameConfig, LegacyDeclaration, LegacyInput,
    LegacyVariable, NameMode,
};

pub(super) fn context(mode: NameMode, file: Option<AllocationFile>) -> AllocationContext {
    AllocationContext {
        config: CapturedNameConfig {
            prefix: "app-".to_string(),
            mode,
        },
        file,
        delivery: None,
    }
}

pub(super) fn declaration(value: Option<&str>) -> LegacyInput {
    LegacyInput::Declaration(LegacyDeclaration {
        property: "color".to_string(),
        level: 0,
        value: value.map(str::to_string),
        selector: None,
        order: None,
    })
}

pub(super) fn variable() -> LegacyInput {
    LegacyInput::Variable(LegacyVariable {
        property: "color".to_string(),
        level: 0,
        selector: None,
    })
}
