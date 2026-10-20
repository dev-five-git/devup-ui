//! Keep file-level JSX compiler instructions attached to emitted program nodes.
use oxc_allocator::Allocator;
use oxc_ast::{
    Comment, CommentContent, CommentPosition,
    ast::{Program, Statement},
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};

/// The original header ends at the first body statement, not the first directive.
pub(crate) struct HeaderPragmas {
    end: u32,
}

impl HeaderPragmas {
    pub(crate) fn capture(program: &Program<'_>) -> Self {
        Self {
            end: program
                .body
                .first()
                .map_or(program.span.end, |node| node.span().start),
        }
    }

    fn contains(&self, comment: &Comment, source: &str) -> bool {
        comment.span.end <= self.end
            && comment
                .content_span()
                .source_text(source)
                .trim_start_matches('*')
                .split_whitespace()
                .any(|token| {
                    matches!(
                        token,
                        "@jsx" | "@jsxFrag" | "@jsxRuntime" | "@jsxImportSource"
                    )
                })
    }

    /// Only synthetic import spans may change; authored node spans remain map inputs.
    pub(crate) fn reanchor(&self, program: &mut Program<'_>) {
        let Some(first) = program
            .comments
            .iter()
            .find(|comment| self.contains(comment, program.source_text))
        else {
            return;
        };
        // Anchor zero prints before the hashbang. A comment-content offset cannot
        // collide with a real statement, and is safe for a generated import.
        if let Some(Statement::ImportDeclaration(import)) = program.body.first_mut()
            && import.span.start == 0
        {
            import.span = Span::empty(first.content_span().start);
        }
        let body_anchor = program
            .body
            .first()
            .map_or(program.span.end, |node| node.span().start);
        for comment in &mut program.comments {
            if self.contains(comment, program.source_text) {
                comment.attached_to = program
                    .directives
                    .iter()
                    .find(|directive| comment.span.end <= directive.span.start)
                    .map_or(body_anchor, |directive| directive.span.start);
                comment.position = CommentPosition::Leading;
                comment.set_followed_by_newline(true);
                // The authored node retains its annotation flag; header comments
                // must print normally rather than through Oxc's annotation store.
                if comment.is_pure() || comment.is_no_side_effects() {
                    comment.content = CommentContent::None;
                }
            }
        }
    }
}

/// A collected stylesheet has new source; copy its authored instructions into
/// that intermediate source, before Oxc generates both the output and its map.
pub(crate) fn stylesheet_source(
    source: &str,
    generated: String,
    source_type: SourceType,
) -> String {
    if generated.is_empty() || !source.contains("@jsx") {
        return generated;
    }
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, source, source_type).parse().program;
    let header = HeaderPragmas::capture(&program);
    let mut output = String::new();
    for comment in &program.comments {
        if header.contains(comment, source) {
            output.push_str(comment.span.source_text(source));
            output.push('\n');
        }
    }
    output.push_str(&generated);
    output
}
