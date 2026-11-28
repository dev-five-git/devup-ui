//! The build errors of the React Native output

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

    /// The error for the prop `prop` of `<element>`, or the key `prop` of a
    /// call such as `css()`
    pub(super) fn on(&self, element: &str, prop: &str) -> String {
        let subject = if element.ends_with(')') {
            element.to_string()
        } else {
            format!("<{element}>")
        };
        format!(
            "`{subject}` cannot use `{prop}` on React Native: {}. Fix: {}",
            self.reason, self.fix
        )
    }

    /// The error for a call such as `css()`
    pub(super) fn call(&self, call: &str) -> String {
        format!(
            "`{call}` cannot compile here on React Native: {}. Fix: {}",
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
/// theme or an orientation, chosen while the app runs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeFeature {
    Responsive,
    Orientation,
    Theme,
}

impl RuntimeFeature {
    const fn what(self) -> &'static str {
        match self {
            Self::Responsive => "a responsive value is chosen by the window width",
            Self::Orientation => "an orientation style is chosen by the orientation of the window",
            Self::Theme => "a theme variant is chosen by the theme the app shows",
        }
    }

    const fn fix(self) -> &'static str {
        match self {
            Self::Responsive => "give one value, or turn the runtime on",
            Self::Orientation => "remove it, or turn the runtime on",
            Self::Theme => "give the value for one theme, or turn the runtime on",
        }
    }

    /// Why the feature cannot compile with `runtime: false`
    pub(super) fn problem(self) -> Problem {
        Problem::new(
            format!(
                "{}, which needs the React Native runtime that `runtime: false` turns off",
                self.what()
            ),
            self.fix(),
        )
    }
}

/// Why the theme API `name` cannot be imported with `runtime: false`
pub(super) fn theme_api(name: &str) -> Problem {
    Problem::new(
        format!(
            "`{name}` reads or changes the theme the app shows, which needs the React Native runtime that `runtime: false` turns off"
        ),
        "remove it, or turn the runtime on",
    )
}
