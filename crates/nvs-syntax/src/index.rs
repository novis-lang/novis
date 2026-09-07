//! `SyntaxIndex`: which node of a parsed file a byte offset is inside, and what
//! that node is inside.
//!
//! `rule:ide/the-index-answers-the-cursor` is the rule this answers, and
//! `rule:ide/one-grammar-one-tree` is why it is an *index* rather than a tree:
//! [`crate::Parsed`]'s statements are the tree, and an entry here is a kind, a
//! span and the id of the entry containing it — one flat `Vec` in source order,
//! holding no node and owning no source. [`SyntaxIndex::at`] answers a
//! [`NodePath`]: the innermost node containing the offset, then its ancestors
//! outward. Hover, definition and completion each take a different depth of
//! that path, and `selectionRange` is the path itself, so each is a projection
//! of this rather than a walk of its own.
//!
//! # Decision: the nodes are [`crate::walk`]'s nodes
//!
//! A node here is a statement, an expression or a member of a declaration —
//! exactly the productions that module walks, for exactly the reasons its own
//! decision gives, and the index is built by flattening that walk rather than
//! by matching on the grammar a second time. Two matches over
//! `#[non_exhaustive]` enums would both have to be updated when a production
//! lands, and the one that was forgotten would answer plausibly rather than
//! failing to build. So there is one traversal and one vocabulary of kinds, and
//! when `rule:tooling/reflection-and-source-parsing-are-core-features`'s typed
//! roster redraws where a node's boundary is, both consumers move with it.
//!
//! A cursor inside a type, a parameter name or a modifier therefore answers
//! with the declaration containing it rather than with the fragment itself.
//!
//! # Decision: containment is a [`Span`]'s own half-open range
//!
//! An offset is inside a node when `start <= offset < end`, which is what a
//! `Span` already means; this file does not give one a second reading. Two
//! consequences are worth naming because M4B meets both:
//!
//! - An **empty span contains nothing.** A span with `start == end` is an
//!   insertion point, not a range — `rule:ide/recovery-is-explicit`'s
//!   `MemberName::Missing` marks a position, and the node that *carries* it is
//!   what an offset lands in.
//! - **A cursor at a node's end is between nodes.** `$u->` with the caret after
//!   the arrow is not inside the property access, because the access ends
//!   there. A caller asking "what am I at the end of" asks about the byte
//!   before the caret; the index answers about bytes, and a caret is not one.
//!
//! # What it spends
//!
//! One entry per statement, expression and member — a kind, a 12-byte span and
//! a parent id — plus the [`crate::walk::Node`] tree it is flattened from,
//! which is built and dropped inside [`SyntaxIndex::of_stmts`]. Both are
//! O(nodes in the file) and neither is cached: the index is rebuilt per
//! analysis, as its rule requires. If
//! `rule:ide/a-full-reanalysis-stays-under-a-bound` ever fails on that
//! intermediate tree, the move is to make the walk emit its nodes to a sink
//! both consumers share — not to write a second traversal here.

use nvs_diagnostics::{BytePos, Span};

use crate::ast::Stmt;
use crate::walk::{self, Node};

/// One node, as the index holds it: which production it is, and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexNode {
    /// The production's own spelling, the same one [`crate::walk::Node`] uses.
    pub kind: &'static str,
    /// The source range the production covers.
    pub span: Span,
}

/// The innermost node containing an offset, and its ancestors.
///
/// Innermost first, the outermost statement last, so `nodes()[0]` is what the
/// cursor is on and the rest is what it is inside. Empty when the offset is
/// inside no node at all — between two statements, in inline HTML, or past the
/// end of the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodePath {
    nodes: Vec<IndexNode>,
}

impl NodePath {
    /// The node the offset is directly inside, if any.
    #[must_use]
    pub fn innermost(&self) -> Option<IndexNode> {
        self.nodes.first().copied()
    }

    /// The whole path, innermost first.
    #[must_use]
    pub fn nodes(&self) -> &[IndexNode] {
        &self.nodes
    }

    /// Whether the offset was inside no node.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Every node of a parsed file by span, answering which one a byte offset is
/// inside.
#[derive(Clone, Debug, Default)]
pub struct SyntaxIndex {
    /// Pre-order, so a node always precedes everything it contains.
    entries: Vec<Entry>,
}

/// One node's row: what it is, where it is, and what contains it.
#[derive(Clone, Copy, Debug)]
struct Entry {
    kind: &'static str,
    span: Span,
    parent: Option<usize>,
}

impl SyntaxIndex {
    /// Builds the index by one walk over the statements of a parsed file.
    #[must_use]
    pub fn of_stmts(stmts: &[Stmt]) -> Self {
        let mut index = Self {
            entries: Vec::new(),
        };
        index.push_all(&walk::of_stmts(stmts), None);
        index
    }

    /// The innermost node containing `offset`, and its ancestors outward.
    ///
    /// The innermost is the *last* entry whose span contains the offset: an
    /// entry precedes everything it contains, and siblings cover disjoint
    /// ranges, so nothing after the deepest containing node can contain it too.
    /// The rest of the path is that entry's parents, which makes the answer a
    /// real path through the tree rather than whatever the spans happened to
    /// overlap.
    #[must_use]
    pub fn at(&self, offset: BytePos) -> NodePath {
        let Some(innermost) = self
            .entries
            .iter()
            .rposition(|entry| entry.span.start <= offset && offset < entry.span.end)
        else {
            return NodePath::default();
        };
        let mut nodes = Vec::new();
        let mut next = Some(innermost);
        while let Some(id) = next {
            let entry = self.entries[id];
            nodes.push(IndexNode {
                kind: entry.kind,
                span: entry.span,
            });
            next = entry.parent;
        }
        NodePath { nodes }
    }

    /// How many nodes the file has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the file has no nodes at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Appends `nodes` and their subtrees, in source order, under `parent`.
    fn push_all(&mut self, nodes: &[Node], parent: Option<usize>) {
        for node in nodes {
            let id = self.entries.len();
            self.entries.push(Entry {
                kind: node.kind,
                span: node.span,
                parent,
            });
            self.push_all(&node.children, Some(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap, Span};

    use super::SyntaxIndex;
    use crate::{parse, walk};

    /// Parses `source` and returns its index, with the offset of `needle`.
    fn index_of(source: &str, needle: &str) -> (SyntaxIndex, u32) {
        let mut map = SourceMap::new();
        let id = map.add("<test>", source);
        let mut diags = Diagnostics::new();
        let parsed = parse(map.file(id), &mut diags);
        assert!(
            !diags.has_errors(),
            "the fixture must parse cleanly: {:?}",
            diags.iter().find(|d| d.is_error()).map(|d| &d.message)
        );
        let at = u32::try_from(source.find(needle).expect("the needle is in the source"))
            .expect("a test fixture is small");
        (parsed.index, at)
    }

    /// Every node of `nodes` and their subtrees — what the index holds one
    /// entry each of.
    fn nodes_in(nodes: &[walk::Node]) -> usize {
        nodes.iter().map(|n| 1 + nodes_in(&n.children)).sum()
    }

    #[test]
    fn the_syntax_index_answers_the_innermost_node_at_an_offset() {
        let (index, at) = index_of("<?nvs echo 1 + 2;", "2;");
        let path = index.at(at);
        let innermost = path.innermost().expect("the literal");
        assert_eq!(innermost.kind, "Int");
        assert_eq!(path.nodes()[0], innermost, "the path opens at the cursor");
    }

    #[test]
    fn the_syntax_index_answers_the_ancestor_path_outward_in_order() {
        let (index, at) = index_of("<?nvs echo 1 + 2;", "2;");
        let path = index.at(at);
        let kinds: Vec<&str> = path.nodes().iter().map(|n| n.kind).collect();
        assert_eq!(kinds, ["Int", "Binary", "Echo"]);
        for pair in path.nodes().windows(2) {
            assert!(
                pair[1].span.start <= pair[0].span.start && pair[0].span.end <= pair[1].span.end,
                "{:?} is not inside the node that follows it, {:?}",
                pair[0],
                pair[1],
            );
        }
    }

    #[test]
    fn the_index_is_filled_by_one_walk() {
        // The index a parse answers with is [`crate::walk`]'s tree flattened
        // and nothing else — same nodes, same order, same parents — which is
        // the claim a second traversal added beside the walk would break
        // silently. Comparing the rows rather than the count is what catches
        // that: two walks agree on how many nodes a file has long after they
        // have stopped agreeing on what contains what.
        let source = "<?nvs class C { public function m(): void { echo 1 + 2; } } echo new C();";
        let mut map = SourceMap::new();
        let id = map.add("<test>", source);
        let mut diags = Diagnostics::new();
        let parsed = parse(map.file(id), &mut diags);
        assert!(!diags.has_errors(), "the fixture must parse cleanly");

        let rows = |index: &SyntaxIndex| -> Vec<(&'static str, Span, Option<usize>)> {
            index
                .entries
                .iter()
                .map(|entry| (entry.kind, entry.span, entry.parent))
                .collect()
        };
        assert_eq!(
            rows(&parsed.index),
            rows(&SyntaxIndex::of_stmts(&parsed.stmts))
        );
        assert_eq!(parsed.index.len(), nodes_in(&walk::of_stmts(&parsed.stmts)));
    }

    #[test]
    fn a_method_body_answers_through_its_member_and_its_class() {
        let (index, at) = index_of(
            "<?nvs class C { public function m(): void { echo 1; } }",
            "1;",
        );
        let kinds: Vec<&str> = index.at(at).nodes().iter().map(|n| n.kind).collect();
        assert_eq!(kinds, ["Int", "Echo", "Method", "ClassDecl"]);
    }

    #[test]
    fn an_offset_inside_no_node_answers_an_empty_path() {
        let (index, at) = index_of("<?nvs echo 1;    echo 2;", "    echo 2");
        assert!(
            index.at(at + 1).is_empty(),
            "the run between two statements"
        );
        assert_eq!(index.at(at + 1).innermost(), None);
        assert!(
            index.at(u32::MAX).is_empty(),
            "nor is anything past the end"
        );
    }

    #[test]
    fn every_node_is_inside_the_node_that_contains_it() {
        let source = "<?nvs
            class C {
                public int $n = 1 + 2;
                public function m(array<string> $xs): void {
                    foreach ($xs as string $x) { echo $x, match ($x) { \"a\" => 1, default => 2 }; }
                }
            }
            enum E: int { A = 1, B = 2 }
            echo (new C())->m([]);";
        let (index, _) = index_of(source, "class");
        assert!(
            index.len() > 20,
            "the fixture is a real tree: {}",
            index.len()
        );
        for (id, entry) in index.entries.iter().enumerate() {
            let Some(parent) = entry.parent else { continue };
            let outer = index.entries[parent].span;
            assert!(
                outer.start <= entry.span.start && entry.span.end <= outer.end,
                "node {id} ({}) escapes its parent ({})",
                entry.kind,
                index.entries[parent].kind,
            );
        }
    }

    #[test]
    fn a_cursor_at_a_nodes_end_is_outside_it() {
        let (index, at) = index_of("<?nvs echo 1;", "1;");
        let literal = index.at(at).innermost().expect("the literal");
        assert_eq!(literal.kind, "Int");
        let after = index
            .at(literal.span.end)
            .innermost()
            .expect("the statement");
        assert_eq!(
            after.kind, "Echo",
            "the caret past the literal is between nodes"
        );
    }
}
