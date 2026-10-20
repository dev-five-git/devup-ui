use super::{Module, with_style_sheet};
use std::collections::HashSet;

fn classes(code: &str) -> Vec<&str> {
    code.split("className=\"")
        .skip(1)
        .flat_map(|part| part.split('"').take(1).flat_map(str::split_whitespace))
        .collect()
}

pub(super) struct Snapshot {
    pub(super) sheets: Vec<String>,
    pub(super) code: Vec<String>,
}

impl Snapshot {
    pub(super) fn capture(modules: &[Module], code: &[String]) -> Self {
        let mut sheets = vec![with_style_sheet(|s| s.create_css(None, false))];
        sheets.extend(
            modules
                .iter()
                .map(|m| with_style_sheet(|s| s.create_css(Some(&m.path), true))),
        );
        Self {
            sheets,
            code: code.to_vec(),
        }
    }

    pub(super) fn assert_loaded_definitions(&self, modules: &[Module]) {
        for (i, module) in modules.iter().enumerate() {
            let names = classes(&self.code[i]);
            let expected = module.declarations();
            assert_eq!(
                names.len(),
                expected.len(),
                "{}: static JSX classes",
                module.path
            );
            assert!(!self.code[i].contains("<Box"));
            assert!(!self.code[i].contains("@devup-ui/react"));
            assert!(!self.code[i].contains("style="));
            let loaded = format!("{}{}", self.sheets[0], self.sheets[i + 1]);
            for (class, declaration) in names.iter().zip(&expected) {
                let rule = format!(".{class}{{{declaration}}}");
                assert!(loaded.contains(&rule), "{}: missing {rule}", module.path);
            }
        }
    }

    pub(super) fn raw_bytes(&self) -> usize {
        self.sheets.iter().map(String::len).sum()
    }

    pub(super) fn name_bytes(&self) -> usize {
        self.code
            .iter()
            .flat_map(|code| classes(code))
            .map(str::len)
            .sum()
    }

    pub(super) fn selector_name_bytes(&self) -> usize {
        let names: HashSet<&str> = self.code.iter().flat_map(|code| classes(code)).collect();
        names
            .iter()
            .map(|name| {
                name.len()
                    * self
                        .sheets
                        .iter()
                        .map(|sheet| sheet.matches(&format!(".{name}{{")).count())
                        .sum::<usize>()
            })
            .sum()
    }

    pub(super) fn changed(&self, before: &Self) -> (usize, usize) {
        let changed: Vec<&String> = self
            .sheets
            .iter()
            .zip(&before.sheets)
            .filter_map(|(after, before)| (after != before).then_some(after))
            .collect();
        (changed.len(), changed.iter().map(|sheet| sheet.len()).sum())
    }
}
