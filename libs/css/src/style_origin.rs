use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

/// A diagnostic witness from the actual source read, never a generated offset.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StyleOrigin {
    pub file: String,
    pub line: usize,
    #[serde(rename = "col")]
    pub column: usize,
    #[serde(rename = "originalExpression")]
    pub expression: String,
}

/// Ordered diagnostic tiers; none claims a generated position is source text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RealLocation {
    Exact(StyleOrigin),
    ProducedByCall(StyleOrigin),
    ModuleExport {
        file: String,
        binding: Option<String>,
    },
}

impl std::fmt::Display for RealLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact(origin) => write!(
                f,
                "{}:{}:{}: `{}`",
                origin.file, origin.line, origin.column, origin.expression
            ),
            Self::ProducedByCall(origin) => write!(
                f,
                "produced by the call at {}:{}:{}: `{}`",
                origin.file, origin.line, origin.column, origin.expression
            ),
            Self::ModuleExport { file, binding } => {
                write!(f, "{file}")?;
                if let Some(binding) = binding {
                    write!(f, " export `{binding}`")?;
                }
                write!(
                    f,
                    ": position unavailable (produced inside eval or a callback)"
                )
            }
        }
    }
}

/// Diagnostic metadata does not participate in a style's semantic identity.
#[derive(Clone, Debug, Default)]
pub struct Origin(pub Option<Box<StyleOrigin>>, pub Option<Box<RealLocation>>);

impl PartialEq for Origin {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for Origin {}
impl Hash for Origin {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}
impl PartialOrd for Origin {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Origin {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl Origin {
    pub fn fill(&mut self, origin: Option<&StyleOrigin>) {
        if self.0.is_none() {
            self.0 = origin.cloned().map(Box::new);
        }
    }

    #[must_use]
    pub fn location(&self) -> Option<RealLocation> {
        self.0
            .as_deref()
            .cloned()
            .map(RealLocation::Exact)
            .or_else(|| self.1.as_deref().cloned())
    }

    #[must_use]
    pub fn from_location(location: RealLocation) -> Self {
        match location {
            RealLocation::Exact(origin) => Self(Some(Box::new(origin)), None),
            RealLocation::ProducedByCall(_) | RealLocation::ModuleExport { .. } => {
                Self(None, Some(Box::new(location)))
            }
        }
    }

    pub fn fill_from(&mut self, origin: &Self) {
        if self.0.is_none() && self.1.is_none() {
            self.0.clone_from(&origin.0);
            self.1.clone_from(&origin.1);
        }
    }
}
