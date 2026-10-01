//! Stable per-statement and per-conditional-edge identifiers.
//!
//! `rule:testing/debug-probes`
//! needs every lowered statement and every conditional CFG edge to carry an id
//! a coverage/branch probe can address, so the ids are reserved here rather
//! than by the code that consumes them: introducing them once M3's codegen
//! builds probe sites on top of the IR would mean re-numbering (and
//! re-validating) every already-lowered program, exactly the "cheap now,
//! expensive later" case `docs/implementation-plan.md`'s M2 paragraph calls
//! out by name.
//!
//! # Stability
//!
//! An id is stable across recompiles of the *same* source: [`lower`](crate::lower)
//! assigns [`StmtId`]s and [`EdgeId`]s in one deterministic pre-order walk of
//! the checked AST, so lowering an unchanged file always reproduces the same
//! numbering. That is the property a probe site needs — `rule:testing/debug-probes` attaches
//! "line N was hit" state to an id captured once when tracing starts, and
//! that id must still mean the same statement the next time the file is
//! compiled unchanged.
//!
//! Ids are scoped to one [`crate::ir::Function`], not process-wide: two
//! different functions each start counting from zero, so an edit in one
//! function never renumbers another. The coverage probe needs one number per
//! statement in the whole program, and codegen makes it by adding a
//! per-function base when it emits the probe
//! ([`Program::stmt_spans`](crate::Program::stmt_spans)), which leaves these
//! ids as they are.

use nvs_diagnostics::Span;

macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub struct $name(u32);

        impl $name {
            /// This id's raw index — the position it was assigned in the
            /// deterministic lowering walk.
            #[must_use]
            pub const fn index(self) -> u32 {
                self.0
            }
        }
    };
}

id_newtype!(
    /// One lowered statement — `rule:testing/debug-probes`'s coverage-probe unit.
    StmtId
);
id_newtype!(
    /// One conditional CFG edge — `rule:testing/debug-probes`'s branch-probe unit. Constructed
    /// by [`lower`](crate::lower) for each outgoing edge of an `if`/`while`'s
    /// [`crate::ir::Terminator::Branch`]; `for`/`switch`/`try` will add more
    /// once they land, with no renumbering of ids already handed out.
    EdgeId
);
id_newtype!(
    /// One basic block within a [`crate::ir::Function`].
    BlockId
);
id_newtype!(
    /// One SSA value within a [`crate::ir::Function`].
    ValueId
);

/// Hands out [`StmtId`]/[`EdgeId`]/[`BlockId`]/[`ValueId`]s in lowering
/// order, and records the [`Span`] each [`StmtId`]/[`EdgeId`] carries for
/// provenance — the same span a diagnostic about that statement would
/// already point at.
#[derive(Debug, Default)]
pub struct IdGen {
    stmt_spans: Vec<Span>,
    edge_spans: Vec<Span>,
    next_block: u32,
    next_value: u32,
}

impl IdGen {
    /// A fresh generator for one function, numbering from zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates the next statement id, recording `span` for later lookup via
    /// [`Self::into_spans`].
    ///
    /// # Panics
    ///
    /// Panics if a single function has lowered more than [`u32::MAX`]
    /// statements — not a real limit, the same "this would already have
    /// failed a saner earlier check" class of bound `nvs-types` documents
    /// elsewhere for its own counters.
    pub fn next_stmt(&mut self, span: Span) -> StmtId {
        let id = StmtId(
            u32::try_from(self.stmt_spans.len())
                .expect("more statements in one function than fit a stable id"),
        );
        self.stmt_spans.push(span);
        id
    }

    /// Allocates the next conditional-edge id, recording `span`.
    ///
    /// # Panics
    ///
    /// See [`Self::next_stmt`].
    pub fn next_edge(&mut self, span: Span) -> EdgeId {
        let id = EdgeId(
            u32::try_from(self.edge_spans.len())
                .expect("more conditional edges in one function than fit a stable id"),
        );
        self.edge_spans.push(span);
        id
    }

    /// Allocates the next basic-block id.
    pub fn next_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    /// Allocates the next SSA value id.
    pub fn next_value(&mut self) -> ValueId {
        let id = ValueId(self.next_value);
        self.next_value += 1;
        id
    }

    /// Consumes the generator, returning every recorded statement span and
    /// every recorded edge span, indexed by the id's own
    /// [`StmtId::index`]/[`EdgeId::index`] — the tables [`crate::ir::Function`]
    /// keeps as `stmt_spans`/`edge_spans`.
    #[must_use]
    pub fn into_spans(self) -> (Vec<Span>, Vec<Span>) {
        (self.stmt_spans, self.edge_spans)
    }
}
