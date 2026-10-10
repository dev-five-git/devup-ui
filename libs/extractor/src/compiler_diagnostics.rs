use super::{CounterProducerError, compiler_receipts as receipts};
use crate::import_alias_visit::Edit;
use std::error::Error;

#[derive(Clone, Debug)]
pub(crate) enum ProjectionError {
    Producer(CounterProducerError),
    Context,
    Variable,
    Site(String),
    Terminal(String),
}
impl std::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Producer(error) => error.fmt(f),
            Self::Context => f.write_str("counter naming configuration changed within compilation"),
            Self::Variable => {
                f.write_str("counter variable requires an unambiguous finalized receipt")
            }
            Self::Site(message) | Self::Terminal(message) => f.write_str(message),
        }
    }
}
impl Error for ProjectionError {}
impl<E> From<ProjectionError> for crate::compiler_policy::CounterCompileError<E> {
    fn from(error: ProjectionError) -> Self {
        Self::Compile(Box::new(error))
    }
}
pub(crate) fn collect<T>(result: Result<T, ProjectionError>, offset: Option<u32>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            receipts::JOURNAL.with_borrow_mut(|journal| {
                if let Some(journal) = journal {
                    journal.errors.push((
                        offset.unwrap_or_else(super::compiler_projection::demand),
                        error,
                        offset
                            .map_or_else(crate::style_origin::current, crate::style_origin::at)
                            .location(),
                    ));
                }
            });
            None
        }
    }
}
pub(crate) fn retain_terminal(error: &(dyn Error + 'static)) {
    if let Some(error) = error.downcast_ref::<ProjectionError>() {
        collect::<()>(Err(error.clone()), None);
    }
}
pub(crate) fn check_terminal(
    filename: &str,
    source: &str,
    edits: &[&[Edit]],
) -> Result<(), Box<dyn Error>> {
    if crate::compiler_policy::active() {
        for (at, message) in crate::provenance::site_errors() {
            collect::<()>(Err(ProjectionError::Site(message)), Some(at));
        }
    }
    let errors = receipts::JOURNAL.with_borrow_mut(|journal| {
        journal
            .as_mut()
            .map_or_else(Vec::new, |journal| std::mem::take(&mut journal.errors))
    });
    if errors.is_empty() {
        return Ok(());
    }
    let message = errors
        .into_iter()
        .map(|(at, error, origin)| match (&error, origin) {
            (ProjectionError::Terminal(message), _) => message.clone(),
            (_, Some(origin)) => format!("{origin}: {error}"),
            (_, None)
                if at == 0
                    || crate::sparse_sites::counter_owner() == css::CounterOwner::Unnumbered =>
            {
                format!(
                    "{}: {error}",
                    css::style_origin::RealLocation::ModuleExport {
                        file: filename.into(),
                        binding: None
                    }
                )
            }
            (_, None) => {
                crate::located_errors(filename, source, edits, vec![(at, error.to_string())])
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    Err(Box::new(ProjectionError::Terminal(message)))
}
pub(crate) fn check_aux(
    filename: &str,
    source: &str,
    errors: Vec<(u32, String)>,
) -> Result<(), Box<dyn Error>> {
    if crate::compiler_policy::active() {
        for (offset, message) in errors {
            collect::<()>(Err(ProjectionError::Site(message)), Some(offset));
        }
        check_terminal(filename, source, &[])?;
    }
    Ok(())
}
