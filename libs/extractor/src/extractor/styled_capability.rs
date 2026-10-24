use oxc_ast::{
    ast::{Expression, Program},
    builder::AstBuilder,
};

use super::styled_css_slot::StyledCssSlot;

/// Independently reserved complete replacements consumed by the defining renderer.
#[derive(Clone)]
pub(crate) struct StyledCapability {
    pub classes: StyledCssSlot,
    pub forwarded: StyledCssSlot,
}

impl StyledCapability {
    pub(crate) fn reserve(program: &Program<'_>) -> Self {
        Self {
            classes: StyledCssSlot::reserve(program),
            forwarded: StyledCssSlot::reserve_named(program, "__devupForwarded"),
        }
    }

    pub(crate) fn omit(&self, names: &mut Vec<String>) {
        names.extend([
            self.classes.name().to_string(),
            self.forwarded.name().to_string(),
        ]);
    }

    pub(crate) fn forwarded<'a>(
        &self,
        ast: &AstBuilder<'a>,
        ordinary: &Expression<'a>,
    ) -> Expression<'a> {
        self.forwarded.classes(ast, Some(ordinary))
    }
}
