//! A marker lives only as long as its particular native data guard.

use std::{cell::RefCell, marker::PhantomData, rc::Rc};

thread_local! {
    static HELD: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

pub(crate) struct RootHeld {
    root: &'static str,
    _thread: PhantomData<Rc<()>>,
}

impl RootHeld {
    pub(crate) fn enter(root: &'static str) -> Self {
        HELD.with(|held| {
            let mut held = held.borrow_mut();
            assert!(!held.contains(&root), "{root}: a data root is already held");
            held.push(root);
        });
        Self {
            root,
            _thread: PhantomData,
        }
    }
}

impl Drop for RootHeld {
    fn drop(&mut self) {
        HELD.with(|held| held.borrow_mut().retain(|root| *root != self.root));
    }
}
