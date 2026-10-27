use std::cell::RefCell;

#[derive(Default)]
struct Journal {
    classes: Vec<(String, String, bool)>,
    files: Vec<String>,
}

thread_local! {
    static JOURNAL: RefCell<Option<Journal>> = const { RefCell::new(None) };
}

pub(crate) fn record_class(filename: &str, key: &str, new_bucket: bool) {
    JOURNAL.with_borrow_mut(|journal| {
        if let Some(journal) = journal {
            journal
                .classes
                .push((filename.into(), key.into(), new_bucket));
        }
    });
}

pub(crate) fn record_file(filename: &str) {
    JOURNAL.with_borrow_mut(|journal| {
        if let Some(journal) = journal {
            journal.files.push(filename.into());
        }
    });
}

/// Journals only new allocator keys on this synchronous extraction thread.
pub struct RowSlots {
    outer: Option<Journal>,
    committed: bool,
    thread_bound: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl Default for RowSlots {
    fn default() -> Self {
        Self {
            outer: JOURNAL.with_borrow_mut(|journal| journal.replace(Journal::default())),
            committed: false,
            thread_bound: std::marker::PhantomData,
        }
    }
}

impl RowSlots {
    /// Keep this row's additions; an enclosing transaction can still roll them back.
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for RowSlots {
    fn drop(&mut self) {
        let journal = JOURNAL.with_borrow_mut(|current| {
            std::mem::replace(current, self.outer.take()).unwrap_or_default()
        });
        if self.committed {
            JOURNAL.with_borrow_mut(|outer| {
                if let Some(outer) = outer {
                    outer.classes.extend(journal.classes);
                    outer.files.extend(journal.files);
                }
            });
        } else {
            crate::class_map::with_class_map_mut(|map| {
                for (file, key, new_bucket) in journal.classes.into_iter().rev() {
                    if new_bucket {
                        map.remove(&file);
                    } else if let Some(entries) = map.get_mut(&file) {
                        entries.remove(&key);
                    }
                }
            });
            crate::file_map::with_file_map_mut(|map| {
                for file in journal.files {
                    map.remove_by_left(&file);
                }
            });
        }
    }
}
