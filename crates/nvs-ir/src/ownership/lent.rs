//! A Novis function's lent body for one parameter
//! (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! A call that lends a slot to a Novis callee passes the slot's own reference
//! with no retain. The callee then owns that reference only when it returns:
//! it changes the value, returns it or releases it. When it fails, the
//! reference stays with the caller, and the slot must still hold the old
//! value. That is the protocol of an owning `Core` entry.
//!
//! The lent body is a copy of the callee with three changes:
//!
//! - A release of the parameter on a path that can only end in a failure is
//!   removed, so the caller's slot keeps its value.
//! - A change of the parameter after which a failure, or a read of a slot it
//!   is lent, is reachable separates first. A retain before the change makes
//!   the change copy, and a release before each return the change reaches
//!   gives up the caller's reference there. Every other change is in place.
//! - When the change can itself fail, its own landing block keeps its release
//!   of the parameter, which is the separate's retain.
//!
//! A shape this does not cover has no lent body, and the call keeps its
//! retain: the parameter read by anything but a release, a change, a borrowed
//! `Core` or helper call, an element read or a `return`; a release from which
//! both a failure and a return are reachable; and a separated change whose
//! return is reachable another way, or whose landing block has another way
//! in.
//!
//! **What it spends:** one more compiled body per function and parameter a
//! call lends, and while compiling one walk per release and change of that
//! parameter.

use rustc_hash::{FxHashMap, FxHashSet};

use super::{At, stmt_at, terminator_targets};
use crate::copies::{Fix, KeptCopy, Reason};
use crate::effects::{OwnedSlot, Summaries, Summary, Touches, callback, types};
use crate::ids::{BlockId, ValueId};
use crate::ir::{Function, Inst, InstKind, Terminator};

/// What one use of the parameter is.
enum Use {
    Release,
    Change,
    Borrow,
}

/// The lent body of `callee` for its parameter `param`, named
/// `<callee>#lent<param>`, or `None` when its shape is not covered. `slots`
/// are the slots its calls lend it. With it come the copies it keeps: one
/// per change that separates, with its [`Fix`] where one exists
/// ([`Reach::fix`]).
pub(super) fn body(
    callee: &Function,
    param: usize,
    slots: &FxHashSet<OwnedSlot>,
    summaries: &Summaries,
) -> Option<(Function, Vec<KeptCopy>)> {
    let index: FxHashMap<u32, usize> = callee
        .blocks
        .iter()
        .enumerate()
        .map(|(at, block)| (block.id.index(), at))
        .collect();
    let p = callee
        .blocks
        .iter()
        .flat_map(|block| &block.insts)
        .find(|inst| matches!(inst.kind, InstKind::Param(k) if k as usize == param))?
        .result?;
    let mut releases = Vec::new();
    let mut changes = Vec::new();
    for (b, block) in callee.blocks.iter().enumerate() {
        for (i, inst) in block.insts.iter().enumerate() {
            match use_of(&inst.kind, p)? {
                Some(Use::Release) => releases.push((b, i)),
                Some(Use::Change) => changes.push((b, i)),
                Some(Use::Borrow) | None => {}
            }
        }
        let mut read = false;
        block.term.for_each_operand(|v| read |= v == p);
        if read && !matches!(block.term, Terminator::Return(Some(v)) if v == p) {
            return None;
        }
    }
    let walk = Reach {
        f: callee,
        index: &index,
        slots,
        summaries,
        tys: types(callee),
        defs: callee
            .blocks
            .iter()
            .flat_map(|block| &block.insts)
            .filter_map(|inst| Some((inst.result?, &inst.kind)))
            .collect(),
    };
    let mut removed: FxHashSet<At> = FxHashSet::default();
    for &(b, i) in &releases {
        let reached = walk.from((b, i));
        match (reached.fails, reached.returns.is_empty()) {
            (true, true) => {
                removed.insert((b, i));
            }
            (true, false) => return None,
            (false, _) => {}
        }
    }
    let entry = index[&callee.entry.index()];
    let mut retain_before: FxHashSet<At> = FxHashSet::default();
    let mut release_before_return: FxHashSet<usize> = FxHashSet::default();
    let mut kept = Vec::new();
    for &(b, i) in &changes {
        let reached = walk.from((b, i));
        if !reached.fails && !reached.reads {
            continue;
        }
        let avoid = walk.avoiding(entry, (b, i));
        if reached
            .returns
            .iter()
            .any(|&rb| rb != b && avoid.contains(&rb))
        {
            return None;
        }
        retain_before.insert((b, i));
        let reason = if reached.reads {
            Reason::ReadAfterChange
        } else {
            Reason::FailureAfterChange
        };
        if let Some((_, span)) = stmt_at(callee, (b, i)) {
            kept.push(KeptCopy {
                span,
                reason,
                fix: walk.fix((b, i), reason),
            });
        }
        release_before_return.extend(reached.returns.iter().copied());
        if let Some(landing) = callee.blocks[b].insts[i].on_error {
            let landing = index[&landing.index()];
            if walk.ways_in(callee.blocks[landing].id) != 1 {
                return None;
            }
            let kept = releases.iter().find(|&&(rb, _)| rb == landing)?;
            removed.remove(kept);
        }
    }
    let mut lent = callee.clone();
    lent.name = format!("{}#lent{param}", callee.name);
    for (b, block) in lent.blocks.iter_mut().enumerate() {
        let insts = std::mem::take(&mut block.insts);
        for (i, inst) in insts.into_iter().enumerate() {
            if retain_before.contains(&(b, i)) {
                block.insts.push(plain(InstKind::Retain { operand: p }));
            }
            if !removed.contains(&(b, i)) {
                block.insts.push(inst);
            }
        }
        if release_before_return.contains(&b) {
            block.insts.push(plain(InstKind::Release { operand: p }));
        }
    }
    Some((lent, kept))
}

/// An instruction with no result and no error edge.
fn plain(kind: InstKind) -> Inst {
    Inst {
        result: None,
        ty: None,
        kind,
        on_error: None,
        raise_site: None,
    }
}

/// What `kind` does with `p`: `Ok(None)` when it does not read it, and
/// `None` when the use is not one a lent body covers.
fn use_of(kind: &InstKind, p: ValueId) -> Option<Option<Use>> {
    let mut read = false;
    kind.for_each_operand(|v| read |= v == p);
    if !read {
        return Some(None);
    }
    if kind.consumes() == Some(p) {
        return Some(Some(Use::Change));
    }
    Some(Some(match kind {
        InstKind::Release { .. } => Use::Release,
        InstKind::CoreCall {
            symbol,
            args,
            subject,
        } => {
            let owned = nvs_stdlib::owning_entry(symbol).is_none() && symbol.ends_with("_owned");
            let on_subject = subject.and_then(|s| args.get(s as usize)) == Some(&p);
            match (owned, on_subject) {
                (true, true) => Use::Change,
                (true, false) => return None,
                (false, _) => Use::Borrow,
            }
        }
        InstKind::HelperCall { helper, .. } if !helper.may_run_user_code() => Use::Borrow,
        InstKind::ArrayGet { array, .. } if *array == p => Use::Borrow,
        _ => return None,
    }))
}

/// What is reachable after one instruction of the callee.
struct Reached {
    /// A path ends in a failure: a [`Terminator::Propagate`].
    fails: bool,
    /// An instruction may read a slot the body is lent.
    reads: bool,
    /// The blocks whose [`Terminator::Return`] a path ends in.
    returns: Vec<usize>,
}

/// The callee's walks.
struct Reach<'a> {
    f: &'a Function,
    index: &'a FxHashMap<u32, usize>,
    slots: &'a FxHashSet<OwnedSlot>,
    summaries: &'a Summaries,
    tys: FxHashMap<ValueId, crate::ty::Ty>,
    defs: FxHashMap<ValueId, &'a InstKind>,
}

impl Reach<'_> {
    /// Everything reachable after the instruction at `(b, i)`, following each
    /// later instruction's error edge and every terminator. The instruction's
    /// own error edge is not followed: a change that fails leaves its operand
    /// unchanged.
    fn from(&self, (b, i): At) -> Reached {
        let mut reached = Reached {
            fails: false,
            reads: false,
            returns: Vec::new(),
        };
        let mut seen: FxHashSet<usize> = FxHashSet::default();
        let mut work = vec![(b, i + 1)];
        while let Some((b, start)) = work.pop() {
            let block = &self.f.blocks[b];
            let mut next: Vec<BlockId> = Vec::new();
            for inst in block.insts.iter().skip(start) {
                if !reached.reads && self.reads_a_slot(&inst.kind) {
                    reached.reads = true;
                }
                next.extend(inst.on_error);
            }
            match block.term {
                Terminator::Propagate { .. } => reached.fails = true,
                Terminator::Return(_) => reached.returns.push(b),
                _ => {}
            }
            next.extend(terminator_targets(&block.term));
            for target in next {
                let at = self.index[&target.index()];
                if seen.insert(at) {
                    work.push((at, 0));
                }
            }
        }
        reached.returns.sort_unstable();
        reached.returns.dedup();
        reached
    }

    /// The fix for the change at `(b, i)`, which copies for `reason`: move
    /// the first statement after it that reads a lent slot, or that can
    /// fail, above the change's own statement.
    ///
    /// The fix is offered only where the move cannot change what the code
    /// does. Both statements are in one block, and the moved one ends before
    /// the block's terminator. It reads no value made from the change's
    /// statement on, and no value it makes is read after it. It writes no
    /// lent slot and no slot the statements it passes read or write, and it
    /// reads no slot they write.
    fn fix(&self, (b, i): At, reason: Reason) -> Option<Fix> {
        let insts = &self.f.blocks[b].insts;
        let offender = (i + 1..insts.len()).find(|&j| match reason {
            Reason::ReadAfterChange => self.reads_a_slot(&insts[j].kind),
            _ => {
                insts[j].on_error.is_some()
                    && !matches!(insts[j].kind, InstKind::Safepoint)
            }
        })?;
        let (start, change) = stmt_at(self.f, (b, i))?;
        let (from, moved) = stmt_at(self.f, (b, offender))?;
        if from <= i {
            return None;
        }
        let to = (from + 1..insts.len())
            .find(|&j| matches!(insts[j].kind, InstKind::StmtMarker(_)))?;
        let made_between: FxHashSet<ValueId> =
            insts[start..from].iter().filter_map(|inst| inst.result).collect();
        let made_in: FxHashSet<ValueId> =
            insts[from..to].iter().filter_map(|inst| inst.result).collect();
        let mut reads_between = false;
        for inst in &insts[from..to] {
            inst.kind
                .for_each_operand(|v| reads_between |= made_between.contains(&v));
        }
        if reads_between || self.read_outside(&made_in, b, from, to) {
            return None;
        }
        let window = self.effects_of(&insts[start..from]);
        let stmt = self.effects_of(&insts[from..to]);
        let lent = Touches::Slots(self.slots.clone());
        if stmt.writes.meets(&lent)
            || stmt.writes.meets(&window.reads)
            || stmt.writes.meets(&window.writes)
            || stmt.reads.meets(&window.writes)
        {
            return None;
        }
        Some(Fix {
            moved,
            before: change,
        })
    }

    /// Whether anything outside block `b`'s instructions `from..to`, a
    /// release aside, reads one of `values`.
    fn read_outside(&self, values: &FxHashSet<ValueId>, b: usize, from: usize, to: usize) -> bool {
        let mut found = false;
        for (ib, block) in self.f.blocks.iter().enumerate() {
            for (j, inst) in block.insts.iter().enumerate() {
                if (ib == b && (from..to).contains(&j))
                    || matches!(inst.kind, InstKind::Release { .. })
                {
                    continue;
                }
                inst.kind.for_each_operand(|v| found |= values.contains(&v));
            }
            block.term.for_each_operand(|v| found |= values.contains(&v));
        }
        found
    }

    /// Everything `insts` may read and write, their callees included.
    fn effects_of(&self, insts: &[Inst]) -> Summary {
        let mut all = Summary::default();
        for inst in insts {
            let one = self.summaries.of_inst(&inst.kind, &self.tys, |v| {
                self.defs.get(&v).and_then(|kind| callback(kind))
            });
            all.reads.join(&one.reads);
            all.writes.join(&one.writes);
        }
        all
    }

    /// Whether `kind` may read one of the slots the body is lent.
    fn reads_a_slot(&self, kind: &InstKind) -> bool {
        if self.slots.is_empty() {
            return false;
        }
        let reads = self
            .summaries
            .of_inst(kind, &self.tys, |v| {
                self.defs.get(&v).and_then(|kind| callback(kind))
            })
            .reads;
        match reads {
            Touches::Anything => true,
            Touches::Slots(read) => read.iter().any(|s| self.slots.contains(s)),
        }
    }

    /// Every block reachable from `entry` without passing the instruction at
    /// `(cb, ci)`. Its own error edge is such a path, because a change that
    /// fails has not changed anything.
    fn avoiding(&self, entry: usize, (cb, ci): At) -> FxHashSet<usize> {
        let mut reached = FxHashSet::default();
        reached.insert(entry);
        let mut work = vec![entry];
        while let Some(b) = work.pop() {
            let block = &self.f.blocks[b];
            let next: Vec<BlockId> = if b == cb {
                block.insts[..=ci]
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

    /// How many error edges and terminators enter `block`.
    fn ways_in(&self, block: BlockId) -> usize {
        self.f
            .blocks
            .iter()
            .map(|b| {
                b.insts
                    .iter()
                    .filter(|inst| inst.on_error == Some(block))
                    .count()
                    + terminator_targets(&b.term)
                        .iter()
                        .filter(|&&t| t == block)
                        .count()
            })
            .sum()
    }
}
