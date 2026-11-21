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
    Motion,
    Viewport,
}

impl RuntimeFeature {
    const fn what(self) -> &'static str {
        match self {
            Self::Responsive => "a responsive value is chosen by the window width",
            Self::Orientation => "an orientation style is chosen by the orientation of the window",
            Self::Theme => "a theme variant is chosen by the theme the app shows",
            Self::Motion => "a motion style is chosen by whether the system reduces motion",
            Self::Viewport => "a viewport length is computed from the window's size",
        }
    }

    const fn fix(self) -> &'static str {
        match self {
            Self::Responsive => "give one value, or turn the runtime on",
            Self::Orientation | Self::Motion => "remove it, or turn the runtime on",
            Self::Theme => "give the value for one theme, or turn the runtime on",
            Self::Viewport => "use a number of pixels or a percentage, or turn the runtime on",
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

/// Why animations and transitions cannot compile in an app without
/// react-native-reanimated, which runs them
pub(super) fn reanimated_problem() -> Problem {
    Problem::new(
        "animations and transitions run on react-native-reanimated, which the app does not have",
        "install react-native-reanimated 4 or later (`npx expo install react-native-reanimated`), then restart Metro",
    )
}

/// Why `name`, the theme API of styled-components or Emotion, cannot be
/// imported with the runtime off
pub(super) fn compat_theme_problem(name: &str) -> Problem {
    Problem::new(
        format!(
            "`{name}` gives the theme of `ThemeProvider` through the React Native runtime, which `runtime: false` turns off"
        ),
        "remove it, or turn the runtime on",
    )
}
