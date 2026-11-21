//! What the style props of one element declare, in the order they are
//! written, and the `style` value that applies them

use super::props::Declaration;
use super::sheet::Sheet;

/// The styles of a value at each breakpoint, each with the styles in each
/// theme: `grid[level][theme]`. A breakpoint past the last takes the styles of
/// the last, and a row of one theme holds the styles of every theme
pub(super) type Grid = Vec<Vec<Vec<Declaration>>>;

/// What a choice applies only while it holds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    /// The theme at this index shows
    Theme(usize),
    /// The window is at this breakpoint or a wider one
    AtLeast(usize),
    /// The window is in landscape (`true`) or portrait (`false`) orientation
    Landscape(bool),
    /// The element is pressed
    Pressed,
    /// The pointer is over the element
    Hovered,
    /// The element has focus
    Focused,
    /// The element is disabled
    Disabled,
}

/// The styles one prop declares
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Choice {
    /// Nothing, as `null` or `undefined` declare
    Empty,
    /// Styles the build knows, hoisted into the stylesheet
    Static(Vec<Declaration>),
    /// Styles whose values the element reads at runtime: each key with the
    /// code of its value
    Dynamic(Vec<(String, String)>),
    /// One choice or the other, by the code of a condition
    Condition {
        test: String,
        consequent: Box<Self>,
        alternate: Box<Self>,
    },
    /// Styles the build knows for each breakpoint and theme; the element
    /// picks one by the breakpoint and the theme the app shows
    Varied(Grid),
    /// A choice that applies only while `state` holds
    When { state: State, choice: Box<Self> },
    /// Choices applied one after the other
    All(Vec<Self>),
    /// A style passed on whole, by its code, which may declare any style
    Given(String),
}

/// What the runtime tells a style: which theme shows, which breakpoint the
/// window is at, and whether it is in landscape
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Reads {
    pub theme: bool,
    pub breakpoint: bool,
    pub landscape: bool,
}

impl Reads {
    /// Whether it reads what a hook of the runtime gives
    pub(super) const fn any(self) -> bool {
        self.theme || self.breakpoint || self.landscape
    }

    /// What it reads, in the order of the runtime's hooks
    pub(super) const fn flags(self) -> [bool; 3] {
        [self.theme, self.breakpoint, self.landscape]
    }

    pub(super) const fn union(self, other: Self) -> Self {
        Self {
            theme: self.theme || other.theme,
            breakpoint: self.breakpoint || other.breakpoint,
            landscape: self.landscape || other.landscape,
        }
    }
}

/// The names of the variables holding what the runtime tells
pub(super) struct Vars<'v> {
    pub theme: &'v str,
    pub breakpoint: &'v str,
    pub landscape: &'v str,
    pub pressed: &'v str,
    pub hovered: &'v str,
    pub focused: &'v str,
    /// The code telling whether the element is disabled
    pub disabled: &'v str,
}

/// The choice of `grid`, as small as it can be: the last breakpoints that
/// repeat the one before dropped, one theme when every theme declares the
/// same, static with one breakpoint and one theme
pub(super) fn varied(mut grid: Grid) -> Choice {
    if grid.iter().flatten().all(Vec::is_empty) {
        return Choice::Empty;
    }
    while grid.len() > 1 && grid[grid.len() - 1] == grid[grid.len() - 2] {
        grid.pop();
    }
    if grid
        .iter()
        .all(|row| row.windows(2).all(|pair| pair[0] == pair[1]))
    {
        for row in &mut grid {
            row.truncate(1);
        }
    }
    if let [row] = grid.as_mut_slice()
        && let [declarations] = row.as_mut_slice()
    {
        return Choice::Static(std::mem::take(declarations));
    }
    Choice::Varied(grid)
}

/// `choice` applied only while the theme at `index` of `count` shows
pub(super) fn in_theme(index: usize, count: usize, choice: Choice) -> Choice {
    match choice {
        Choice::Empty => Choice::Empty,
        choice if count <= 1 => choice,
        Choice::Static(declarations) => {
            in_theme(index, count, Choice::Varied(vec![vec![declarations]]))
        }
        Choice::Varied(grid) => varied(
            grid.into_iter()
                .map(|row| {
                    (0..count)
                        .map(|at| {
                            if at == index {
                                cell(&row, at).clone()
                            } else {
                                Vec::new()
                            }
                        })
                        .collect()
                })
                .collect(),
        ),
        choice => when(State::Theme(index), choice),
    }
}

/// `choice` applied only while `state` holds
pub(super) fn when(state: State, choice: Choice) -> Choice {
    match choice {
        Choice::Empty => Choice::Empty,
        // A block in a block of the same state, such as `_landscape` in
        // `_landscape`, holds while the outer one does
        Choice::When {
            state: inner,
            choice,
        } if inner == state => Choice::When { state, choice },
        choice => Choice::When {
            state,
            choice: Box::new(choice),
        },
    }
}

/// The styles of `row` in the theme at `theme`
const fn cell(row: &[Vec<Declaration>], theme: usize) -> &Vec<Declaration> {
    &row[if row.len() == 1 { 0 } else { theme }]
}

/// The styles of `grid` at the breakpoint `level` in the theme at `theme`
fn grid_cell(grid: &Grid, level: usize, theme: usize) -> &Vec<Declaration> {
    cell(&grid[level.min(grid.len() - 1)], theme)
}

/// How many themes a row of `grid` holds
fn themes_of(grid: &Grid) -> usize {
    grid.iter().map(Vec::len).max().unwrap_or(1)
}

impl Choice {
    /// Whether the choice declares the React Native style `key`
    fn declares(&self, key: &str) -> bool {
        match self {
            Self::Empty => false,
            Self::Static(declarations) => declares(declarations, key),
            Self::Dynamic(values) => values.iter().any(|(name, _)| name == key),
            Self::Given(_) => true,
            Self::Condition {
                consequent,
                alternate,
                ..
            } => consequent.declares(key) || alternate.declares(key),
            Self::Varied(grid) => grid
                .iter()
                .flatten()
                .any(|declarations| declares(declarations, key)),
            Self::When { choice, .. } => choice.declares(key),
            Self::All(choices) => choices.iter().any(|choice| choice.declares(key)),
        }
    }

    /// What the choice reads from the runtime
    pub(super) fn reads(&self) -> Reads {
        match self {
            Self::Empty | Self::Static(_) | Self::Dynamic(_) | Self::Given(_) => Reads::default(),
            Self::Condition {
                consequent,
                alternate,
                ..
            } => consequent.reads().union(alternate.reads()),
            Self::Varied(grid) => Reads {
                theme: themes_of(grid) > 1,
                breakpoint: grid.len() > 1,
                landscape: false,
            },
            Self::When { state, choice } => choice.reads().union(state.reads()),
            Self::All(choices) => choices.iter().fold(Reads::default(), |reads, choice| {
                reads.union(choice.reads())
            }),
        }
    }

    /// Whether the choice applies styles only while the element is in
    /// `state`
    pub(super) fn holds(&self, state: State) -> bool {
        match self {
            Self::Condition {
                consequent,
                alternate,
                ..
            } => consequent.holds(state) || alternate.holds(state),
            Self::When {
                state: held,
                choice,
            } => *held == state || choice.holds(state),
            Self::All(choices) => choices.iter().any(|choice| choice.holds(state)),
            _ => false,
        }
    }

    /// The code of the choice, its static styles named in `sheet`
    fn code(&self, sheet: &mut Sheet, vars: &Vars<'_>) -> String {
        match self {
            Self::Empty => "null".to_string(),
            Self::Static(declarations) => sheet.reference(declarations),
            Self::Dynamic(values) => object_code(values),
            Self::Given(code) => code.clone(),
            Self::Condition {
                test,
                consequent,
                alternate,
            } => {
                let test = operand(test);
                match alternate.as_ref() {
                    Self::Empty => {
                        format!("{test} && {}", consequent.operand_code(sheet, vars))
                    }
                    alternate => format!(
                        "{test} ? {} : {}",
                        consequent.operand_code(sheet, vars),
                        alternate.operand_code(sheet, vars)
                    ),
                }
            }
            Self::Varied(grid) => {
                let parts = split(grid);
                let codes: Vec<String> = parts
                    .iter()
                    .map(|part| match part {
                        Self::Varied(grid) => grid_code(grid, sheet, vars),
                        part => part.code(sheet, vars),
                    })
                    .collect();
                if codes.len() == 1 {
                    codes.concat()
                } else {
                    format!("[{}]", codes.join(", "))
                }
            }
            Self::When { state, choice } => {
                format!(
                    "{} && {}",
                    state.test(vars),
                    choice.operand_code(sheet, vars)
                )
            }
            Self::All(choices) => {
                let codes: Vec<String> = choices
                    .iter()
                    .map(|choice| choice.code(sheet, vars))
                    .collect();
                format!("[{}]", codes.join(", "))
            }
        }
    }

    /// The code of the choice as an operand of a condition
    fn operand_code(&self, sheet: &mut Sheet, vars: &Vars<'_>) -> String {
        let code = self.code(sheet, vars);
        let compound = match self {
            Self::Condition { .. } | Self::When { .. } => true,
            Self::Varied(grid) => grid.len() > 1,
            _ => false,
        };
        if compound { format!("({code})") } else { code }
    }
}

impl State {
    const fn reads(self) -> Reads {
        Reads {
            theme: matches!(self, Self::Theme(_)),
            breakpoint: matches!(self, Self::AtLeast(_)),
            landscape: matches!(self, Self::Landscape(_)),
        }
    }

    /// The code testing that the state holds
    fn test(self, vars: &Vars<'_>) -> String {
        match self {
            Self::Theme(index) => format!("{} === {index}", vars.theme),
            Self::AtLeast(level) => format!("{} >= {level}", vars.breakpoint),
            Self::Landscape(true) => vars.landscape.to_string(),
            Self::Landscape(false) => format!("!{}", vars.landscape),
            Self::Pressed => vars.pressed.to_string(),
            Self::Hovered => vars.hovered.to_string(),
            Self::Focused => vars.focused.to_string(),
            Self::Disabled => operand(vars.disabled),
        }
    }
}

/// The parts of `grid` by what their styles differ by: the styles every
/// breakpoint and theme share, those that differ only by theme, only by
/// breakpoint, and by both, so a style object is repeated only for what
/// differs
fn split(grid: &Grid) -> Vec<Choice> {
    let mut keys: Vec<&str> = Vec::new();
    for (key, _) in grid.iter().flatten().flatten() {
        if !keys.contains(&key.as_str()) {
            keys.push(key);
        }
    }
    let themes = themes_of(grid);
    let mut groups: [Vec<&str>; 4] = Default::default();
    for key in keys {
        let by_theme = grid.iter().any(|row| {
            (1..themes).any(|theme| value(cell(row, theme), key) != value(cell(row, 0), key))
        });
        let by_breakpoint = (0..themes).any(|theme| {
            (1..grid.len()).any(|level| {
                value(grid_cell(grid, level, theme), key) != value(grid_cell(grid, 0, theme), key)
            })
        });
        groups[usize::from(by_theme) + 2 * usize::from(by_breakpoint)].push(key);
    }
    groups
        .iter()
        .filter(|keys| !keys.is_empty())
        .map(|keys| varied(project(grid, keys)))
        .collect()
}

/// The value of `key` in `declarations`
fn value<'d>(declarations: &'d [Declaration], key: &str) -> Option<&'d Declaration> {
    declarations.iter().find(|(name, _)| name == key)
}

/// `grid` with only the styles of `keys`
fn project(grid: &Grid, keys: &[&str]) -> Grid {
    grid.iter()
        .map(|row| {
            row.iter()
                .map(|declarations| {
                    declarations
                        .iter()
                        .filter(|(key, _)| keys.contains(&key.as_str()))
                        .cloned()
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// The code picking the style object of `grid` by breakpoint and theme: a
/// condition on the breakpoint for each run of breakpoints that share their
/// styles, each picking by theme where the themes differ
fn grid_code(grid: &Grid, sheet: &mut Sheet, vars: &Vars<'_>) -> String {
    let mut runs: Vec<(usize, &Vec<Vec<Declaration>>)> = Vec::new();
    for (level, row) in grid.iter().enumerate() {
        if runs.last().is_none_or(|(_, last)| *last != row) {
            runs.push((level, row));
        }
    }
    let codes: Vec<String> = runs
        .iter()
        .map(|(_, row)| row_code(row, sheet, vars))
        .collect();
    match (runs.as_slice(), codes.as_slice()) {
        ([_, (start, _)], [first, second]) if first == "null" => {
            format!("{} >= {start} && {second}", vars.breakpoint)
        }
        _ => {
            let mut code = codes.last().cloned().unwrap_or_default();
            for index in (0..runs.len() - 1).rev() {
                code = format!(
                    "{} < {} ? {} : {code}",
                    vars.breakpoint,
                    runs[index + 1].0,
                    codes[index]
                );
            }
            code
        }
    }
}

/// The code picking the style object of `row` by theme
fn row_code(row: &[Vec<Declaration>], sheet: &mut Sheet, vars: &Vars<'_>) -> String {
    let mut reference = |declarations: &Vec<Declaration>| {
        if declarations.is_empty() {
            "null".to_string()
        } else {
            sheet.reference(declarations)
        }
    };
    if row.windows(2).all(|pair| pair[0] == pair[1]) {
        return reference(&row[0]);
    }
    let references: Vec<String> = row.iter().map(reference).collect();
    format!("[{}][{}]", references.join(", "), vars.theme)
}

fn declares(declarations: &[Declaration], key: &str) -> bool {
    declarations.iter().any(|(name, _)| name == key)
}

/// `code` as an operand: a name or a member read as it is, else parenthesized
fn operand(code: &str) -> String {
    if code
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.'))
    {
        code.to_string()
    } else {
        format!("({code})")
    }
}

/// An object literal of `values`, each key with the code of its value
fn object_code(values: &[(String, String)]) -> String {
    let properties: Vec<String> = values
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect();
    format!("{{ {} }}", properties.join(", "))
}

/// `declarations` with `later` declared after them, a later key winning
fn merge(declarations: &mut Vec<Declaration>, later: Vec<Declaration>) {
    for (key, value) in later {
        match declarations.iter_mut().find(|(name, _)| *name == key) {
            Some(declaration) => declaration.1 = value,
            None => declarations.push((key, value)),
        }
    }
}

/// `declarations` of a narrower breakpoint with `later` of a wider one
/// declared after them, as a media query declares them on the web: a later
/// shorthand also replaces the longhands it covers, which React Native would
/// otherwise prefer whatever their order
pub(super) fn cascade(declarations: &mut Vec<Declaration>, later: Vec<Declaration>) {
    declarations.retain(|(key, _)| !later.iter().any(|(shorthand, _)| covers(shorthand, key)));
    merge(declarations, later);
}

/// Whether React Native prefers the style `longhand` over `shorthand`, which
/// sets it too
fn covers(shorthand: &str, longhand: &str) -> bool {
    let sides = |base: &str, sides: &[&str]| {
        longhand
            .strip_prefix(base)
            .is_some_and(|side| sides.contains(&side))
    };
    match shorthand {
        "margin" | "padding" => longhand != shorthand && longhand.starts_with(shorthand),
        "inset" => {
            longhand != shorthand
                && (longhand.starts_with(shorthand)
                    || matches!(longhand, "top" | "right" | "bottom" | "left"))
        }
        "marginVertical" => sides("margin", &["Top", "Bottom"]),
        "paddingVertical" => sides("padding", &["Top", "Bottom"]),
        "marginHorizontal" => sides("margin", &["Left", "Right", "Start", "End"]),
        "paddingHorizontal" => sides("padding", &["Left", "Right", "Start", "End"]),
        "gap" => matches!(longhand, "rowGap" | "columnGap"),
        "borderWidth" | "borderColor" | "borderRadius" => {
            longhand != shorthand
                && longhand.starts_with("border")
                && longhand.ends_with(&shorthand["border".len()..])
        }
        _ => false,
    }
}

/// `later` merged into the last of `rest` when both are static or both
/// differ by breakpoint or theme, else `later` itself to apply after it
fn merge_last(rest: &mut [Choice], later: Choice) -> Option<Choice> {
    match (rest.last_mut(), later) {
        (Some(Choice::Static(last)), Choice::Static(declarations)) => {
            merge(last, declarations);
            None
        }
        (Some(Choice::Varied(last)), Choice::Static(declarations)) => {
            for cell in last.iter_mut().flatten() {
                merge(cell, declarations.clone());
            }
            None
        }
        (Some(Choice::Varied(last)), Choice::Varied(grid)) => {
            *last = merge_grids(last, &grid);
            None
        }
        (_, later) => Some(later),
    }
}

/// `later` declared after `earlier`, at every breakpoint and theme of either
fn merge_grids(earlier: &Grid, later: &Grid) -> Grid {
    let themes = themes_of(earlier).max(themes_of(later));
    (0..earlier.len().max(later.len()))
        .map(|level| {
            (0..themes)
                .map(|theme| {
                    let mut declarations = grid_cell(earlier, level, theme).clone();
                    merge(&mut declarations, grid_cell(later, level, theme).clone());
                    declarations
                })
                .collect()
        })
        .collect()
}

/// The choices of an element in the order they apply, with as many static
/// styles as can be moved first merged into one
fn ordered(choices: Vec<Choice>) -> Vec<Choice> {
    let mut first = Vec::new();
    let mut rest: Vec<Choice> = Vec::new();
    for choice in flattened(choices) {
        match choice {
            Choice::Empty => {}
            Choice::Static(declarations)
                if declarations
                    .iter()
                    .all(|(key, _)| rest.iter().all(|choice| !choice.declares(key))) =>
            {
                merge(&mut first, declarations);
            }
            choice => {
                let unmerged = merge_last(&mut rest, choice);
                rest.extend(unmerged);
            }
        }
    }
    if !first.is_empty() {
        rest.insert(0, Choice::Static(first));
    }
    // A later style can make every breakpoint or theme declare the same, and
    // what differs splits from what does not
    rest.into_iter()
        .flat_map(|choice| match choice {
            Choice::Varied(grid) => match varied(grid) {
                Choice::Varied(grid) => split(&grid),
                choice => vec![choice],
            },
            choice => vec![choice],
        })
        .filter(|choice| *choice != Choice::Empty)
        .collect()
}

/// One choice applying `choices` one after the other, as small as it can be
pub(super) fn combined(choices: Vec<Choice>) -> Choice {
    let mut choices = ordered(choices);
    match choices.len() {
        0 => Choice::Empty,
        1 => choices.remove(0),
        _ => Choice::All(choices),
    }
}

/// `choices` with the choices of each `All` in its place
fn flattened(choices: Vec<Choice>) -> Vec<Choice> {
    choices
        .into_iter()
        .flat_map(|choice| match choice {
            Choice::All(choices) => flattened(choices),
            choice => vec![choice],
        })
        .collect()
}

/// The `style` an element takes, and what it reads from the runtime
pub(super) struct StyleCode {
    pub code: String,
    pub reads: Reads,
}

/// The code of the `style` an element takes: the code of its classes (what
/// `className` gives), its choices, then the code of the styles it is given
/// whole (a spread's `style`, its own `style` prop); `None` when it has no
/// class and its props declare no style, and it keeps what it is given
pub(super) fn style_code(
    choices: Vec<Choice>,
    (classes, given): (Vec<String>, Vec<String>),
    sheet: &mut Sheet,
    vars: &Vars<'_>,
) -> Option<StyleCode> {
    let choices = ordered(choices);
    let reads = choices.iter().fold(Reads::default(), |reads, choice| {
        reads.union(choice.reads())
    });
    let mut entries = classes;
    entries.extend(choices.iter().map(|choice| match choice {
        Choice::Varied(grid) => grid_code(grid, sheet, vars),
        choice => choice.code(sheet, vars),
    }));
    if entries.is_empty() {
        return None;
    }
    entries.extend(given);
    let code = if entries.len() == 1 {
        entries.concat()
    } else {
        format!("[{}]", entries.join(", "))
    };
    Some(StyleCode { code, reads })
}

/// The states `Pressable` tells the function it calls for its style
pub(super) const INTERACTIONS: [State; 3] = [State::Pressed, State::Hovered, State::Focused];

/// The keys of the state `Pressable` gives its style function that `choices`
/// read, each bound to its variable, such as `pressed: __p`
pub(super) fn interaction_keys(choices: &[Choice], vars: &Vars<'_>) -> Vec<String> {
    [
        (State::Pressed, "pressed", vars.pressed),
        (State::Hovered, "hovered", vars.hovered),
        (State::Focused, "focused", vars.focused),
    ]
    .into_iter()
    .filter(|(state, _, _)| choices.iter().any(|choice| choice.holds(*state)))
    .map(|(_, key, variable)| format!("{key}: {variable}"))
    .collect()
}

/// `choice` without what applies only in `state`, which never holds
pub(super) fn without(choice: Choice, state: State) -> Choice {
    match choice {
        Choice::When { state: held, .. } if held == state => Choice::Empty,
        Choice::When {
            state: held,
            choice,
        } => when(held, without(*choice, state)),
        Choice::Condition {
            test,
            consequent,
            alternate,
        } => Choice::Condition {
            test,
            consequent: Box::new(without(*consequent, state)),
            alternate: Box::new(without(*alternate, state)),
        },
        Choice::All(choices) => Choice::All(
            choices
                .into_iter()
                .map(|choice| without(choice, state))
                .collect(),
        ),
        choice => choice,
    }
}
