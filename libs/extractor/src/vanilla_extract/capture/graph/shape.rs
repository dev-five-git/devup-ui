use super::Value;
use boa_engine::property::Attribute;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::vanilla_extract::capture) enum Kind {
    Record { null_prototype: bool },
    Array { null_prototype: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::vanilla_extract::capture) struct Property {
    pub name: String,
    pub indexed: bool,
    pub value: Value,
    pub attributes: Attribute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::vanilla_extract::capture) struct Shape {
    pub kind: Kind,
    pub properties: Vec<Property>,
    pub extensible: bool,
}

impl Shape {
    pub fn plain(&self) -> bool {
        self.extensible
            && match self.kind {
                Kind::Record { null_prototype } => {
                    !null_prototype
                        && self
                            .properties
                            .iter()
                            .all(|property| property.attributes == Attribute::all())
                }
                Kind::Array { .. } => false,
            }
    }
}
