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
