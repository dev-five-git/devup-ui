use std::{borrow::Cow, collections::VecDeque};

use css::style_selector::{StyleSelector, split_selector_list};

#[derive(Default)]
pub(super) struct SelectorText<'a> {
    pieces: Vec<Cow<'a, str>>,
    order: VecDeque<usize>,
    pending: VecDeque<(usize, usize)>,
    list: bool,
    lexical_safe: bool,
}

impl<'a> SelectorText<'a> {
    fn piece(&mut self, value: Cow<'a, str>) -> usize {
        let id = self.pieces.len();
        self.pieces.push(value);
        id
    }

    pub fn nest(&mut self, template: &'a str) {
        if self.order.is_empty() {
            self.lexical_safe = true;
            self.list = split_selector_list(template).len() > 1;
            let id = self.piece(Cow::Borrowed(template));
            self.order.push_back(id);
            self.pending
                .extend(positions(template, &mut 0).into_iter().map(|at| (id, at)));
        } else if template.matches('&').count() == 1 && !self.list {
            if let Some((before, after)) = template.split_once('&') {
                if after.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-') {
                    self.lexical_safe = false;
                }
                let mut depth = 0;
                let before_positions = positions(before, &mut depth);
                if depth > 0 {
                    self.pending.clear();
                }
                if !before.is_empty() {
                    let id = self.piece(Cow::Borrowed(before));
                    self.order.push_front(id);
                    for at in before_positions.into_iter().rev() {
                        self.pending.push_front((id, at));
                    }
                }
                let after_positions = positions(after, &mut depth);
                if !after.is_empty() {
                    let id = self.piece(Cow::Borrowed(after));
                    self.order.push_back(id);
                    self.pending
                        .extend(after_positions.into_iter().map(|at| (id, at)));
                }
                self.list = split_selector_list(template).len() > 1;
            }
        } else {
            let parent = self.text().map(StyleSelector::Selector);
            let nested = StyleSelector::nest_selector(parent.as_ref(), template).to_string();
            self.list = split_selector_list(&nested).len() > 1;
            self.lexical_safe = true;
            let candidates = positions(&nested, &mut 0);
            let id = self.piece(Cow::Owned(nested));
            self.order.clear();
            self.order.push_back(id);
            self.pending = candidates.into_iter().map(|at| (id, at)).collect();
        }
    }

    pub fn params(&mut self, params: &str) {
        if !self.lexical_safe {
            if let Some(text) = self.text() {
                let text =
                    css::add_selector_params(StyleSelector::Selector(text), params).to_string();
                let candidates = positions(&text, &mut 0);
                let id = self.piece(Cow::Owned(text));
                self.order.clear();
                self.order.push_back(id);
                self.pending = candidates.into_iter().map(|at| (id, at)).collect();
                self.lexical_safe = true;
            }
            return;
        }
        let target = self
            .pending
            .pop_back()
            .or_else(|| self.order.back().map(|&id| (id, self.pieces[id].len())));
        if let Some((id, at)) = target {
            let value = self.pieces[id].to_mut();
            value.insert_str(at, &format!("({params})"));
        }
    }

    pub fn text(&self) -> Option<String> {
        (!self.order.is_empty()).then(|| {
            self.order
                .iter()
                .map(|&id| self.pieces[id].as_ref())
                .collect()
        })
    }
}

fn positions(value: &str, depth: &mut usize) -> Vec<usize> {
    let bytes = value.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'(' | b'[' => *depth += 1,
            b')' | b']' => *depth = depth.saturating_sub(1),
            b':' if *depth == 0 => {
                let start = index + 1 + usize::from(bytes.get(index + 1) == Some(&b':'));
                let end = value[start..]
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .map_or(value.len(), |length| start + length);
                if bytes.get(end) != Some(&b'(') {
                    let probe = StyleSelector::Selector(format!(":{} x", &value[start..end]));
                    if css::add_selector_params(probe, "")
                        .to_string()
                        .ends_with(" x")
                    {
                        found.push(end);
                    }
                }
                index = end;
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    found
}

#[derive(Clone, Default)]
struct Branch {
    targets: Option<usize>,
    trailing: Option<(String, bool)>,
}

enum TargetsNode {
    Item,
    Join(usize, usize),
    Unknown,
}

#[derive(Default)]
pub(crate) struct Targets {
    nodes: Vec<TargetsNode>,
    tails: Vec<std::cell::OnceCell<Option<usize>>>,
    required: Vec<Option<bool>>,
    branches: Vec<Vec<Branch>>,
}

impl Targets {
    fn item(&mut self, required: bool, parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(TargetsNode::Item);
        self.tails.push(std::cell::OnceCell::from(parent));
        self.required.push(Some(required));
        id
    }

    fn join(&mut self, left: Option<usize>, right: Option<usize>) -> Option<usize> {
        match (left, right) {
            (None, other) | (other, None) => other,
            (Some(left), Some(right)) => {
                let id = self.nodes.len();
                self.nodes.push(TargetsNode::Join(left, right));
                self.tails.push(std::cell::OnceCell::new());
                self.required.push(self.required[right]);
                Some(id)
            }
        }
    }

    fn pop(&mut self, root: Option<usize>) -> Option<usize> {
        let mut pending = Vec::new();
        let mut cursor = root?;
        let mut tail = loop {
            if let Some(tail) = self.tails[cursor].get().copied() {
                break tail;
            }
            match self.nodes[cursor] {
                TargetsNode::Join(left, right) => {
                    pending.push((cursor, left));
                    cursor = right;
                }
                TargetsNode::Item => break None,
                TargetsNode::Unknown => break Some(cursor),
            }
        };
        for (id, left) in pending.into_iter().rev() {
            tail = self.join(Some(left), tail);
            self.tails[id] = std::cell::OnceCell::from(tail);
        }
        tail
    }

    pub fn required(&self, state: Option<usize>) -> bool {
        state.is_some_and(|state| {
            self.branches[state].iter().any(|branch| {
                branch
                    .targets
                    .is_some_and(|id| self.required[id] == Some(true))
            })
        })
    }

    pub fn consume(&mut self, state: Option<usize>) -> Option<usize> {
        let mut branches = self.branches[state?].clone();
        for branch in &mut branches {
            branch.targets = self.pop(branch.targets);
            branch.trailing = None;
        }
        Some(self.save(branches))
    }

    pub fn uncertain(&mut self) -> usize {
        let id = self.nodes.len();
        self.nodes.push(TargetsNode::Unknown);
        self.tails.push(std::cell::OnceCell::from(Some(id)));
        self.required.push(None);
        self.save(vec![Branch {
            targets: Some(id),
            trailing: None,
        }])
    }

    fn save(&mut self, branches: Vec<Branch>) -> usize {
        let id = self.branches.len();
        self.branches.push(branches);
        id
    }

    pub fn nest(&mut self, state: Option<usize>, template: &str) -> usize {
        let parents = state.map_or_else(|| vec![Branch::default()], |id| self.branches[id].clone());
        let mut branches = Vec::new();
        for parent in parents {
            for part in split_selector_list(template) {
                let mut branch = Branch::default();
                let bytes = part.as_bytes();
                let mut depth = 0usize;
                let mut skip = 0;
                for (index, byte) in css::quoted::unquoted_bytes(part) {
                    if index < skip {
                        continue;
                    }
                    match byte {
                        b'(' | b'[' => depth += 1,
                        b')' | b']' => depth = depth.saturating_sub(1),
                        b'&' if depth == 0 => {
                            let mut inserted = parent.targets;
                            let mut trailing = parent.trailing.clone();
                            if let Some((name, target)) = &mut trailing {
                                let end = part[index + 1..]
                                    .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                                    .map_or(part.len(), |length| index + 1 + length);
                                if end > index + 1 || bytes.get(end) == Some(&b'(') {
                                    if *target {
                                        inserted = self.pop(inserted);
                                    }
                                    name.push_str(&part[index + 1..end]);
                                    *target = false;
                                    if bytes.get(end) != Some(&b'(')
                                        && let Some(required) = target_kind(name)
                                    {
                                        inserted = Some(self.item(required, inserted));
                                        *target = true;
                                    }
                                    skip = end;
                                }
                                if end < part.len() {
                                    trailing = None;
                                }
                            }
                            branch.targets = self.join(branch.targets, inserted);
                            branch.trailing = if skip == part.len() || index + 1 == part.len() {
                                trailing
                            } else {
                                None
                            };
                        }
                        b':' if depth == 0 => {
                            let start =
                                index + 1 + usize::from(bytes.get(index + 1) == Some(&b':'));
                            let end = part[start..]
                                .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                                .map_or(part.len(), |length| start + length);
                            let required = (bytes.get(end) != Some(&b'('))
                                .then(|| target_kind(&part[start..end]))
                                .flatten();
                            if let Some(required) = required {
                                branch.targets = Some(self.item(required, branch.targets));
                            }
                            if end == part.len() {
                                branch.trailing =
                                    Some((part[start..end].to_string(), required.is_some()));
                            }
                            skip = end;
                        }
                        _ => {}
                    }
                }
                branches.push(branch);
            }
        }
        self.save(branches)
    }
}

fn target_kind(name: &str) -> Option<bool> {
    let selector = StyleSelector::Selector(format!(":{name}"));
    css::selector_params::parameter_name(&selector)
        .map(|name| !matches!(name, "host" | "cue" | "cue-region"))
}
