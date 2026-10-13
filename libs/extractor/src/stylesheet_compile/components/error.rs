use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) struct Failure {
    diagnostic: String,
}

impl Failure {
    pub(super) fn new(diagnostic: impl Display) -> Self {
        Self {
            diagnostic: diagnostic.to_string(),
        }
    }

    pub(super) fn check(diagnostics: impl IntoIterator<Item = impl Display>) -> Result<(), Self> {
        diagnostics
            .into_iter()
            .next()
            .map_or(Ok(()), |diagnostic| Err(Self::new(diagnostic)))
    }

    pub(crate) fn located(
        self,
        stylesheet: crate::vanilla_extract::Stylesheet<'_>,
        span: oxc_span::Span,
    ) -> String {
        crate::located_errors(
            stylesheet.filename,
            stylesheet.source,
            stylesheet.edits,
            vec![(span.start, self.to_string())],
        )
    }
}

impl Display for Failure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Cannot compile component: {}. Fix: correct its expression",
            self.diagnostic
        )
    }
}

impl std::error::Error for Failure {}

#[cfg(test)]
mod tests;
