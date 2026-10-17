use super::{Constant, FxHashMap, Rc};

pub(super) enum Exports {
    Partial(Rc<FxHashMap<String, Constant>>),
    Ignored,
}

impl Exports {
    pub(super) fn named(&self, name: &str) -> Option<Constant> {
        match self {
            Self::Partial(exports) => exports.get(name).cloned(),
            Self::Ignored => Some(if name == "default" {
                Constant::IgnoredObject
            } else {
                Constant::Undefined
            }),
        }
    }

    pub(super) fn namespace(self) -> Constant {
        match self {
            Self::Partial(exports) => Constant::Object(exports),
            Self::Ignored => Constant::IgnoredObject,
        }
    }

    pub(super) fn entries(&self) -> impl Iterator<Item = (&String, &Constant)> {
        let entries = match self {
            Self::Partial(exports) => Some(exports),
            Self::Ignored => None,
        };
        entries.into_iter().flat_map(|exports| exports.iter())
    }
}
