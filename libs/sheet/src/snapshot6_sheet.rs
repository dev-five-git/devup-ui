use super::{
    Authority, EvidenceError,
    raw::{Object, Raw},
    wire::{Wire, unique},
};
use crate::{StyleSheet, StyleSheetProperty};
use rustc_hash::FxHashSet;

impl Wire for FxHashSet<StyleSheetProperty> {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        unique(Vec::<StyleSheetProperty>::read(raw)?).map(|items| items.into_iter().collect())
    }
    fn write(&self) -> Raw {
        Raw::Array(self.iter().map(Wire::write).collect())
    }
}

impl Wire for Authority {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let mut object = Object::read(raw)?;
        if object.take::<u8>("atomNamingVersion")? != 6 {
            return Err(EvidenceError::Schema);
        }
        let sheet = StyleSheet {
            counter_state: Some(object.take("evidence")?),
            atom_plan: object.take("atom_plan")?,
            properties: object.take("properties")?,
            css: object.take("css")?,
            keyframes: object.take("keyframes")?,
            global_css_files: object.take("global_css_files")?,
            imports: object.take("imports")?,
            font_faces: object.take("font_faces")?,
            source_ids: object.take("sourceIds")?,
            ..StyleSheet::default()
        };
        let value = Self {
            sheet,
            classes: object.take("classMap")?,
            files: object.take("fileMap")?,
        };
        object.finish()?;
        Ok(value)
    }
    fn write(&self) -> Raw {
        let sheet = &self.sheet;
        Raw::Object(Object(vec![
            ("atomNamingVersion".into(), 6u8.write()),
            (
                "evidence".into(),
                sheet.counter_state.as_ref().map_or(Raw::Null, Wire::write),
            ),
            ("atom_plan".into(), sheet.atom_plan.write()),
            ("properties".into(), sheet.properties.write()),
            ("css".into(), sheet.css.write()),
            ("keyframes".into(), sheet.keyframes.write()),
            ("global_css_files".into(), sheet.global_css_files.write()),
            ("imports".into(), sheet.imports.write()),
            ("font_faces".into(), sheet.font_faces.write()),
            ("sourceIds".into(), sheet.source_ids.write()),
            ("classMap".into(), self.classes.write()),
            ("fileMap".into(), self.files.write()),
        ]))
    }
}
