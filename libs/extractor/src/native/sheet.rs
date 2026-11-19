//! The styles of a file, hoisted into one `StyleSheet.create`

use super::props::Declaration;

/// The static styles of a file, each declared once under a short name
pub(super) struct Sheet {
    /// The name the file's stylesheet constant takes
    name: String,
    /// The code of each style object, in the order they are first used
    objects: Vec<String>,
}

impl Sheet {
    pub(super) const fn new(name: String) -> Self {
        Self {
            name,
            objects: Vec::new(),
        }
    }

    /// The code reading the style object of `declarations` from the sheet
    pub(super) fn reference(&mut self, declarations: &[Declaration]) -> String {
        let object = object_code(declarations);
        let index = self
            .objects
            .iter()
            .position(|existing| *existing == object)
            .unwrap_or_else(|| {
                self.objects.push(object);
                self.objects.len() - 1
            });
        format!("{}.{}", self.name, key_name(index))
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// The declaration of the sheet, created by `style_sheet`, the local name
    /// of React Native's `StyleSheet`
    pub(super) fn declaration(&self, style_sheet: &str) -> String {
        let objects: Vec<String> = self
            .objects
            .iter()
            .enumerate()
            .map(|(index, object)| format!("{}: {object}", key_name(index)))
            .collect();
        format!(
            "const {} = {style_sheet}.create({{ {} }});",
            self.name,
            objects.join(", ")
        )
    }
}

/// The code of the style object `declarations` declare
fn object_code(declarations: &[Declaration]) -> String {
    let properties: Vec<String> = declarations
        .iter()
        .map(|(key, value)| format!("{key}: {}", value.code()))
        .collect();
    format!("{{ {} }}", properties.join(", "))
}

/// The key of the style object at `index`: `a` to `z`, then `aa`, `ab`, ...
pub(super) fn key_name(index: usize) -> String {
    let mut name = Vec::new();
    let mut rest = index + 1;
    while rest > 0 {
        rest -= 1;
        name.push(b'a' + (rest % 26) as u8);
        rest /= 26;
    }
    name.reverse();
    String::from_utf8_lossy(&name).into_owned()
}
