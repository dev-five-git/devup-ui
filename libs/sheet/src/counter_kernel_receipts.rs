use super::{KernelAttempt, PreparedUpdate, UpdateRequest};
use crate::counter_kernel::{
    Candidate, FrozenAuthority, KernelError, link, prepare, production, state_live, traversal,
    validation::BuildConfig,
};
use crate::theme::Theme;
pub(in crate::counter_kernel) use app::witness;
use css::allocation_input::CapturedDelivery as Delivery;
use extractor::extract_style::{
    compiler_associations::VariableUse,
    compiler_receipts::{
        CompilerReceiptBatch, ProducedData, ReceiptBatchView as View, ReceiptWitness,
        WitnessDisposition,
    },
    extract_style_value::ExtractStyleValue as Value,
};
use production::{application as app, require, same};
use std::collections::BTreeMap;
type Checked<T> = Result<T, KernelError>;

pub(in crate::counter_kernel) struct Observation {
    pub(in crate::counter_kernel) operand: Value,
    pub(in crate::counter_kernel) produced: ProducedData,
    pub(in crate::counter_kernel) expected: Candidate,
    pub(in crate::counter_kernel) genuine: bool,
}
impl Observation {
    pub(super) fn replay(&self, theme: &Theme) -> Checked<(Candidate, Delivery)> {
        let a = app::acquired(&self.operand, &self.produced)?;
        let p = &self.expected.proof.emission.seed.placement;
        if self.genuine {
            app::producer(a, theme, p)
        } else {
            app::apply(a, theme, p)
        }
    }
}
pub(in crate::counter_kernel) struct AttemptCheck {
    pub(super) authority: FrozenAuthority,
    pub(super) build: BuildConfig,
    pub(super) routes: BTreeMap<String, (String, bool, bool)>,
    pub(super) observations: Vec<Observation>,
    pub(super) variables: Vec<(VariableUse, Vec<ProducedData>)>,
}
impl AttemptCheck {
    pub(in crate::counter_kernel) fn keep(
        &mut self,
        o: Observation,
        delivery: Delivery,
    ) -> Checked<()> {
        prepare::references(&o.operand, &mut self.authority.originals)?;
        self.authority.capture(&o.expected, delivery)?;
        link::check(&o.expected, &self.authority)?;
        self.route(Some(&o.expected.proof.emission.seed.placement.source_file));
        self.observations.push(o);
        Ok(())
    }
    fn route(&mut self, raw: Option<&str>) {
        if let Some(raw) = raw {
            self.routes.insert(raw.into(), prepare::route(raw));
        }
    }
    pub(in crate::counter_kernel) fn producer(
        &mut self,
        w: &ReceiptWitness,
        theme: &Theme,
        p: &crate::emission_seed::EmissionContext,
    ) -> Checked<(Candidate, Delivery)> {
        let (candidate, delivery) =
            app::producer(app::acquired(&w.operand, &w.produced)?, theme, p)?;
        self.keep(
            Observation {
                operand: w.operand.clone(),
                produced: w.produced.clone(),
                expected: candidate.clone(),
                genuine: true,
            },
            delivery.clone(),
        )?;
        Ok((candidate, delivery))
    }
}
pub(super) fn variable(u: &VariableUse, receipts: &[ProducedData]) -> Checked<()> {
    let prefix = &u.config.prefix;
    let mut name = u.original.site().map(|s| s.variable_name(prefix));
    for data in receipts {
        let ProducedData::Dynamic(data) = data else {
            return Err(KernelError::Authority);
        };
        require(name.as_ref().is_none_or(|name| name == &data.variable))?;
        name = Some(data.variable.clone());
    }
    Ok(())
}
pub(in crate::counter_kernel) fn check(v: &View<'_>, theme: &Theme) -> Checked<AttemptCheck> {
    let g = v.associations;
    let mut c = AttemptCheck {
        authority: FrozenAuthority::live(),
        build: state_live::build(),
        routes: BTreeMap::new(),
        observations: Vec::new(),
        variables: Vec::new(),
    };
    let originals = css::file_map::get_original_ids();
    for i in &g.invocations {
        require(originals.get(&i.raw) == Some(&i.original))?;
        c.authority.originals.insert(i.raw.clone(), i.original);
        c.route(Some(&i.raw));
    }
    for a in &g.operands {
        prepare::references(&a.operand, &mut c.authority.originals)?;
    }
    for w in v.witnesses {
        let (raw, _) = prepare::frame(g, w.invocation, None)?;
        let emitted = match w.disposition {
            WitnessDisposition::Emission => true,
            WitnessDisposition::ReservationOnly => false,
        };
        require(emitted == g.emissions.iter().any(|f| f.receipt == w.id))?;
        c.route(w.route.as_deref());
        c.producer(w, theme, &app::placement(raw, w.route.as_deref()))?;
    }
    for f in &g.emissions {
        require(
            same(&f.operand, &witness(v, f.receipt)?.operand)
                && v.styles.iter().any(|s| same(s, &f.operand))
                && g.operands
                    .iter()
                    .any(|a| a.survivor == f.survivor && same(&a.operand, &f.operand)),
        )?;
    }
    for n in &g.naming {
        let w = witness(v, n.receipt)?;
        let (raw, inherited) = prepare::frame(g, n.invocation, n.scope)?;
        let route = n.route.as_deref().or(inherited);
        require(same(&n.operand, &w.operand))?;
        c.route(route);
        c.producer(w, theme, &app::placement(raw, route))?;
    }
    for e in &g.rewrites {
        prepare::frame(g, e.invocation, e.scope)?;
        require(traversal::identifier_only(&e.before, &e.after))?;
        prepare::references(
            &Value::Dynamic(e.before.clone()),
            &mut c.authority.originals,
        )?;
    }
    for e in &g.orders {
        prepare::frame(g, e.invocation, e.scope)?;
        require(
            traversal::order_only(&e.before, &e.after)
                && same(&e.after, &witness(v, e.receipt)?.operand),
        )?;
        prepare::references(&e.before, &mut c.authority.originals)?;
    }
    for u in &g.variables {
        let (_, inherited) = prepare::frame(g, u.invocation, u.scope)?;
        let route = u.route.as_deref().or(inherited);
        require(u.config == c.build.config)?;
        c.route(route);
        let source = Value::Dynamic(u.original.clone());
        prepare::references(&source, &mut c.authority.originals)?;
        let mut reachable = vec![source];
        let within = |id, scope| id == u.invocation && scope == u.scope;
        for e in g.rewrites.iter().filter(|e| within(e.invocation, e.scope)) {
            let before = Value::Dynamic(e.before.clone());
            if reachable.iter().any(|s| same(s, &before)) {
                reachable.push(Value::Dynamic(e.after.clone()));
            }
        }
        let mut reached = Vec::new();
        for e in g.orders.iter().filter(|e| within(e.invocation, e.scope)) {
            if reachable
                .iter()
                .any(|s| same(s, &e.before) || same(s, &e.after))
            {
                reachable.push(e.after.clone());
                reached.push(e.receipt);
            }
        }
        let mut data = Vec::new();
        for id in &u.receipts {
            let w = witness(v, *id)?;
            require(reached.contains(id))?;
            // Variable observation routes need not be the already-checked class naming routes.
            data.push(w.produced.clone());
        }
        variable(u, &data)?;
        c.variables.push((u.clone(), data));
    }
    Ok(c)
}
impl<'id> KernelAttempt<'id> {
    /// Consume genuine compiler receipts without acquiring a second set of names.
    /// # Errors
    /// Rejects stale BASE or incompatible semantic, producer or live authority.
    pub fn prepare_compiled(
        self,
        batch: CompilerReceiptBatch<'_>,
        request: UpdateRequest<'_>,
    ) -> Result<PreparedUpdate<'id>, KernelError> {
        batch.consume(|v| self.prepare_input(prepare::Input::Compiled(&v), request))
    }
}
