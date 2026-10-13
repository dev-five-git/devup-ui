use super::{Kinds, Naming, PreparedStyled, StyledDefinition};
use crate::styled_reads::Forward;
use oxc_ast::ast::Expression;

pub(crate) struct StyledInput<'s, 'a> {
    pub naming: Naming<'s>,
    pub imports: Kinds<'s>,
    pub attrs: &'s [Expression<'a>],
    pub inherited: Option<&'s StyledDefinition<'a>>,
    pub forward: Option<Forward>,
    pub prepared: Option<PreparedStyled<'a>>,
}
