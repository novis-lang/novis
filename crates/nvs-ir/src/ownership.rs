//! The ownership pass: it decides every move from the lowered IR, after
//! lowering and before code generation (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! The lowering decides nothing about moves. It emits a plain retain for every
//! argument a callee owns that another holder still has, and a plain release
//! wherever a holder's reference ends. This pass then reads each call and asks
//! one question of the IR: is the reference the call could take released on
//! every path after the call, exactly once, and read by nothing else? Where it
//! is, the call takes that reference, and the retain and those releases go.
//! The callee then holds the only reference, so its change is in place when
//! nothing else shares the value.
//!
//! # The two kinds of call
//!
//! - **A Novis callee** ([`InstKind::Call`], [`InstKind::CallVirtual`]) owns
//!   each argument on every path, its error exit included. The pass removes
//!   the [`InstKind::Retain`] the lowering put just before the call, and every
//!   release of the value on the paths from the call, the error edge
//!   included.
//! - **An owning `Core` member** ([`InstKind::CoreCall`]) takes its subject's
//!   reference only when it succeeds, and leaves it with the caller on an
//!   error (`nvs_runtime::owned`). The pass switches the call to the member's
//!   owning entry (`nvs_stdlib::owning_entry`), and removes the releases on
//!   the paths from the call's normal exit. The error edge keeps its releases.
//!   The switch is made only where [`InstKind::CoreCall::subject`] equals the
//!   entry's own slot, so a wrong slot can only miss a move.
//!
//! # The check
//!
//! A forward walk from the call follows every edge, error edges, loop back
//! edges and a generator's resume included, because they are all edges of
//! the IR. It fails, and the call keeps borrowing or keeps its retain, when a
//! path from the call:
//!
//! - reads the value in any way except one [`InstKind::Release`], a phi on
//!   that path's edge included;
//! - releases it twice, so a second holder had a reference of its own;
//! - leaves the function without releasing it, so a slot or a caller still
//!   holds that reference.
//!
//! Each release it found must also be unreachable from the function's entry
//! without passing the call, or another path would lose its release.
//!
//! **What it spends:** while compiling, one walk per candidate call over the
//! code after it. Nothing at run time: the program makes one retain and one
//! release fewer for each move.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::ValueId;
use crate::ir::{Function, InstKind, Terminator};
use crate::ty::Ty;

/// Decides every move in `functions`.
pub fn run(functions: &mut [Function]) {
    for f in functions {
        run_function(f);
    }
}

/// One instruction's place: the block's index in [`Function::blocks`] and the
/// instruction's index in it.
type At = (usize, usize);

/// What a call's move does once the check passes.
enum Move {
    /// A Novis callee: remove this retain.
    Novis { retain: At },
    /// An owning `Core` member: call this symbol instead.
    Core { owned: &'static str },
}

/// Decides every move in one function. Removed instructions are collected and
/// dropped at the end, so every [`At`] stays valid while the pass runs.
fn run_function(f: &mut Function) {
    let index: FxHashMap<u32, usize> = f
        .blocks
        .iter()
        .enumerate()
        .map(|(at, block)| (block.id.index(), at))
        .collect();
    let tys: FxHashMap<ValueId, Ty> = f
        .blocks
        .iter()
        .flat_map(|block| &block.insts)
        .filter_map(|inst| Some((inst.result?, inst.ty?)))
        .collect();
    let entry = index[&f.entry.index()];
    let mut removed: FxHashSet<At> = FxHashSet::default();
    for b in 0..f.blocks.len() {
        for c in 0..f.blocks[b].insts.len() {
            for (value, how) in candidates(f, (b, c), &tys, &removed) {
                let error_edge_owned = matches!(how, Move::Novis { .. });
                let walk = Walk {
                    f,
                    index: &index,
                    removed: &removed,
                    value,
                    call: (b, c),
                };
                let Some(releases) = walk.releases_after(error_edge_owned) else {
                    continue;
                };
                let avoid = walk.reachable_avoiding(entry, !error_edge_owned);
                if releases
                    .iter()
                    .any(|&(rb, ri)| !(rb == b && ri > c) && avoid.contains(&rb))
                {
                    continue;
                }
                removed.extend(releases);
                match how {
                    Move::Novis { retain } => {
                        removed.insert(retain);
                    }
                    Move::Core { owned } => {
                        if let InstKind::CoreCall { symbol, .. } = &mut f.blocks[b].insts[c].kind {
                            *symbol = owned;
                        }
                    }
                }
                // One move per call: a second argument with the same value
                // was rejected by `candidates`, and a different value is a
                // different reference the next call may move.
            }
        }
    }
    for (b, block) in f.blocks.iter_mut().enumerate() {
        let mut i = 0;
        block.insts.retain(|_| {
            let keep = !removed.contains(&(b, i));
            i += 1;
            keep
        });
    }
}

/// The values the call at `call` could take the reference of, each with what
/// its move does.
fn candidates(
    f: &Function,
    (b, c): At,
    tys: &FxHashMap<ValueId, Ty>,
    removed: &FxHashSet<At>,
) -> Vec<(ValueId, Move)> {
    let insts = &f.blocks[b].insts;
    match &insts[c].kind {
        InstKind::CoreCall {
            symbol,
            args,
            subject: Some(subject),
        } => {
            let Some(entry) = nvs_stdlib::owning_entry(symbol) else {
                return Vec::new();
            };
            let slot = *subject as usize;
            let Some(&value) = args.get(slot) else {
                return Vec::new();
            };
            if entry.slot != slot
                || args.iter().filter(|&&a| a == value).count() != 1
                || !matches!(tys.get(&value), Some(Ty::Array | Ty::Str | Ty::Bytes))
            {
                return Vec::new();
            }
            vec![(
                value,
                Move::Core {
                    owned: entry.symbol,
                },
            )]
        }
        InstKind::Call { receiver, args, .. } | InstKind::CallVirtual { receiver, args, .. } => {
            args.iter()
                .filter(|&&value| {
                    receiver != &Some(value) && args.iter().filter(|&&a| a == value).count() == 1
                })
                .filter_map(|&value| {
                    let retain = retain_before(insts, c, value, b, removed)?;
                    Some((
                        value,
                        Move::Novis {
                            retain: (b, retain),
                        },
                    ))
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// The index of the [`InstKind::Retain`] of `value` the lowering put before
/// the call at `c`, where nothing between the two can fail or reads `value`.
/// A failure between them would release the retained reference in its own
/// landing block, which the move would leave unbalanced.
fn retain_before(
    insts: &[crate::ir::Inst],
    c: usize,
    value: ValueId,
    b: usize,
    removed: &FxHashSet<At>,
) -> Option<usize> {
    for i in (0..c).rev() {
        if removed.contains(&(b, i)) {
            continue;
        }
        let inst = &insts[i];
        if let InstKind::Retain { operand } = inst.kind
            && operand == value
        {
            return Some(i);
        }
        if inst.on_error.is_some() || reads(&inst.kind, value) {
            return None;
        }
    }
    None
}

/// Whether `kind` reads `value`.
fn reads(kind: &InstKind, value: ValueId) -> bool {
    let mut found = false;
    kind.for_each_operand(|v| found |= v == value);
    found
}

/// One candidate's walk over the function.
struct Walk<'a> {
    f: &'a Function,
    index: &'a FxHashMap<u32, usize>,
    removed: &'a FxHashSet<At>,
    value: ValueId,
    call: At,
}

impl Walk<'_> {
    /// Every release of the value on the paths from the call, or `None` when a
    /// path reads it another way, releases it twice or ends without releasing
    /// it. `error_edge` says whether the call's own error edge is one of
    /// those paths: it is for a Novis callee, which owns its arguments on
    /// every path.
    fn releases_after(&self, error_edge: bool) -> Option<Vec<At>> {
        let (cb, ci) = self.call;
        let mut releases = Vec::new();
        let mut seen: FxHashSet<(usize, bool)> = FxHashSet::default();
        let mut work: Vec<(usize, usize, bool)> = vec![(cb, ci + 1, false)];
        if error_edge && let Some(landing) = self.f.blocks[cb].insts[ci].on_error {
            self.edge(cb, landing, false, &mut seen, &mut work)?;
        }
        while let Some((b, start, mut released)) = work.pop() {
            let block = &self.f.blocks[b];
            for (i, inst) in block.insts.iter().enumerate().skip(start) {
                if self.removed.contains(&(b, i)) || matches!(inst.kind, InstKind::Phi { .. }) {
                    continue;
                }
                match inst.kind {
                    InstKind::Release { operand } if operand == self.value => {
                        if released {
                            return None;
                        }
                        released = true;
                        releases.push((b, i));
                    }
                    ref kind if reads(kind, self.value) => return None,
                    _ => {}
                }
                if let Some(landing) = inst.on_error {
                    self.edge(b, landing, released, &mut seen, &mut work)?;
                }
            }
            let mut read = false;
            block.term.for_each_operand(|v| read |= v == self.value);
            if read {
                return None;
            }
            if matches!(
                block.term,
                Terminator::Return(_) | Terminator::Propagate { .. }
            ) && !released
            {
                return None;
            }
            for target in terminator_targets(&block.term) {
                self.edge(b, target, released, &mut seen, &mut work)?;
            }
        }
        releases.sort_unstable();
        releases.dedup();
        Some(releases)
    }

    /// Follows the edge from block `from` into `to`, or `None` when a phi of
    /// `to` reads the value on that edge.
    fn edge(
        &self,
        from: usize,
        to: crate::ids::BlockId,
        released: bool,
        seen: &mut FxHashSet<(usize, bool)>,
        work: &mut Vec<(usize, usize, bool)>,
    ) -> Option<()> {
        let to = self.index[&to.index()];
        let from_id = self.f.blocks[from].id;
        for inst in &self.f.blocks[to].insts {
            if let InstKind::Phi { incoming } = &inst.kind
                && incoming
                    .iter()
                    .any(|&(pred, v)| pred == from_id && v == self.value)
            {
                return None;
            }
        }
        if seen.insert((to, released)) {
            work.push((to, 0, released));
        }
        Some(())
    }

    /// Every block reachable from `entry` without passing the call. With
    /// `through_error_edge`, the call's own error edge is one such path, as
    /// it is for an owning `Core` member, which leaves the value with the
    /// caller when it fails.
    fn reachable_avoiding(&self, entry: usize, through_error_edge: bool) -> FxHashSet<usize> {
        let (cb, ci) = self.call;
        let mut reached = FxHashSet::default();
        let mut work = vec![entry];
        reached.insert(entry);
        while let Some(b) = work.pop() {
            let block = &self.f.blocks[b];
            let next: Vec<crate::ids::BlockId> = if b == cb {
                let last = if through_error_edge { ci + 1 } else { ci };
                block.insts[..last]
                    .iter()
                    .filter_map(|inst| inst.on_error)
                    .collect()
            } else {
                block.successors()
            };
            for target in next {
                let at = self.index[&target.index()];
                if reached.insert(at) {
                    work.push(at);
                }
            }
        }
        reached
    }
}

/// The blocks a terminator can jump to.
fn terminator_targets(term: &Terminator) -> Vec<crate::ids::BlockId> {
    match term {
        Terminator::Return(_) | Terminator::Propagate { .. } => Vec::new(),
        Terminator::Jump(target) => vec![*target],
        Terminator::Throw { landing, .. } => vec![*landing],
        Terminator::Catch { handler, onward } => vec![*handler, *onward],
        Terminator::Switch { arms, default, .. } => {
            let mut out: Vec<_> = arms.iter().map(|(_, target, _)| *target).collect();
            out.push(*default);
            out
        }
        Terminator::Branch {
            then_block,
            else_block,
            ..
        } => vec![*then_block, *else_block],
    }
}
