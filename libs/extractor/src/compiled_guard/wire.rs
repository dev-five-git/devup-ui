use super::text::Text;
use rustc_hash::FxHashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Origin {
    pub request: Text,
    pub ids: Vec<Text>,
    pub require: bool,
}

impl Origin {
    fn fields(&self, out: &mut String) {
        out.push_str("\"request\":");
        out.push_str(self.request.json());
        out.push_str(",\"ids\":");
        strings(out, &self.ids);
        if self.require {
            out.push_str(",\"kind\":\"require-call\"");
        }
    }
}

#[derive(Default)]
pub(super) struct Facts {
    pub references: Vec<(Origin, usize)>,
    pub exports: Vec<(Text, Option<Origin>)>,
    pub stars: Vec<Text>,
    pub observation: Option<Observation>,
    export_indices: FxHashMap<Text, usize>,
}

#[derive(Default)]
pub(super) struct Observation {
    pub default_callable: bool,
    pub unsupported_requests: Vec<Text>,
    pub import_only_requests: Option<Vec<Text>>,
}

impl Facts {
    pub fn new(observe: bool) -> Self {
        Self {
            observation: observe.then(Observation::default),
            ..Self::default()
        }
    }

    pub fn export(&mut self, name: Text, origin: Option<Origin>) {
        if let Some(index) = self.export_indices.get(&name) {
            self.exports[*index].1 = origin;
        } else {
            self.export_indices.insert(name.clone(), self.exports.len());
            self.exports.push((name, origin));
        }
    }

    pub fn json(&self) -> String {
        let mut out = String::from("{\"references\":[");
        for (index, (origin, pos)) in self.references.iter().enumerate() {
            separator(&mut out, index);
            out.push('{');
            origin.fields(&mut out);
            out.push_str(",\"pos\":");
            out.push_str(&pos.to_string());
            out.push('}');
        }
        out.push_str("],\"exports\":[");
        for (index, (name, origin)) in self.exports.iter().enumerate() {
            separator(&mut out, index);
            out.push('[');
            out.push_str(name.json());
            out.push(',');
            match origin {
                Some(origin) => {
                    out.push('{');
                    origin.fields(&mut out);
                    out.push('}');
                }
                None => out.push_str("null"),
            }
            out.push(']');
        }
        out.push_str("],\"stars\":");
        strings(&mut out, &self.stars);
        if let Some(observation) = &self.observation {
            out.push_str(",\"observation\":{\"defaultCallable\":");
            out.push_str(if observation.default_callable {
                "true"
            } else {
                "false"
            });
            out.push_str(",\"unsupportedRequests\":");
            strings(&mut out, &observation.unsupported_requests);
            if let Some(requests) = &observation.import_only_requests {
                out.push_str(",\"importOnlyRequests\":");
                strings(&mut out, requests);
            }
            out.push('}');
        }
        out.push('}');
        out
    }
}

fn separator(out: &mut String, index: usize) {
    if index > 0 {
        out.push(',');
    }
}

fn strings(out: &mut String, values: &[Text]) {
    out.push('[');
    for (index, text) in values.iter().enumerate() {
        separator(out, index);
        out.push_str(text.json());
    }
    out.push(']');
}

/// Sparse corrections keep ASCII modules allocation-free for offset conversion.
pub(super) struct Positions {
    corrections: Vec<(usize, usize)>,
}

impl Positions {
    pub fn new(code: &str) -> Self {
        let mut corrections = Vec::new();
        let mut excess = 0;
        for (byte, ch) in code.char_indices().filter(|(_, ch)| !ch.is_ascii()) {
            excess += ch.len_utf8() - ch.len_utf16();
            corrections.push((byte + ch.len_utf8(), excess));
        }
        Self { corrections }
    }

    pub fn utf16(&self, byte: u32) -> usize {
        let byte = usize::try_from(byte).unwrap_or(usize::MAX);
        let index = self.corrections.partition_point(|(end, _)| *end <= byte);
        let excess = index
            .checked_sub(1)
            .and_then(|at| self.corrections.get(at))
            .map_or(0, |(_, excess)| *excess);
        byte - excess
    }
}

pub(super) fn insert_request(requests: &mut Vec<Text>, request: Text) {
    if !requests.contains(&request) {
        requests.push(request);
    }
}
