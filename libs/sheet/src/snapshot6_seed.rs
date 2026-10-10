use super::{
    EvidenceError,
    raw::{Object, Raw},
    wire::{Wire, tagged, variant, wire_struct},
};
use crate::emission_seed::{
    DeclarationSeed, EmissionContext, EmissionInput, EmissionSeed, FrozenPreset, FrozenTypography,
    NumericSite, Resolution, Yield,
};

wire_struct!(EmissionContext { source_file => "source_file", bucket => "bucket", single_css => "single_css", hoisted => "hoisted" });
wire_struct!(DeclarationSeed {
    property => "property", value => "value", level => "level", selector => "selector",
    style_order => "style_order", layer => "layer", resolution => "resolution", first_value => "first_value", preset => "preset"
});
wire_struct!(FrozenTypography {
    font_family => "font_family", font_size => "font_size", font_style => "font_style", font_weight => "font_weight",
    line_height => "line_height", letter_spacing => "letter_spacing", text_transform => "text_transform"
});
wire_struct!(FrozenPreset { name => "name", frames => "frames" });
wire_struct!(Yield { property => "property", from => "from" });
wire_struct!(NumericSite { source => "source", at => "at", role => "role" });
wire_struct!(EmissionSeed { placement => "placement", body => "body" });

impl Wire for Resolution {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        match String::read(raw)?.as_str() {
            "CssVariable" => Ok(Self::CssVariable),
            "FirstValue" => Ok(Self::FirstValue),
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::CssVariable => "CssVariable",
            Self::FirstValue => "FirstValue",
        }
        .to_string()
        .write()
    }
}
impl Wire for EmissionInput {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let (name, payload) = variant(raw)?;
        match name.as_str() {
            "Static" => DeclarationSeed::read(payload).map(Self::Static),
            "Typography" => DeclarationSeed::read(payload).map(Self::Typography),
            "Dynamic" => {
                let mut object = Object::read(payload)?;
                let value = Self::Dynamic {
                    declaration: object.take("declaration")?,
                    variable: object.take("variable")?,
                    site: object.take("site")?,
                    important: object.take("important")?,
                };
                object.finish()?;
                Ok(value)
            }
            "Keyframes" => {
                let mut object = Object::read(payload)?;
                let steps = object.take("steps")?;
                object.finish()?;
                Ok(Self::Keyframes { steps })
            }
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::Static(value) => tagged("Static", value.write()),
            Self::Typography(value) => tagged("Typography", value.write()),
            Self::Dynamic {
                declaration,
                variable,
                site,
                important,
            } => tagged(
                "Dynamic",
                Raw::Object(Object(vec![
                    ("declaration".into(), declaration.write()),
                    ("variable".into(), variable.write()),
                    ("site".into(), site.write()),
                    ("important".into(), important.write()),
                ])),
            ),
            Self::Keyframes { steps } => tagged(
                "Keyframes",
                Raw::Object(Object(vec![("steps".into(), steps.write())])),
            ),
        }
    }
}
