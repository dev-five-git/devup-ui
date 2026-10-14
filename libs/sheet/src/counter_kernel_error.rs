use crate::counter_evidence::ReplayError;
use extractor::extract_style::CounterProducerError;

/// Dormant kernel rejection, separate from caller output failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    /// Independent allocation, placement or referenced binding disagrees.
    Authority,
    /// Independent seed replay or exact full-record coverage disagrees.
    Coverage,
    /// Retained cleanup or owner phase disagrees.
    Cleanup,
    /// Used raw preset frames disagree with the current registry.
    Preset,
    /// IR was not constructed with genuine numeric Counter authority.
    Producer(CounterProducerError),
}

impl From<ReplayError> for KernelError {
    fn from(error: ReplayError) -> Self {
        match error {
            ReplayError::Allocation => Self::Authority,
            ReplayError::Expansion => Self::Coverage,
            ReplayError::Cleanup => Self::Cleanup,
            ReplayError::MissingPreset | ReplayError::Preset => Self::Preset,
        }
    }
}

impl From<CounterProducerError> for KernelError {
    fn from(error: CounterProducerError) -> Self {
        Self::Producer(error)
    }
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dormant counter kernel: {self:?}")
    }
}
impl std::error::Error for KernelError {}

/// A kernel rejection or the caller's typed construction/output error.
#[derive(Debug, PartialEq, Eq)]
pub enum UpdateError<E> {
    /// Checked kernel failure.
    Kernel(KernelError),
    /// Caller failure, returned unchanged.
    Output(E),
}
impl<E> From<KernelError> for UpdateError<E> {
    fn from(error: KernelError) -> Self {
        Self::Kernel(error)
    }
}
impl<E: std::fmt::Display> std::fmt::Display for UpdateError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kernel(error) => error.fmt(f),
            Self::Output(error) => error.fmt(f),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for UpdateError<E> {}
