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
//! One entry per statement, expression and member — a kind, a 12-byte span, a
//! parent id and the id where its subtree ends — plus the
//! [`crate::walk::Node`] tree it is flattened from,
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
    /// Whether the entries' starts never decrease, which is what the
    /// logarithmic lookups need. The walk emits source order, so this holds
    /// for every tree it builds; a tree that broke it would be answered by a
    /// scan, slowly and still correctly.
    ordered: bool,
}

/// One node's row: what it is, where it is, and what contains it.
#[derive(Clone, Copy, Debug)]
struct Entry {
    kind: &'static str,
    span: Span,
    parent: Option<usize>,
    /// One past the last entry of this node's subtree, so the next sibling's id.
    end: usize,
}

impl SyntaxIndex {
    /// Builds the index by one walk over the statements of a parsed file.
    #[must_use]
    pub fn of_stmts(stmts: &[Stmt]) -> Self {
        let mut index = Self {
            entries: Vec::new(),
            ordered: true,
        };
        index.push_all(&walk::of_stmts(stmts), None);
        index.ordered = index
            .entries
            .windows(2)
            .all(|pair| pair[0].span.start <= pair[1].span.start);
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
        let Some(innermost) = self.innermost(offset) else {
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

    /// The nodes `node` directly contains, in source order.
    ///
    /// The one question that goes *down* rather than up. [`Self::at`] answers a
    /// cursor, and a cursor is always inside the node it is asking about — but
    /// the member half of `$u->` is inside no node of its own, so what
    /// `nvs_lsp::completion` needs is the receiver *beside* it: the access's
    /// first child. Reaching it by scanning the source back over an arrow
    /// would be this crate's grammar re-read in another one.
    ///
    /// Matched on the whole node rather than on its span alone, because a
    /// statement and the expression it wraps cover the same bytes and only the
    /// kind tells them apart. A path from [`Self::at`] is what supplies one, so
    /// the caller is asking about a node this index really has.
    #[must_use]
    pub fn children_of(&self, node: IndexNode) -> Vec<IndexNode> {
        let Some(parent) = self.find(node) else {
            return Vec::new();
        };
        let mut children = Vec::new();
        let mut child = parent + 1;
        while child < self.entries[parent].end {
            let entry = self.entries[child];
            children.push(IndexNode {
                kind: entry.kind,
                span: entry.span,
            });
            child = entry.end;
        }
        children
    }

    /// The id of the last entry containing `offset`.
    ///
    /// Ordered entries are searched for the last one starting at or before the
    /// offset. The innermost containing node is that entry or one of its
    /// ancestors: any entry after the innermost node's subtree starts at or past
    /// its end, and so past the offset. Climbing the parents to the first one
    /// that contains the offset therefore finds it, in O(log n + depth).
    fn innermost(&self, offset: BytePos) -> Option<usize> {
        if !self.ordered {
            return self
                .entries
                .iter()
                .rposition(|entry| entry.span.start <= offset && offset < entry.span.end);
        }
        let after = self
            .entries
            .partition_point(|entry| entry.span.start <= offset);
        let mut next = after.checked_sub(1);
        while let Some(id) = next {
            let entry = self.entries[id];
            if offset < entry.span.end && entry.span.start < entry.span.end {
                return Some(id);
            }
            next = entry.parent;
        }
        None
    }

    /// The id of the first entry that is `node`, kind and span both.
    ///
    /// Ordered entries that start where `node` does are one contiguous run,
    /// found by binary search; it is as long as the nodes sharing that start.
    fn find(&self, node: IndexNode) -> Option<usize> {
        let is_node = |entry: &Entry| entry.kind == node.kind && entry.span == node.span;
        if !self.ordered {
            return self.entries.iter().position(is_node);
        }
        let from = self
            .entries
            .partition_point(|entry| entry.span.start < node.span.start);
        self.entries[from..]
            .iter()
            .take_while(|entry| entry.span.start == node.span.start)
            .position(is_node)
            .map(|at| from + at)
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
                end: id + 1,
            });
            self.push_all(&node.children, Some(id));
            self.entries[id].end = self.entries.len();
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
    fn the_search_answers_what_a_scan_of_every_entry_answers() {
        // The binary search and the climb are an optimisation of a scan, so
        // they must agree with it at every byte of a real tree, the gaps
        // between nodes and the bytes past the end included.
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
        assert!(index.ordered, "the walk emits source order");
        let len = u32::try_from(source.len()).expect("a test fixture is small");
        for offset in 0..len + 2 {
            let scanned = index
                .entries
                .iter()
                .rposition(|e| e.span.start <= offset && offset < e.span.end);
            assert_eq!(index.innermost(offset), scanned, "at byte {offset}");
            let Some(id) = scanned else { continue };
            let node = super::IndexNode {
                kind: index.entries[id].kind,
                span: index.entries[id].span,
            };
            let first = index
                .entries
                .iter()
                .position(|e| e.kind == node.kind && e.span == node.span);
            assert_eq!(index.find(node), first, "the node at byte {offset}");
            let children: Vec<super::IndexNode> = index
                .entries
                .iter()
                .filter(|e| e.parent == first)
                .map(|e| super::IndexNode {
                    kind: e.kind,
                    span: e.span,
                })
                .collect();
            assert_eq!(index.children_of(node), children, "at byte {offset}");
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
