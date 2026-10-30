//! Rust-owned evaluated rules; indices keep capture, cloning and destruction stack-safe.
#[derive(Debug, Clone)]
pub(crate) enum Node {
    Scalar(usize),
    Array(Vec<usize>),
    Object(Vec<Property>),
}

#[derive(Debug, Clone)]
pub(crate) struct Property {
    pub name: String,
    pub slot: usize,
    pub value: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Plan {
    pub nodes: Vec<Node>,
    pub slots: Vec<String>,
    pub roots: Vec<usize>,
    pub deep: bool,
    pub composing: bool,
    pub template: Option<Vec<String>>,
    pub origin: Option<(String, String)>,
}

impl Plan {
    pub fn needs_bridge(&self) -> bool {
        self.deep
            || (self.template.is_some()
                && self
                    .roots
                    .iter()
                    .any(|id| matches!(self.nodes[*id], Node::Array(_) | Node::Object(_))))
    }
    pub fn code(&self, root: usize) -> String {
        enum Step<'a> {
            Node(usize),
            Text(&'a str),
            Key(&'a Property),
        }
        let mut pending = vec![Step::Node(root)];
        let mut code = String::new();
        while let Some(step) = pending.pop() {
            match step {
                Step::Text(text) => code.push_str(text),
                Step::Key(key) => {
                    let expression = &self.slots[key.slot];
                    if expression == &crate::vanilla_extract::json_string(&key.name) {
                        code.push_str(expression);
                    } else {
                        code.push('[');
                        code.push_str(expression);
                        code.push(']');
                    }
                    code.push(':');
                }
                Step::Node(id) => match &self.nodes[id] {
                    Node::Scalar(slot) => code.push_str(&self.slots[*slot]),
                    Node::Array(items) => {
                        code.push('[');
                        pending.push(Step::Text("]"));
                        for (index, &item) in items.iter().enumerate().rev() {
                            pending.push(Step::Node(item));
                            if index > 0 {
                                pending.push(Step::Text(","));
                            }
                        }
                    }
                    Node::Object(properties) => {
                        code.push('{');
                        pending.push(Step::Text("}"));
                        for (index, key) in properties.iter().enumerate().rev() {
                            pending.push(Step::Node(key.value));
                            pending.push(Step::Key(key));
                            if index > 0 {
                                pending.push(Step::Text(","));
                            }
                        }
                    }
                },
            }
        }
        code
    }

    pub fn bounded_call(&self) -> String {
        let keys: rustc_hash::FxHashSet<_> = self
            .nodes
            .iter()
            .filter_map(|node| match node {
                Node::Object(properties) => Some(properties),
                Node::Scalar(_) | Node::Array(_) => None,
            })
            .flatten()
            .map(|p| p.slot)
            .collect();
        let slots: Vec<_> = self
            .slots
            .iter()
            .enumerate()
            .map(|(index, code)| {
                if keys.contains(&index) {
                    format!("({{[{code}]:0}})")
                } else {
                    code.clone()
                }
            })
            .collect();
        format!("css([{}])", slots.join(","))
    }
}
