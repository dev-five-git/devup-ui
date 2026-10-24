use crate::{
    ExtractStyleProp, ExtractStyleValue,
    composition::{KnownPart, KnownStyles},
    utils::unwrap_syntax_only,
};
use oxc_ast::ast::Expression;

#[derive(Clone, Copy)]
pub(super) enum PartContext<'s> {
    Classes,
    ComposedCssProp(&'s str),
    StandaloneCssPropObject(&'s str),
}

impl<'s> PartContext<'s> {
    pub(super) const fn element(self) -> Option<&'s str> {
        match self {
            Self::Classes => None,
            Self::ComposedCssProp(element) | Self::StandaloneCssPropObject(element) => {
                Some(element)
            }
        }
    }

    pub(super) const fn retains_raw(self) -> bool {
        match self {
            Self::StandaloneCssPropObject(_) => true,
            Self::Classes | Self::ComposedCssProp(_) => false,
        }
    }
}

pub(super) struct CssPropComposition<'e, 'p, 'a> {
    pub element: &'e str,
    pub earlier: &'p [ExtractStyleProp<'a>],
    pub covering: &'p [ExtractStyleValue],
    pub parts: &'p [KnownPart<'a>],
}

impl<'e> CssPropComposition<'e, '_, '_> {
    pub(super) fn part_context(self) -> PartContext<'e> {
        match (self.earlier, self.covering, self.parts) {
            ([], [], [KnownPart::Styles(side)]) => match side.as_slice() {
                [KnownStyles::Rules(rules)] => match unwrap_syntax_only(rules) {
                    Expression::ObjectExpression(_) => {
                        PartContext::StandaloneCssPropObject(self.element)
                    }
                    _ => PartContext::ComposedCssProp(self.element),
                },
                _ => PartContext::ComposedCssProp(self.element),
            },
            _ => PartContext::ComposedCssProp(self.element),
        }
    }
}
