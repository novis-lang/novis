//! The borrow verifier: after the ownership pass, a value read from an
//! element or a property with no retain of its own is never read after the
//! value it was read from may have been freed or changed
//! (`rule:expressions/an-operand-keeps-its-value-while-later-operands-run`).
//!
//! It runs in test and debug builds alone, on every function the pass leaves,
//! and a release build compiles none of it. The lowering and the pass decide
//! every retain; this walk only checks what they decided, so a borrow the
//! lowering left unretained and the pass then outlived fails the debug suite
//! at the function that has it.
//!
//! # What it reads
//!
//! **A borrowed value** is the result of one of these, when its type is
//! counted ([`Ty::is_refcounted`]):
//!
//! - [`InstKind::ArrayGet`], borrowed from its array;
//! - [`InstKind::ArrayValueAt`], borrowed from the array a `foreach` walks;
//! - [`InstKind::FieldGet`], borrowed from its object.
//!
//! A borrowed value read from another borrowed value that is never retained
//! is borrowed from that value's owners too.
//!
//! **An owner is given up** by one of these, unless a [`InstKind::Retain`] of
//! the owner since the borrow's read paid for it:
//!
//! - an [`InstKind::Release`] of the owner;
//! - an argument of [`InstKind::Call`], [`InstKind::CallVirtual`] or a
//!   constructor's [`InstKind::New`], on the normal exit and the error edge,
//!   because a Novis callee owns its arguments on every path;
//! - the subject of a [`InstKind::CoreCall`] switched to its owning entry, on
//!   the normal exit alone;
//! - the operand [`InstKind::consumes`] names, on the normal exit alone.
//!
//! A path from the borrow's read that meets a [`InstKind::Retain`] of the
//! borrowed value before its owner is given up is safe. A path that reads the
//! borrowed value after that, in an instruction, a phi or a terminator, is
//! refused. A path that reaches the read again, on a loop's back edge, ends.
//!
//! # What a missing row costs
//!
//! A borrowing instruction or a way to give up an owner that is not listed
//! above is a missed check, never a wrong program: the verifier changes
//! nothing it walks. That is why this list may live here and not in the pass.
//! A phi of a borrowed value is not followed, which misses a check too.
//!
//! **What it spends:** in a test or debug build, one walk per borrowed value
//! over the code after its read, while compiling. Nothing in a release build.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::{BlockId, ValueId};
use crate::ir::{Function, Inst, InstKind, Terminator};

/// The one place in a function where a borrowed value is read after its
/// owner was given up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The function's name.
    pub function: String,
    /// The borrowed value.
    pub borrowed: ValueId,
    /// The owner it was read from, which was given up.
    pub owner: ValueId,
    /// The block of the read after the owner was given up.
    pub block: BlockId,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {:?} is borrowed from {:?}, and it is read in block {:?} after {:?} was given up \
             with no retain of its own",
            self.function, self.borrowed, self.owner, self.block, self.owner
        )
    }
}

/// Checks every borrowed value of `f`, and returns the first one read after
/// its owner was given up.
///
/// # Errors
///
/// The [`Refusal`] that names the borrowed value, its owner and the block of
/// the read.
pub fn verify(f: &Function) -> Result<(), Refusal> {
    let index: FxHashMap<u32, usize> = f
        .blocks
        .iter()
        .enumerate()
        .map(|(at, block)| (block.id.index(), at))
        .collect();
    let mut retained: FxHashSet<ValueId> = FxHashSet::default();
    let mut borrowed: Vec<(At, ValueId, ValueId)> = Vec::new();
    let mut base_of: FxHashMap<ValueId, ValueId> = FxHashMap::default();
    for (b, block) in f.blocks.iter().enumerate() {
        for (i, inst) in block.insts.iter().enumerate() {
            if let InstKind::Retain { operand } = inst.kind {
                retained.insert(operand);
            }
            let (Some(result), Some(base)) = (inst.result, borrowed_from(&inst.kind)) else {
                continue;
            };
            if inst.ty.is_some_and(crate::ty::Ty::is_refcounted) {
                borrowed.push(((b, i), result, base));
                base_of.insert(result, base);
            }
        }
    }
    for &(at, value, base) in &borrowed {
        let mut owner = Some(base);
        while let Some(o) = owner {
            let walk = Walk {
                f,
                index: &index,
                value,
                owner: o,
            };
            if let Some(block) = walk.read_after_given_up(at) {
                return Err(Refusal {
                    function: f.name.clone(),
                    borrowed: value,
                    owner: o,
                    block,
                });
            }
            owner = base_of.get(&o).copied().filter(|o2| {
                // A base that is retained somewhere holds a reference of its
                // own, so its owner's end does not free it.
                !retained.contains(&o) && *o2 != value
            });
        }
    }
    Ok(())
}

/// A position in a function: the block's index and the instruction's.
type At = (usize, usize);

/// The value `kind` borrows its result from, when it is one of the borrowing
/// instructions the module comment lists.
fn borrowed_from(kind: &InstKind) -> Option<ValueId> {
    match kind {
        InstKind::ArrayGet { array, .. } | InstKind::ArrayValueAt { array, .. } => Some(*array),
        InstKind::FieldGet { object, .. } => Some(*object),
        _ => None,
    }
}

/// How an instruction gives up `owner`: on its normal exit, and on its error
/// edge too.
fn gives_up(inst: &Inst, owner: ValueId) -> (bool, bool) {
    match &inst.kind {
        InstKind::Release { operand } => (*operand == owner, false),
        InstKind::Call { args, .. }
        | InstKind::CallVirtual { args, .. }
        | InstKind::New {
            ctor: Some(_),
            args,
            ..
        } => {
            let given = args.contains(&owner);
            (given, given)
        }
        InstKind::CoreCall {
            symbol,
            args,
            subject: Some(subject),
            ..
        } => {
            let owning = symbol
                .strip_suffix("_owned")
                .and_then(nvs_stdlib::owning_entry)
                .is_some_and(|entry| entry.symbol == *symbol && entry.slot == *subject as usize);
            (owning && args.get(*subject as usize) == Some(&owner), false)
        }
        kind => (kind.consumes() == Some(owner), false),
    }
}

/// Whether `kind` reads `value`.
fn reads(kind: &InstKind, value: ValueId) -> bool {
    let mut found = false;
    kind.for_each_operand(|v| found |= v == value);
    found
}

/// One borrowed value and one of its owners.
struct Walk<'a> {
    f: &'a Function,
    index: &'a FxHashMap<u32, usize>,
    value: ValueId,
    owner: ValueId,
}

/// The state of one path: how many retains of the owner since the read are
/// not yet given up, capped, and whether the owner was given up.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct State {
    paid: u8,
    given_up: bool,
}

impl State {
    /// The state after the owner is given up once.
    fn give_up(self) -> Self {
        match self.paid.checked_sub(1) {
            Some(paid) => Self { paid, ..self },
            None => Self {
                paid: 0,
                given_up: true,
            },
        }
    }
}

/// The most retains of the owner a path counts.
const MAX_PAID: u8 = 4;

impl Walk<'_> {
    /// The block where a path from the read at `(rb, ri)` reads the borrowed
    /// value after the owner was given up, or `None` when no path does.
    fn read_after_given_up(&self, (rb, ri): At) -> Option<BlockId> {
        let start = State {
            paid: 0,
            given_up: false,
        };
        let mut seen: FxHashSet<(usize, State)> = FxHashSet::default();
        let mut work: Vec<(usize, usize, State)> = vec![(rb, ri + 1, start)];
        'paths: while let Some((b, from, mut state)) = work.pop() {
            let block = &self.f.blocks[b];
            for inst in block.insts.iter().skip(from) {
                if inst.result == Some(self.value) {
                    continue 'paths;
                }
                if matches!(inst.kind, InstKind::Phi { .. }) {
                    continue;
                }
                if state.given_up && reads(&inst.kind, self.value) {
                    return Some(block.id);
                }
                match inst.kind {
                    InstKind::Retain { operand } if operand == self.value => continue 'paths,
                    InstKind::Retain { operand } if operand == self.owner => {
                        state.paid = (state.paid + 1).min(MAX_PAID);
                    }
                    _ => {}
                }
                let (normal, error) = gives_up(inst, self.owner);
                if let Some(landing) = inst.on_error {
                    let on_error = if error { state.give_up() } else { state };
                    if let Some(at) = self.edge(b, landing, on_error, &mut seen, &mut work) {
                        return Some(at);
                    }
                }
                if normal {
                    state = state.give_up();
                }
            }
            if state.given_up {
                let mut read = false;
                block.term.for_each_operand(|v| read |= v == self.value);
                if read {
                    return Some(block.id);
                }
            }
            for target in targets(&block.term) {
                if let Some(at) = self.edge(b, target, state, &mut seen, &mut work) {
                    return Some(at);
                }
            }
        }
        None
    }

    /// Follows the edge from block `from` into `to`. Returns the block `to`
    /// when one of its phis reads the borrowed value on that edge after the
    /// owner was given up.
    fn edge(
        &self,
        from: usize,
        to: BlockId,
        state: State,
        seen: &mut FxHashSet<(usize, State)>,
        work: &mut Vec<(usize, usize, State)>,
    ) -> Option<BlockId> {
        let at = self.index[&to.index()];
        let from_id = self.f.blocks[from].id;
        if state.given_up {
            for inst in &self.f.blocks[at].insts {
                if let InstKind::Phi { incoming } = &inst.kind
                    && incoming
                        .iter()
                        .any(|&(pred, v)| pred == from_id && v == self.value)
                {
                    return Some(to);
                }
            }
        }
        if seen.insert((at, state)) {
            work.push((at, 0, state));
        }
        None
    }
}

/// The blocks a terminator can jump to.
fn targets(term: &Terminator) -> Vec<BlockId> {
    super::terminator_targets(term)
}
