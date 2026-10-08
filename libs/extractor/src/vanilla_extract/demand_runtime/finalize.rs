use super::{BTreeSet, Binding, NameScope, Rc, State, StylesheetImports, name_values};
use boa_engine::{Context, JsResult, JsValue};

impl State {
    pub(super) fn finish(
        &mut self,
        index: usize,
        array: &JsValue,
        context: &mut Context,
    ) -> JsResult<()> {
        let owner = self.owners.get(index).ok_or_else(|| {
            boa_engine::JsNativeError::range().with_message("unknown native owner")
        })?;
        if owner.finalized {
            return Err(boa_engine::JsNativeError::typ()
                .with_message("native owner initialized twice")
                .into());
        }
        let values = crate::vanilla_extract::array_items(array, context)?.ok_or_else(|| {
            boa_engine::JsNativeError::typ()
                .with_message("native terminal bindings must be an array")
        })?;
        let bindings: Vec<_> = owner
            .plan
            .bindings
            .iter()
            .map(|binding| Binding {
                name: binding.name.clone(),
                exported: true,
                read: binding.name.clone(),
                alias: None,
                init: None,
            })
            .collect();
        let named: Vec<_> = bindings.iter().zip(values.iter().cloned()).collect();
        let mut collected = std::mem::take(&mut owner.collector.borrow_mut().styles);
        name_values(
            &mut collected,
            &named,
            NameScope {
                file_num: owner.collector.borrow().file_num,
                reserved: &owner.plan.reserved,
            },
        );
        let mut required = Vec::new();
        for binding in &owner.plan.native {
            if let Some((_, value)) = owner
                .plan
                .bindings
                .iter()
                .zip(&values)
                .find(|(candidate, _)| candidate.symbol == binding.symbol)
            {
                crate::vanilla_extract::capture::terminal::Terminal::validate(std::slice::from_ref(value), context)
                    .map_err(|error| boa_engine::JsNativeError::typ().with_message(format!("{}: native result `{}` cannot be captured exactly: {error}. Fix: return data-only results and retain ordinary callable source", crate::locate(&owner.plan.filename, &owner.plan.source, usize::try_from(binding.span.start).unwrap_or(0)), binding.name)))?;
                required.push(value.clone());
            }
        }
        let material = owner
            .compile(
                &collected,
                StylesheetImports {
                    artifacts: Default::default(),
                    dependencies: BTreeSet::new(),
                    kept_imports: Vec::new(),
                    atoms: self.atoms.clone(),
                    references: self.references.clone(),
                },
            )
            .map_err(|error| boa_engine::JsNativeError::typ().with_message(error))?;
        self.terminal.names.extend(material.values);
        self.terminal.materialize(&required, context)
            .map_err(|error| boa_engine::JsNativeError::typ().with_message(format!("{}:1:1: required terminal native data cannot be captured exactly: {error}. Fix: retain data-only values and lexical callables", owner.plan.filename)))?;
        self.atoms.merge(material.atoms);
        self.references.merge(material.references);
        self.artifacts
            .merge(material.artifacts)
            .map_err(|error| boa_engine::JsNativeError::typ().with_message(error))?;
        self.owners[index].finalized = true;
        self.refresh();
        Ok(())
    }

    fn refresh(&self) {
        for collector in
            std::iter::once(&self.entry).chain(self.owners.iter().map(|owner| &owner.collector))
        {
            let mut collector = collector.borrow_mut();
            collector.imported_atoms = Rc::new(self.atoms.clone());
            collector.imported_references = Rc::new(self.references.clone());
        }
    }
}
