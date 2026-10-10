use super::{
    EvidenceError,
    raw::{Object, Raw},
    wire::{Wire, tagged, variant, wire_named_enum},
};
use crate::{StyleSheetCss, counter_evidence::RecordFootprint};

wire_named_enum!(RecordFootprint {
    Property { bucket, order, level, record },
    Keyframes { bucket, name, steps },
    Css { source, css },
    Import { source, url },
    FontFace { source, properties },
    GlobalCssOwner { source },
});

impl Wire for StyleSheetCss {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let mut object = Object::read(raw)?;
        let css = object.take("css")?;
        object.finish()?;
        Ok(Self { css })
    }
    fn write(&self) -> Raw {
        Raw::Object(Object(vec![("css".into(), self.css.write())]))
    }
}
