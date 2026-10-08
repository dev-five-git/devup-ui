use oxc_ast::ast::ObjectExpression;

mod declarations;
mod identity;
mod parse;
mod values;

pub(crate) use declarations::DeclarationReader;
pub(crate) use identity::{IdentityDomain, name as content_name};
pub(crate) use values::css_value;

pub(crate) enum DeclarationValue {
    Omitted,
    Scalar(String),
}

/// The two standalone `StyleX` rule compilers, with separate identity domains.
#[derive(Clone, Copy)]
pub(crate) enum TransitionApi {
    Position,
    View,
}

impl TransitionApi {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Position => "stylex.positionTry",
            Self::View => "stylex.viewTransitionClass",
        }
    }
}

/// A validated view-transition target, rather than an arbitrary CSS selector.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Group,
    ImagePair,
    Old,
    New,
}

impl Slot {
    const fn name(self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::ImagePair => "imagePair",
            Self::Old => "old",
            Self::New => "new",
        }
    }
    const fn selector(self) -> &'static str {
        match self {
            Self::Group => "::view-transition-group",
            Self::ImagePair => "::view-transition-image-pair",
            Self::Old => "::view-transition-old",
            Self::New => "::view-transition-new",
        }
    }
}

/// Rule content whose ordering matches the standalone upstream serializers.
pub(crate) enum TransitionRules {
    Position(String),
    View(Vec<(Slot, String)>),
}

impl TransitionRules {
    pub(crate) fn parse(
        api: TransitionApi,
        object: &ObjectExpression<'_>,
        reader: &DeclarationReader<'_>,
    ) -> Result<Self, Vec<(u32, String)>> {
        parse::rules(api, object, reader)
    }

    pub(crate) fn compile(&self, filename: &str) -> (String, String) {
        let (api, content) = match self {
            Self::Position(content) => (TransitionApi::Position, content.clone()),
            Self::View(slots) => {
                let mut content = String::new();
                for (slot, declarations) in slots {
                    content.push_str(slot.selector());
                    content.push(':');
                    content.push_str(declarations);
                    content.push(';');
                }
                (TransitionApi::View, content)
            }
        };
        let domain = match api {
            TransitionApi::Position => IdentityDomain::Position,
            TransitionApi::View => IdentityDomain::View,
        };
        let name = content_name(filename, domain, &content);
        let css = match self {
            Self::Position(declarations) => format!("@position-try {name}{{{declarations}}}"),
            Self::View(slots) => {
                let mut css = String::new();
                for (slot, declarations) in slots {
                    css.push_str(slot.selector());
                    css.push_str("(*.");
                    css.push_str(&name);
                    css.push_str("){");
                    css.push_str(declarations);
                    css.push('}');
                }
                css
            }
        };
        (name, css)
    }
}
