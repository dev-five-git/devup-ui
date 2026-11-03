use rustc_hash::{FxHashMap, FxHashSet};

/// The names the visit gives what it evaluates once, with the code each stands for.
#[derive(Default)]
pub(super) struct Names {
    taken: FxHashSet<String>,
    next: FxHashMap<&'static str, usize>,
    stand_for: Vec<(String, String)>,
}

impl Names {
    pub(super) fn reserve_program(&mut self, program: &oxc_ast::ast::Program<'_>) {
        self.taken.extend(
            program
                .source_text
                .split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '$')
                .filter(|token| token.starts_with("__devup") || token.starts_with("DevupAs"))
                .map(str::to_string),
        );
        oxc_ast_visit::Visit::visit_program(self, program);
    }

    pub(super) fn reserve(&mut self, scoping: &oxc_semantic::Scoping) {
        self.taken
            .extend(scoping.symbol_names().map(str::to_string));
        self.taken.extend(
            scoping
                .root_unresolved_references()
                .keys()
                .map(|name| name.as_str().to_string()),
        );
    }

    pub(super) fn fresh(&mut self, prefix: &'static str) -> String {
        loop {
            let next = self.next.entry(prefix).or_default();
            let name = format!("{prefix}{next}");
            *next += 1;
            if self.taken.insert(name.clone()) {
                return name;
            }
        }
    }

    pub(super) fn stands_for(&mut self, name: &str, code: String) {
        self.stand_for.push((name.to_string(), code));
    }

    pub(super) fn restore(&self, message: &str) -> String {
        self.stand_for
            .iter()
            .rev()
            .fold(message.to_string(), |message, (name, code)| {
                let boundary = |ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '$';
                let mut restored = String::new();
                for chunk in message.split_inclusive(boundary) {
                    let token = chunk.trim_end_matches(boundary);
                    restored.push_str(if token == name { code } else { token });
                    restored.push_str(&chunk[token.len()..]);
                }
                restored
            })
    }
}

impl<'a> oxc_ast_visit::Visit<'a> for Names {
    fn visit_binding_identifier(&mut self, identifier: &oxc_ast::ast::BindingIdentifier<'a>) {
        self.taken.insert(identifier.name.to_string());
    }

    fn visit_identifier_reference(&mut self, identifier: &oxc_ast::ast::IdentifierReference<'a>) {
        self.taken.insert(identifier.name.to_string());
    }
}
