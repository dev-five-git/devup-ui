use std::collections::BTreeMap;

/// A whole observation dominates only the subtree it observes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Demand {
    pub whole: bool,
    pub members: BTreeMap<String, Self>,
}

impl Demand {
    pub(crate) const fn whole() -> Self {
        Self {
            whole: true,
            members: BTreeMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, path: &[String]) -> bool {
        if self.whole {
            return false;
        }
        match path.split_first() {
            None => {
                self.whole = true;
                self.members.clear();
                true
            }
            Some((name, rest)) => self.members.entry(name.clone()).or_default().insert(rest),
        }
    }

    pub(crate) fn merge(&mut self, other: &Self) -> bool {
        if self.whole {
            return false;
        }
        if other.whole {
            return self.insert(&[]);
        }
        let mut changed = false;
        for (name, child) in &other.members {
            changed |= self.members.entry(name.clone()).or_default().merge(child);
        }
        changed
    }

    pub(crate) fn child(&self, name: &str) -> Option<&Self> {
        if self.whole {
            Some(self)
        } else {
            self.members.get(name)
        }
    }

    pub(crate) fn prefixed(name: &str, child: &Self) -> Self {
        Self {
            whole: false,
            members: BTreeMap::from([(name.to_string(), child.clone())]),
        }
    }
}
