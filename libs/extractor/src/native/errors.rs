//! The build errors of the React Native output

use super::NativeTarget;

/// Why a value or prop cannot compile, and what to write instead
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Problem {
    pub reason: String,
    pub fix: String,
}

impl Problem {
    pub(super) fn new(reason: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            fix: fix.into(),
        }
    }

    /// The error for the prop `prop` of `<element>`
    pub(super) fn on(&self, element: &str, prop: &str) -> String {
        format!(
            "`<{element}>` cannot use `{prop}` on React Native: {}. Fix: {}",
            self.reason, self.fix
        )
    }

    /// The error for the import `name` of `package`
    pub(super) fn import(&self, name: &str, package: &str) -> String {
        format!(
            "`{name}` from `{package}` cannot be used on React Native: {}. Fix: {}",
            self.reason, self.fix
        )
    }
}

/// What needs the React Native runtime: one style object a breakpoint, a
/// theme or a press state, chosen while the app runs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeFeature {
    Responsive,
    Theme,
    Pressed,
}

impl RuntimeFeature {
    const fn what(self) -> &'static str {
        match self {
            Self::Responsive => "a responsive value is chosen by the window width",
            Self::Theme => "a theme variant is chosen by the theme the app shows",
            Self::Pressed => "a press style is chosen while the element is pressed",
        }
    }

    const fn fix(self) -> &'static str {
        match self {
            Self::Responsive => "give one value",
            Self::Theme => "give the value for one theme",
            Self::Pressed => {
                "pass a function to the `style` of a React Native `Pressable`, reading `pressed`"
            }
        }
    }

    /// Why the feature cannot compile for `target`
    pub(super) fn problem(self, target: NativeTarget) -> Problem {
        let runtime = if target.runtime {
            "which needs the React Native runtime that this version does not include yet"
        } else {
            "which needs the React Native runtime that `runtime: false` turns off"
        };
        Problem::new(format!("{}, {runtime}", self.what()), self.fix())
    }
}

/// A theme token, read from the theme in a later version
pub(super) fn theme_token(value: &str) -> Problem {
    Problem::new(
        format!("the theme token in `{value}` is not read on React Native yet"),
        "write the value itself",
    )
}
