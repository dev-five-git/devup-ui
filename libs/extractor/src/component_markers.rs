//! The component-selector markers a file gives and the ones its styles read.
//!
//! A styled component carries a marker class when a selector may pick it out
//! (`${Child} { ... }`, `[Child]`): one in its own file, or, as it is
//! exported, one in a module importing it. A build that has seen every
//! module can drop each marker no module's styles read; these lists are what
//! it needs to know which.

/// The marker classes of one extraction
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ComponentMarkers {
    /// Markers the file's components render with, in order, once each
    pub given: Vec<String>,
    /// Markers the file's styles select a component by, in order, once each
    pub selected: Vec<String>,
}

impl ComponentMarkers {
    /// Record `marker` as one a component of the file renders with
    pub(crate) fn give(&mut self, marker: &str) {
        push_once(&mut self.given, marker);
    }

    /// Record `marker` as one a selector of the file reads
    pub(crate) fn select(&mut self, marker: &str) {
        push_once(&mut self.selected, marker);
    }
}

fn push_once(list: &mut Vec<String>, marker: &str) {
    if !list.iter().any(|known| known == marker) {
        list.push(marker.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_marker_is_recorded_once_in_order() {
        // Given
        let mut markers = ComponentMarkers::default();
        // When
        markers.give("a--a");
        markers.give("a--b");
        markers.give("a--a");
        markers.select("b--a");
        markers.select("b--a");
        // Then
        assert_eq!(markers.given, ["a--a", "a--b"]);
        assert_eq!(markers.selected, ["b--a"]);
    }
}
