//! The errors the script engine gives, told with the code as written

use crate::module_loader::IMPORT_CYCLE;

#[cfg(test)]
mod tests;

/// The name the evaluated script goes by, which the engine prints in its
/// positions
pub(crate) const SCRIPT_PATH: &str = "devup-ui-stylesheet";

const FRAME_SEPARATOR: &str = "\n    at ";

/// Where the engine says an error or a call happened
enum Place {
    /// `line` and `column`, counted from 1, of the script
    Script(u32, u32),
    /// A function the build provides
    Native,
    /// A script no author wrote or no position is known for
    Foreign,
}

struct Frame<'t> {
    function: &'t str,
    place: Place,
}

/// What `inner` of `(inner)` says, `None` when it is not a place
fn place(inner: &str) -> Option<Place> {
    if inner.starts_with(crate::evaluation_sandbox::INTERNAL_SOURCE) {
        return Some(Place::Foreign);
    }
    if inner == "native" {
        return Some(Place::Native);
    }
    if let Some(position) = inner
        .strip_prefix(SCRIPT_PATH)
        .and_then(|position| position.strip_prefix(':'))
    {
        return Some(
            position
                .split_once(':')
                .and_then(|(line, column)| {
                    Some(Place::Script(line.parse().ok()?, column.parse().ok()?))
                })
                .unwrap_or(Place::Foreign),
        );
    }
    ["unknown at ", "eval at ", "json at "]
        .iter()
        .any(|prefix| inner.starts_with(prefix))
        .then_some(Place::Foreign)
}

/// `text` without the place it ends with, and the place
fn split_place(text: &str) -> Option<(&str, Place)> {
    let text = if text.starts_with("<read> (") {
        text.strip_suffix(" (native)").unwrap_or(text)
    } else {
        text
    };
    let (before, inner) = text.strip_suffix(')')?.rsplit_once(" (")?;
    Some((before, place(inner)?))
}

/// The position a syntax error ends with: `message at line 3, col 7`. The
/// engine gives the early errors of a whole script, like a name declared twice,
/// the position 1:1 whichever they are, which is no position.
fn split_position(text: &str) -> Option<(&str, Place)> {
    let (before, position) = text.rsplit_once(" at line ")?;
    let (line, column) = position.split_once(", col ")?;
    Some((
        before,
        match (line.parse().ok()?, column.parse().ok()?) {
            (1, 1) => Place::Foreign,
            (line, column) => Place::Script(line, column),
        },
    ))
}

/// The error without the call stack, the call stack from the innermost call,
/// and where the error itself was raised
fn parse(error: &str) -> (String, Vec<Frame<'_>>, Option<Place>) {
    let mut segments: Vec<&str> = error.split(FRAME_SEPARATOR).collect();
    let mut frames = Vec::new();
    while segments.len() > 1
        && let Some((function, place)) = segments.last().and_then(|last| split_place(last))
    {
        frames.push(Frame { function, place });
        segments.pop();
    }
    frames.reverse();
    let head = segments.join(FRAME_SEPARATOR);
    let (head, placed) =
        split_place(&head).map_or((head.as_str(), None), |(head, place)| (head, Some(place)));
    match split_position(head) {
        Some((cause, place)) => (cause.to_string(), frames, Some(place)),
        None => (head.to_string(), frames, placed),
    }
}

/// What to change for an error to stop
fn fix(cause: &str) -> String {
    if cause.contains(IMPORT_CYCLE) {
        return "break the import cycle, or read the imported value only after both modules finished evaluating, for example inside a function called later".to_string();
    }
    if let Some(name) = cause
        .strip_prefix("ReferenceError: ")
        .and_then(|message| message.strip_suffix(" is not defined"))
    {
        return format!(
            "declare or import `{name}` before this code reads it; it is not available when the stylesheet is evaluated at build time"
        );
    }
    if cause.starts_with("SyntaxError") {
        return "correct the syntax at this location".to_string();
    }
    if cause.starts_with("TypeError") {
        return "check that every value read, called or constructed here exists when the stylesheet is evaluated at build time".to_string();
    }
    "remove or guard the code that fails here, or correct the cause above, so the stylesheet evaluates without throwing".to_string()
}

/// The error `error` the engine gave, led by where the code as written raised
/// it, with its call stack in that code and what to fix. `locate` tells where
/// the code as written is for a position of the script, `None` for code the
/// build generated; `fallback` leads when no position is known
pub(super) fn describe(
    error: &str,
    locate: impl Fn(u32, u32) -> Option<String>,
    fallback: &str,
) -> String {
    let (cause, frames, raised) = parse(error);
    let in_script = |place: &Place| match place {
        Place::Script(line, column) => locate(*line, *column),
        Place::Native | Place::Foreign => None,
    };
    let mut at = raised
        .filter(|_| frames.is_empty())
        .and_then(|place| in_script(&place));
    let mut calls = Vec::new();
    for frame in &frames {
        if frame.function.starts_with("__devup_read_site_")
            || frame.function.starts_with("__devup_operation_site_")
        {
            continue;
        }
        if matches!(frame.place, Place::Native) {
            calls.push(format!("{FRAME_SEPARATOR}{} (native)", frame.function));
        } else if let Some(located) = in_script(&frame.place) {
            calls.push(format!("{FRAME_SEPARATOR}{} ({located})", frame.function));
            at.get_or_insert(located);
        }
    }
    let cause = cause.trim_end_matches('.');
    format!(
        "{}: JS execution error: {cause}. Fix: {}{}",
        at.as_deref().unwrap_or(fallback),
        fix(cause),
        calls.concat()
    )
}

#[cfg(test)]
mod coverage_tests;
