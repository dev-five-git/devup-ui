use oxc_span::Span;

use super::{SelectedModule, policy, source::Source};
use crate::imported_constants::consumer::ReadPlan;
use crate::vanilla_extract::capture::Capture;

#[cfg(test)]
mod tests;

pub(super) struct Slots<'a> {
    plan: Option<&'a ReadPlan>,
    cursor: usize,
}

impl<'a> Slots<'a> {
    pub const fn new(plan: Option<&'a ReadPlan>) -> Self {
        Self { plan, cursor: 0 }
    }

    pub fn before(
        &mut self,
        source: &mut Source,
        input: (SelectedModule<'_>, Span),
    ) -> Result<(), String> {
        let (module, unit) = input;
        if let Some(plan) = self.plan {
            while let Some(span) = plan
                .slots
                .get(self.cursor)
                .filter(|span| span.start < unit.start)
            {
                slot(source, module, (*span, plan));
                self.cursor += 1;
            }
            if plan
                .slots
                .get(self.cursor)
                .is_some_and(|span| unit.contains_inclusive(*span))
            {
                return Err(format!(
                    "{}: consumer read overlaps selected initialization. Fix: keep Devup styling outside native initializers",
                    policy::place(module.stylesheet, unit.start)
                ));
            }
        }
        Ok(())
    }

    pub fn finish(self, source: &mut Source, module: SelectedModule<'_>) {
        if let Some(plan) = self.plan {
            for span in plan.slots.iter().skip(self.cursor) {
                slot(source, module, (*span, plan));
            }
        }
    }
}

pub(super) fn slot(
    source: &mut Source,
    module: SelectedModule<'_>,
    input: (Span, &crate::imported_constants::consumer::ReadPlan),
) {
    let (span, plan) = input;
    let mut name = format!("__ve_consumer_{}__", source.captures.len());
    while source.reserved.contains(&name) {
        name.push('_');
    }
    source.reserved.insert(name.clone());
    source
        .mapped
        .synthesize(span.start, &format!("const {name}=("));
    for guard in plan
        .guards
        .iter()
        .find(|(root, _)| *root == span)
        .into_iter()
        .flat_map(|(_, guards)| guards)
    {
        use crate::imported_constants::consumer::GuardKind;
        source.mapped.synthesize(guard.test.start, "(");
        super::imports::copy(module, guard.test, &mut source.mapped);
        let condition = match guard.kind {
            GuardKind::Truthy => ")?(",
            GuardKind::Falsy => ")?void 0:(",
            GuardKind::Nullish => ")==null?(",
        };
        source.mapped.synthesize(guard.test.end, condition);
    }
    super::imports::copy(module, span, &mut source.mapped);
    for guard in plan
        .guards
        .iter()
        .find(|(root, _)| *root == span)
        .into_iter()
        .flat_map(|(_, guards)| guards)
        .rev()
    {
        use crate::imported_constants::consumer::GuardKind;
        source.mapped.synthesize(
            span.end,
            match guard.kind {
                GuardKind::Truthy | GuardKind::Nullish => "):void 0",
                GuardKind::Falsy => ")",
            },
        );
    }
    source.mapped.synthesize(span.end, ");\n");
    source.captures.push(Capture {
        name: name.clone(),
        read: name.clone(),
        place: policy::place(module.stylesheet, span.start),
        root: span,
    });
    source.identities.push((span, None));
    source.observations.write(
        &mut source.mapped,
        crate::vanilla_extract::capture::Observation {
            span,
            place: policy::place(module.stylesheet, span.start),
            kind: crate::vanilla_extract::capture::ObservationKind::After,
            reads: vec![name],
            mutations: Vec::new(),
        },
    );
}
