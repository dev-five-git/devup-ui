pub(crate) use super::compiler_associations::{CollectedStyles, ConsumerScope, RawScope, complete};
pub(crate) use super::compiler_diagnostics::{ProjectionError, check_aux, check_terminal};
pub(crate) use super::compiler_request::{configuration, same};
use super::{CounterProducerError, ExtractDynamicStyle as Dynamic, ProducerPolicy};
use super::{
    compiler_receipts as receipts, extract_style_value::ExtractStyleValue as Value,
    style_property::StyleProperty,
};
pub use crate::assignment_value::RewriteEdge;
pub use crate::gen_class_name::OrderEdge;
pub use crate::gen_style::VariableUse;
use css::allocation_input::{AllocationContext, CapturedNameConfig};
use receipts::ProducedData;
use std::cell::Cell;
use std::rc::Rc;
type ProjectionResult<T> = Result<T, ProjectionError>;

thread_local! {
    static DEMAND: Cell<u32> = const { Cell::new(0) };
}
pub(crate) struct DemandScope(u32);
impl DemandScope {
    pub(crate) fn enter(offset: u32) -> Self {
        Self(DEMAND.replace(offset))
    }
}
impl Drop for DemandScope {
    fn drop(&mut self) {
        DEMAND.set(self.0);
    }
}
pub(crate) fn demand() -> u32 {
    DEMAND.get()
}
pub(crate) fn frozen_configuration() -> Result<CapturedNameConfig, ProjectionError> {
    receipts::JOURNAL.with_borrow(|journal| match journal {
        Some(journal) if journal.config == configuration() => Ok(journal.config.clone()),
        Some(_) | None => Err(ProjectionError::Context),
    })
}
pub(crate) fn produce(value: &Value, filename: Option<&str>) -> ProjectionResult<ProducedData> {
    match value {
        Value::Static(style) => style
            .counter_produce(filename)
            .map(Rc::new)
            .map(ProducedData::Allocation),
        Value::Dynamic(style) => style
            .counter_produce(filename)
            .map(Rc::new)
            .map(ProducedData::Dynamic),
        Value::Keyframes(frames) => frames
            .counter_produce(filename)
            .map(Rc::new)
            .map(ProducedData::Allocation),
        Value::Typography(_) | Value::Css(_) | Value::Import(_) | Value::FontFace(_) => {
            Err(CounterProducerError::WrongPolicy)
        }
    }
    .map_err(ProjectionError::Producer)
}
pub(crate) use crate::gen_class_name::{keyframe_name, static_name};
pub(crate) fn context(value: &Value, route: Option<&str>) -> ProjectionResult<AllocationContext> {
    let inherited = receipts::JOURNAL
        .with_borrow(|journal| journal.as_ref().and_then(|journal| journal.graph.route()));
    super::compiler_request::context(value, route.or(inherited.as_deref()))
}
fn try_variable(style: &Dynamic, consumer: &str, route: Option<&str>) -> ProjectionResult<String> {
    if !crate::compiler_policy::active() {
        return Ok(style.variable_name());
    }
    let config = frozen_configuration()?;
    #[cfg(test)]
    crate::compiler_policy::tests::observe_variable();
    receipts::JOURNAL.with_borrow_mut(|journal| {
        let journal = journal.as_mut().ok_or(ProjectionError::Context)?;
        let graph = &mut journal.graph;
        let invocation = graph.invocation.ok_or(ProjectionError::Context)?;
        let original = graph
            .rewrites
            .iter()
            .rev()
            .find(|edge| {
                edge.invocation == invocation
                    && edge.scope == graph.scope
                    && same(
                        &Value::Dynamic(edge.after.clone()),
                        &Value::Dynamic(style.clone()),
                    )
            })
            .map_or_else(|| style.clone(), |edge| edge.before.clone());
        let value = Value::Dynamic(original.clone());
        let mut ids = Vec::new();
        for edge in &graph.orders {
            if edge.invocation == invocation
                && edge.scope == graph.scope
                && (same(&edge.before, &value) || same(&edge.after, &value))
                && !ids.contains(&edge.receipt)
            {
                ids.push(edge.receipt);
            }
        }
        let variable = match (original.producer_policy(), original.site()) {
            (ProducerPolicy::Current, _) => {
                return Err(ProjectionError::Producer(CounterProducerError::WrongPolicy));
            }
            (ProducerPolicy::CounterOriginal(_), Some(site)) => match &site.file {
                css::sparse_site::SourceFile::D9(_) => site.variable_name(&config.prefix),
                css::sparse_site::SourceFile::Unnumbered(_) => {
                    return Err(ProjectionError::Producer(
                        CounterProducerError::UnnumberedSite,
                    ));
                }
            },
            (ProducerPolicy::CounterOriginal(_), None) => {
                let mut names = ids
                    .iter()
                    .map(|id| match &journal.witnesses[id.0].produced {
                        ProducedData::Dynamic(value) => Ok(value.variable.clone()),
                        ProducedData::Allocation(_) => Err(ProjectionError::Variable),
                    });
                let first = names.next().ok_or(ProjectionError::Variable)??;
                for next in names {
                    if next? != first {
                        return Err(ProjectionError::Variable);
                    }
                }
                first
            }
        };
        graph.variables.push(VariableUse {
            original,
            consumer: consumer.into(),
            route: route.map(str::to_string),
            config,
            invocation,
            scope: graph.scope,
            receipts: ids,
        });
        Ok(variable)
    })
}
pub(crate) fn variable_for(style: &Dynamic, offset: u32, consumer: &str) -> Option<String> {
    receipts::collect(try_variable(style, consumer, None), Some(offset))
}
pub(crate) struct InlineAssignment {
    pub(crate) variable_name: String,
    pub(crate) identifier: String,
}
pub(crate) fn variables_ready(variables: &[VariableUse]) -> bool {
    variables.iter().all(|usage| !usage.receipts.is_empty())
}
pub(crate) fn inline(value: &Value, filename: Option<&str>) -> Option<InlineAssignment> {
    if !crate::compiler_policy::active() {
        return match value.extract(filename) {
            Some(StyleProperty::Variable {
                variable_name,
                identifier,
                ..
            }) => Some(InlineAssignment {
                variable_name,
                identifier,
            }),
            Some(StyleProperty::ClassName(_)) | None => None,
        };
    }
    match value {
        Value::Dynamic(style) => {
            receipts::collect_operand(try_variable(style, style.identifier(), filename), value).map(
                |variable_name| InlineAssignment {
                    variable_name,
                    identifier: style.identifier().to_string(),
                },
            )
        }
        Value::Static(_)
        | Value::Typography(_)
        | Value::Keyframes(_)
        | Value::Css(_)
        | Value::Import(_)
        | Value::FontFace(_) => None,
    }
}
pub(crate) fn auxiliary(
    source: &str,
    filename: &str,
) -> Option<(
    receipts::ReservationScope,
    crate::sparse_sites::SiteScope,
    RawScope,
)> {
    match crate::sparse_sites::producer_policy() {
        ProducerPolicy::Current => None,
        ProducerPolicy::CounterOriginal(original) => Some((
            receipts::ReservationScope::enter(),
            crate::sparse_sites::SiteScope::enter_counter_generated(original, source),
            RawScope::enter(filename, original),
        )),
    }
}
