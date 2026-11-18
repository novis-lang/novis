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
//! # Two more decisions
//!
//! - **A reference nobody needs is released at the last use that needs
//!   one.** Where every path from a candidate call releases the value twice
//!   or more, and reads it no other way, the first release on each path moves
//!   to just before the call. The other holder's reference still covers the
//!   call, and the move check then sees one release.
//! - **A by-value `foreach` whose body writes only its own entry holds no
//!   reference of its own.** The lowering records each such loop in
//!   [`Function::own_entry_loops`], and `drop_loop_reference` makes the walk
//!   read the variable's current array and drops the loop's retain.
//! - **A join and a literal's first spread take what they open with.** A
//!   [`InstKind::Concat`] of strings always consumes its first piece, so a
//!   local it opens with moves in as into a Novis callee: `$s = $s . $x` and
//!   `$s = "{$s}x"` write into `$s`'s buffer. A literal's first
//!   [`InstKind::ArraySpread`] into its empty array is switched to `owned`
//!   where its local subject moves, as an owning `Core` call is switched to
//!   its owning entry, and it leaves the subject with the caller on an error.
//!   A slot is not lent to a spread: an item after it can fail, and its
//!   landing would free the array the slot still names.
//!
//! # The lend
//!
//! `slot = M(slot, …)` with an owning `Core` member `M`, where the slot is a
//! plain property, a static property or a property reached through plain
//! properties, gives `M` the slot's own reference. The call switches to the
//! owning entry, and the store's second read of the old value and its release
//! go. `Lend::of` reads each instruction's [`InstKind::effects`] between the
//! slot's read and its store, never the source: nothing there may run user
//! code, write a slot on the chain or read the lent slot again. A callback the
//! member runs is allowed when it is an anonymous function whose effect
//! summary ([`crate::effects`]) writes none of those slots, because a member runs
//! its callbacks before it changes anything. A hook, a guard and an observer
//! are calls, so a slot that has one is never lent.
//!
//! `slot .= e` and `slot = slot . e` are the same lend. Their
//! [`InstKind::StrAppend`] and [`InstKind::Concat`] always consume the
//! target ([`InstKind::consumes`]), so the lend switches no symbol and
//! removes the retain the lowering put before it.
//!
//! An element write, an element `.=` and an element `unset` whose root is
//! such a slot are the same lend, one level and nested. The instruction that
//! consumes the root is the write, the removal, the nested helper call, or
//! the last store of a nested `unset`'s climb. A nested `unset` reads rows
//! from the root before that store. Such a read borrows the root, and is
//! allowed where its row is read only before the store.
//!
//! An element of an array such a slot holds, `slot[k] = M(slot[k], …)` with
//! the same `string` or `int` key, is lent the same way. The array must be
//! the entry's only holder, so the pass makes it sole right after the
//! slot's read ([`InstKind::ArraySole`]) and stores it back, and the write
//! puts the result over the entry without releasing the old value, which
//! the call took ([`InstKind::ArraySet`]'s `lent`). **What it spends:** a
//! shared array is copied at the read, where the write would have copied it.
//!
//! `slot = f(slot, …)` with a known Novis callee `f` is the same lend, made
//! after every function's own moves ([`lend_to_callees`]). The callee must
//! write no slot on the chain by its effect summary. The call is pointed at
//! the callee's lent body for that parameter (`lent::body`), which leaves the
//! slot's value with the caller on every error exit and copies only at a
//! change after which a failure or a read of the slot is reachable.
//!
//! A local whose move into a known Novis callee was refused only because the
//! call's error edge reads it, as inside a `try` whose `catch` reads it, is
//! lent the same way ([`lend_local`]). The call takes the local's reference
//! with no retain, the releases on its normal paths go, and the error edge
//! keeps the local's value and its releases.
//!
//! A lent property is poisoned in a debug build ([`InstKind::LendPoison`]):
//! from just after its read to just before the call, where nothing in that
//! window can fail, and from the call's normal exit to the store. A compiled
//! read that finds the poison stops the request and names its statement, so
//! a wrong effect decision fails the debug suite at once. The call itself is
//! not covered, because a callback and a lent body may read the old value.
//! `NOVIS_LEND_IGNORE_EFFECTS` makes a debug build lend with no regard to
//! effects, which is how the trap's own test reaches it.
//!
//! **What it spends:** while compiling, one walk per candidate call over the
//! code after it, and the effect summaries [`crate::effects`] solves. At run
//! time the program makes one retain and one release fewer for each move and
//! each lend. Each copy the pass keeps of an `array`, a `string` or `bytes`
//! whose call or change gives the same type is counted
//! ([`InstKind::CopyCount`]): one runtime call each time that site runs,
//! which tests one context field unless the request is recorded.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::copies::{KeptCopy, Reason};
use crate::effects::{OwnedSlot, Summaries, Summary, callback, own, takes_an_object, types};
use crate::ids::ValueId;
use crate::ir::{Function, Inst, InstKind, Slot, Terminator, UserCode};
use crate::ty::Ty;

mod lent;

/// Decides every move and every lend in `functions`, appends the lent body
/// of each function and parameter a call lends a slot to, and returns every
/// copy it keeps ([`crate::copies`]). Each copy it can count gets an
/// [`InstKind::CopyCount`] with its row's id (`count_copies`).
pub fn run(functions: &mut Vec<Function>) -> Vec<KeptCopy> {
    let summaries = Summaries::of(functions);
    let ignore = ignore_effects();
    let mut refused: Vec<(usize, ValueId, KeptCopy, Option<Count>)> = Vec::new();
    for (at, f) in functions.iter_mut().enumerate() {
        refused.extend(
            run_function(f, &summaries, ignore)
                .into_iter()
                .map(|(value, copy, count)| (at, value, copy, count)),
        );
    }
    let (lent, lent_rows) = lend_to_callees(functions, &summaries, ignore);
    let rows: Vec<Row> = refused
        .into_iter()
        .filter(|(at, value, ..)| !lent.contains(&(*at, *value)))
        .map(|(at, _, copy, count)| (copy, count.map(|count| (at, count))))
        .chain(lent_rows)
        .collect();
    count_copies(functions, &rows);
    rows.into_iter().map(|(copy, _)| copy).collect()
}

/// One row of [`run`]'s list, with the function's index and the [`Count`]
/// where the copy can be counted.
type Row = (KeptCopy, Option<(usize, Count)>);

/// One row a lent body keeps, with the [`Count`] where it can be counted.
type BodyRow = (KeptCopy, Option<Count>);

/// Where one kept copy is counted at run time: the value the copy is made
/// from, the value the call or change gives, and the type both have.
#[derive(Clone, Copy)]
struct Count {
    before: ValueId,
    after: ValueId,
    ty: Ty,
}

impl Count {
    /// The count for a copy of `before` that `after` gives, or `None` when
    /// there is no `after` or the two are not the same `array`, `string` or
    /// `bytes` type, so their lengths cannot be compared.
    fn of(tys: &FxHashMap<ValueId, Ty>, before: ValueId, after: Option<ValueId>) -> Option<Self> {
        let after = after?;
        let ty = *tys.get(&before)?;
        let counted = matches!(ty, Ty::Array | Ty::Str | Ty::Bytes);
        (counted && tys.get(&after) == Some(&ty)).then_some(Self { before, after, ty })
    }
}

/// Puts an [`InstKind::CopyCount`] right after the instruction whose result
/// is each counted row's `after`, in that row's function, with the row's id:
/// its index in `rows` plus one. A result is defined once, and the normal
/// path of an instruction with an error edge goes on in the same block, so
/// the count runs exactly when the call or change returned. `before` is still
/// alive there: a refused value is read after the call, a slot's read is
/// released after its store, and a lent body retains the parameter before
/// its separated change.
///
/// **What it spends:** one call per counted copy site that runs, in every
/// build. With counting off the call tests one context field and returns.
fn count_copies(functions: &mut [Function], rows: &[Row]) {
    let mut by_function: FxHashMap<usize, FxHashMap<ValueId, Vec<Inst>>> = FxHashMap::default();
    for (index, (_, count)) in rows.iter().enumerate() {
        let (Some((at, count)), Ok(site)) = (count, u32::try_from(index + 1)) else {
            continue;
        };
        by_function
            .entry(*at)
            .or_default()
            .entry(count.after)
            .or_default()
            .push(Inst {
                result: None,
                ty: None,
                kind: InstKind::CopyCount {
                    site,
                    ty: count.ty,
                    before: count.before,
                    after: count.after,
                },
                on_error: None,
                raise_site: None,
            });
    }
    for (at, mut counts) in by_function {
        let Some(f) = functions.get_mut(at) else {
            continue;
        };
        for block in &mut f.blocks {
            if counts.is_empty() {
                break;
            }
            let insts = std::mem::take(&mut block.insts);
            for inst in insts {
                let placed = inst.result.and_then(|result| counts.remove(&result));
                block.insts.push(inst);
                block.insts.extend(placed.into_iter().flatten());
            }
        }
    }
}

/// The span of the statement the instruction at `(b, c)` belongs to: the
/// nearest [`InstKind::StmtMarker`] before it in its block.
fn stmt_at(f: &Function, (b, c): At) -> Option<(usize, nvs_diagnostics::Span)> {
    let insts = &f.blocks[b].insts;
    (0..=c.min(insts.len().checked_sub(1)?))
        .rev()
        .find_map(|i| match insts[i].kind {
            InstKind::StmtMarker(id) => Some((i, *f.stmt_spans.get(id.index() as usize)?)),
            _ => None,
        })
}

/// One call that may lend a slot to its known Novis callee: the caller's
/// index, the call, the callee's index and the lent parameter's index.
struct Site {
    caller: usize,
    call: At,
    callee: usize,
    param: usize,
    /// The argument it lends.
    value: ValueId,
    lent: Lent,
}

/// Lends a slot to each call of a known Novis callee where
/// [`Lend::of_call`] allows it and the callee has a lent body for that
/// parameter ([`lent::body`]). The call is pointed at the lent body, and the
/// body is built once per function and parameter, after every function's
/// own moves, so it copies the callee as it runs.
///
/// Returns each caller's index and argument it lent, and the copies the lent
/// bodies keep, each with its body's index once the bodies are appended.
fn lend_to_callees(
    functions: &mut Vec<Function>,
    summaries: &Summaries,
    ignore: bool,
) -> (FxHashSet<(usize, ValueId)>, Vec<Row>) {
    let by_name: FxHashMap<&str, usize> = functions
        .iter()
        .enumerate()
        .map(|(at, f)| (f.name.as_str(), at))
        .collect();
    let mut sites = Vec::new();
    for (caller, f) in functions.iter().enumerate() {
        let tys = types(f);
        let index: FxHashMap<u32, usize> = f
            .blocks
            .iter()
            .enumerate()
            .map(|(at, block)| (block.id.index(), at))
            .collect();
        let entry = index[&f.entry.index()];
        let defs: FxHashMap<ValueId, At> = f
            .blocks
            .iter()
            .enumerate()
            .flat_map(|(b, block)| {
                block
                    .insts
                    .iter()
                    .enumerate()
                    .filter_map(move |(i, inst)| Some((inst.result?, (b, i))))
            })
            .collect();
        let removed = FxHashSet::default();
        let lend = Lend {
            f,
            defs: &defs,
            tys: &tys,
            summaries,
            removed: &removed,
            ignore,
        };
        for (b, block) in f.blocks.iter().enumerate() {
            for (c, inst) in block.insts.iter().enumerate() {
                let InstKind::Call {
                    target,
                    receiver,
                    args,
                } = &inst.kind
                else {
                    continue;
                };
                let (Some(&callee), Some(summary)) =
                    (by_name.get(target.as_str()), summaries.get(target))
                else {
                    continue;
                };
                let Some(offset) = functions[callee].params.len().checked_sub(args.len()) else {
                    continue;
                };
                for (k, &arg) in args.iter().enumerate() {
                    if *receiver == Some(arg) {
                        continue;
                    }
                    let lent = lend
                        .of_call((b, c), arg, summary)
                        .or_else(|| lend_local(f, &index, entry, (b, c), arg));
                    if let Some(lent) = lent {
                        sites.push(Site {
                            caller,
                            call: (b, c),
                            callee,
                            param: offset + k,
                            value: arg,
                            lent,
                        });
                        break;
                    }
                }
            }
        }
    }
    // The slots each body is lent, so it copies before a read of any of them.
    let mut slots: FxHashMap<(usize, usize), FxHashSet<OwnedSlot>> = FxHashMap::default();
    for site in &sites {
        slots
            .entry((site.callee, site.param))
            .or_default()
            .extend(site.lent.chain.first().cloned());
    }
    let mut bodies: FxHashMap<(usize, usize), Option<String>> = FxHashMap::default();
    let mut built = Vec::new();
    let mut copies = Vec::new();
    let mut keys: Vec<_> = slots.keys().copied().collect();
    keys.sort_unstable();
    for key in keys {
        let body = lent::body(&functions[key.0], key.1, &slots[&key], summaries);
        bodies.insert(key, body.as_ref().map(|(f, _)| f.name.clone()));
        if let Some((body, kept)) = body {
            // Its index once `functions.extend(built)` below has run.
            let at = functions.len() + built.len();
            built.push(body);
            copies.extend(
                kept.into_iter()
                    .map(|(copy, count)| (copy, count.map(|count| (at, count)))),
            );
        }
    }
    let mut lent_values = FxHashSet::default();
    let mut per_caller: FxHashMap<usize, Vec<(At, Lent, String)>> = FxHashMap::default();
    for site in sites {
        if let Some(Some(name)) = bodies.get(&(site.callee, site.param)) {
            lent_values.insert((site.caller, site.value));
            per_caller
                .entry(site.caller)
                .or_default()
                .push((site.call, site.lent, name.clone()));
        }
    }
    for (caller, sites) in per_caller {
        let f = &mut functions[caller];
        let mut removed = FxHashSet::default();
        let mut before = FxHashMap::default();
        let mut after = FxHashMap::default();
        for ((b, c), lent, name) in sites {
            removed.extend(lent.gone);
            if let Some(poison) = lent.poison {
                place_poison(f, (b, c), &poison, &mut before, &mut after);
            }
            if let InstKind::Call { target, .. } = &mut f.blocks[b].insts[c].kind {
                *target = name;
            }
        }
        rebuild(f, &removed, before, after, &FxHashMap::default());
    }
    functions.extend(built);
    (lent_values, copies)
}

/// A local lent to the Novis call at `call`, where the move into it was
/// refused only because the call's error edge reads the local, as a `catch`
/// does. The call then takes the local's reference with no retain, and its
/// lent body leaves the reference with the caller when it fails. So the
/// releases on the paths from the call's normal exit go, with the retain,
/// and the error edge keeps its releases, as for an owning `Core` member.
/// The lent body is lent no slot, so it copies only at a change after which
/// a failure is reachable.
fn lend_local(
    f: &Function,
    index: &FxHashMap<u32, usize>,
    entry: usize,
    (b, c): At,
    value: ValueId,
) -> Option<Lent> {
    f.blocks[b].insts[c].on_error?;
    let removed = FxHashSet::default();
    let retain = retain_before(&f.blocks[b].insts, c, value, b, &removed)?;
    let walk = Walk {
        f,
        index,
        removed: &removed,
        value,
        call: (b, c),
    };
    let mut gone: Vec<At> = walk
        .releases_after(false, 1)?
        .into_iter()
        .map(|(at, _)| at)
        .collect();
    let avoid = walk.reachable_avoiding(entry, true);
    if gone
        .iter()
        .any(|&(rb, ri)| !(rb == b && ri > c) && avoid.contains(&rb))
    {
        return None;
    }
    gone.push((b, retain));
    Some(Lent {
        symbol: None,
        gone,
        element: None,
        poison: None,
        chain: Vec::new(),
    })
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
    /// A literal's first spread: take the subject's reference
    /// ([`InstKind::ArraySpread`]'s `owned`). It leaves the subject with the
    /// caller on an error, as an owning `Core` entry does.
    Spread,
}

/// Decides every move in one function. Removed instructions are collected and
/// dropped at the end, so every [`At`] stays valid while the pass runs.
///
/// Returns each `array`, `string` or `bytes` value whose move it refused,
/// with the copy that keeps ([`refusal`]). A later lend to a Novis callee
/// may still take it.
fn run_function(
    f: &mut Function,
    summaries: &Summaries,
    ignore: bool,
) -> Vec<(ValueId, KeptCopy, Option<Count>)> {
    let mut kept = Vec::new();
    let mut lent_calls: FxHashSet<At> = FxHashSet::default();
    let index: FxHashMap<u32, usize> = f
        .blocks
        .iter()
        .enumerate()
        .map(|(at, block)| (block.id.index(), at))
        .collect();
    let tys = types(f);
    let entry = index[&f.entry.index()];
    let mut removed: FxHashSet<At> = FxHashSet::default();
    for lp in f.own_entry_loops.clone() {
        drop_loop_reference(f, &index, entry, lp, &mut removed);
    }
    let defs: FxHashMap<ValueId, At> = f
        .blocks
        .iter()
        .enumerate()
        .flat_map(|(b, block)| {
            block
                .insts
                .iter()
                .enumerate()
                .filter_map(move |(i, inst)| Some((inst.result?, (b, i))))
        })
        .collect();
    let mut ids =
        crate::ids::IdGen::values_from(defs.keys().map(|v| v.index() + 1).max().unwrap_or(0));
    let mut after: FxHashMap<At, Vec<Inst>> = FxHashMap::default();
    let mut before: FxHashMap<At, Vec<Inst>> = FxHashMap::default();
    for b in 0..f.blocks.len() {
        for c in 0..f.blocks[b].insts.len() {
            let lend = Lend {
                f,
                defs: &defs,
                tys: &tys,
                summaries,
                removed: &removed,
                ignore,
            };
            if let Some(lent) = lend.of((b, c)) {
                lent_calls.insert((b, c));
                removed.extend(lent.gone);
                if let Some(poison) = lent.poison {
                    place_poison(f, (b, c), &poison, &mut before, &mut after);
                }
                if let (Some(owned), InstKind::CoreCall { symbol, .. }) =
                    (lent.symbol, &mut f.blocks[b].insts[c].kind)
                {
                    *symbol = owned;
                }
                if let Some(element) = lent.element {
                    make_sole(f, &element, &mut ids, &mut after);
                }
            }
        }
    }
    let mut released_before: FxHashMap<At, Vec<ValueId>> = FxHashMap::default();
    for b in 0..f.blocks.len() {
        for c in 0..f.blocks[b].insts.len() {
            for (value, how) in candidates(f, (b, c), &tys, &removed) {
                let error_edge_owned = matches!(how, Move::Novis { .. });
                while let Some(firsts) = (Walk {
                    f,
                    index: &index,
                    removed: &removed,
                    value,
                    call: (b, c),
                })
                .excess_releases(entry)
                {
                    removed.extend(firsts);
                    released_before.entry((b, c)).or_default().push(value);
                }
                let walk = Walk {
                    f,
                    index: &index,
                    removed: &removed,
                    value,
                    call: (b, c),
                };
                let releases = walk
                    .releases_after(error_edge_owned, 1)
                    .map(|found| found.into_iter().map(|(at, _)| at).collect::<Vec<_>>())
                    .filter(|releases| {
                        let avoid = walk.reachable_avoiding(entry, !error_edge_owned);
                        !releases
                            .iter()
                            .any(|&(rb, ri)| !(rb == b && ri > c) && avoid.contains(&rb))
                    });
                let Some(releases) = releases else {
                    if !lent_calls.contains(&(b, c))
                        && let Some((copy, count)) = refusal(f, (b, c), value, &tys, &defs)
                    {
                        kept.push((value, copy, count));
                    }
                    continue;
                };
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
                    Move::Spread => {
                        if let InstKind::ArraySpread { owned, .. } = &mut f.blocks[b].insts[c].kind
                        {
                            *owned = true;
                        }
                    }
                }
                // One move per call: a second argument with the same value
                // was rejected by `candidates`, and a different value is a
                // different reference the next call may move.
            }
        }
    }
    rebuild(f, &removed, before, after, &released_before);
    kept
}

/// The copy the call at `call` keeps of `value`, whose move it refused: a
/// value read from a property is [`Reason::SlotKept`], or
/// [`Reason::CalleeUnknown`] for a call that may run another body, and any
/// other value is [`Reason::StillUsed`]. `None` for a value that is not an
/// `array`, a `string` or `bytes`, which a change never copies, or a call
/// with no statement. With it comes the [`Count`] of `value` against the
/// call's result, where the two have the same type.
fn refusal(
    f: &Function,
    call: At,
    value: ValueId,
    tys: &FxHashMap<ValueId, Ty>,
    defs: &FxHashMap<ValueId, At>,
) -> Option<(KeptCopy, Option<Count>)> {
    if !matches!(tys.get(&value), Some(Ty::Array | Ty::Str | Ty::Bytes)) {
        return None;
    }
    let from_slot = defs
        .get(&value)
        .is_some_and(|&(db, di)| slot_of(&f.blocks[db].insts[di].kind).is_some());
    let reason = match (&f.blocks[call.0].insts[call.1].kind, from_slot) {
        (InstKind::CallVirtual { .. }, true) => Reason::CalleeUnknown,
        (_, true) => Reason::SlotKept,
        (_, false) => Reason::StillUsed,
    };
    let copy = KeptCopy {
        span: stmt_at(f, call)?.1,
        reason,
        fix: None,
    };
    let count = Count::of(tys, value, f.blocks[call.0].insts[call.1].result);
    Some((copy, count))
}

/// Puts a lent property's poison around the call at `(b, c)`
/// ([`InstKind::LendPoison`]).
fn place_poison(
    f: &Function,
    (b, c): At,
    poison: &Poison,
    before: &mut FxHashMap<At, Vec<Inst>>,
    after: &mut FxHashMap<At, Vec<Inst>>,
) {
    let kind = &f.blocks[b].insts[poison.read].kind;
    if poison.before {
        after
            .entry((b, poison.read))
            .or_default()
            .extend(poison_of(kind, poison.ty, true));
        before
            .entry((b, c))
            .or_default()
            .extend(poison_of(kind, poison.ty, false));
    }
    after
        .entry((b, c))
        .or_default()
        .extend(poison_of(kind, poison.ty, true));
}

/// Writes the pass's decisions into `f`: the instructions `before` and
/// `after` each place, a release of each value in `released_before`, and
/// every instruction in `removed` dropped.
fn rebuild(
    f: &mut Function,
    removed: &FxHashSet<At>,
    mut before: FxHashMap<At, Vec<Inst>>,
    mut after: FxHashMap<At, Vec<Inst>>,
    released_before: &FxHashMap<At, Vec<ValueId>>,
) {
    for (b, block) in f.blocks.iter_mut().enumerate() {
        let insts = std::mem::take(&mut block.insts);
        for (i, inst) in insts.into_iter().enumerate() {
            block
                .insts
                .extend(before.remove(&(b, i)).into_iter().flatten());
            for &operand in released_before.get(&(b, i)).into_iter().flatten() {
                block.insts.push(Inst {
                    result: None,
                    ty: None,
                    kind: InstKind::Release { operand },
                    on_error: None,
                    raise_site: None,
                });
            }
            if !removed.contains(&(b, i)) {
                block.insts.push(inst);
            }
            block
                .insts
                .extend(after.remove(&(b, i)).into_iter().flatten());
        }
    }
}

/// Rewrites an element's lend: after the container's read, the array is
/// made sole and stored back into its slot, the element is read from the
/// sole array, and the write into the second read of the slot is lent.
fn make_sole(
    f: &mut Function,
    element: &Element,
    ids: &mut crate::ids::IdGen,
    after: &mut FxHashMap<At, Vec<Inst>>,
) {
    let (cb, ci) = element.container;
    let read = &f.blocks[cb].insts[ci];
    let (Some(container), Some(store)) = (
        read.result,
        match &read.kind {
            InstKind::FieldGet {
                object,
                class,
                field,
            } => Some((Some(*object), class.clone(), field.clone())),
            InstKind::StaticGet { class, name } => Some((None, class.clone(), name.clone())),
            _ => None,
        },
    ) else {
        unreachable!("`Lend::of_element` lends only an array read from a slot");
    };
    let sole = ids.next_value();
    let (object, class, name) = store;
    let store = match object {
        Some(object) => InstKind::FieldSet {
            object,
            class,
            field: name,
            value: sole,
        },
        None => InstKind::StaticSet {
            class,
            name,
            value: sole,
        },
    };
    after.entry(element.container).or_default().extend([
        Inst {
            result: Some(sole),
            ty: Some(Ty::Array),
            kind: InstKind::ArraySole { array: container },
            on_error: None,
            raise_site: None,
        },
        Inst {
            result: None,
            ty: None,
            kind: store,
            on_error: None,
            raise_site: None,
        },
    ]);
    let (gb, gi) = element.get;
    if let InstKind::ArrayGet { array, .. } = &mut f.blocks[gb].insts[gi].kind {
        *array = sole;
    }
    let (sb, si) = element.set;
    if let InstKind::ArraySet { lent, .. } = &mut f.blocks[sb].insts[si].kind {
        *lent = true;
    }
}

/// Lets a by-value `foreach` whose body writes only the entry it is on walk
/// its variable's current array, and drops the reference the loop took.
///
/// The variable's value on every back edge must come from its value at the
/// header through writes at the loop's own key and joins of such values. Then
/// nothing is added or removed, and every entry keeps its position, because a
/// copy keeps its holes (`nvs_runtime::array`). The walk reads each entry
/// before the body runs, so it reads what the original array had. The
/// variable holds the array the whole time, so the loop needs no reference of
/// its own, and the body's write is in place when nothing else shares it.
///
/// The reference goes only when the loop's retain is the last thing before
/// the header that can fail, and every path from the header releases the
/// walked array before any other read of it.
fn drop_loop_reference(
    f: &mut Function,
    index: &FxHashMap<u32, usize>,
    entry: usize,
    lp: crate::ir::OwnEntryLoop,
    removed: &mut FxHashSet<At>,
) {
    let header = index[&lp.header.index()];
    let Some((phi_at, incoming)) =
        f.blocks[header]
            .insts
            .iter()
            .enumerate()
            .find_map(|(i, inst)| match &inst.kind {
                InstKind::Phi { incoming } if inst.result == Some(lp.current) => {
                    Some((i, incoming.clone()))
                }
                _ => None,
            })
    else {
        return;
    };
    let Some(&(pre, _)) = incoming.iter().find(|&&(_, v)| v == lp.walked) else {
        return;
    };
    let mut chain: FxHashSet<ValueId> = FxHashSet::default();
    chain.insert(lp.current);
    loop {
        let mut grew = false;
        for inst in f.blocks.iter().flat_map(|block| &block.insts) {
            let Some(result) = inst.result else {
                continue;
            };
            let joins = match &inst.kind {
                InstKind::ArraySet { array, key, .. } => chain.contains(array) && *key == lp.key,
                InstKind::Phi { incoming } => {
                    result != lp.current && incoming.iter().all(|(_, v)| chain.contains(v))
                }
                _ => false,
            };
            if joins && chain.insert(result) {
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    if !incoming
        .iter()
        .all(|&(from, v)| from == pre || chain.contains(&v))
    {
        return;
    }
    // The walk reads the variable's current array from here on.
    let mut slot = None;
    for inst in &mut f.blocks[header].insts {
        if let InstKind::ArrayNextSlot { array, .. } = &mut inst.kind
            && *array == lp.walked
        {
            *array = lp.current;
            slot = inst.result;
        }
    }
    let Some(slot) = slot else {
        return;
    };
    for inst in f.blocks.iter_mut().flat_map(|block| &mut block.insts) {
        if let InstKind::ArrayKeyAt { array, slot: at } | InstKind::ArrayValueAt { array, slot: at } =
            &mut inst.kind
            && *array == lp.walked
            && *at == slot
        {
            *array = lp.current;
        }
    }
    // The loop's own retain, and the first release of the walked array on
    // every path from the header.
    let pre = index[&pre.index()];
    let Some(retain) = f.blocks[pre].insts.iter().rposition(
        |inst| matches!(inst.kind, InstKind::Retain { operand } if operand == lp.walked),
    ) else {
        return;
    };
    if removed.contains(&(pre, retain))
        || f.blocks[pre].insts[retain + 1..]
            .iter()
            .any(|inst| inst.on_error.is_some() || reads(&inst.kind, lp.walked))
    {
        return;
    }
    let walk = Walk {
        f,
        index,
        removed,
        value: lp.walked,
        call: (header, phi_at),
    };
    let Some(firsts) = walk.first_releases() else {
        return;
    };
    let avoid = walk.reachable_avoiding(entry, false);
    if firsts
        .iter()
        .any(|&(rb, ri)| !(rb == header && ri > phi_at) && avoid.contains(&rb))
    {
        return;
    }
    removed.extend(firsts);
    removed.insert((pre, retain));
}

/// What a lend changes: the owning symbol a `Core` call switches to, the
/// instructions it removes, and for an element, the instructions it rewrites.
/// An instruction that always consumes its operand switches to nothing.
struct Lent {
    symbol: Option<&'static str>,
    gone: Vec<At>,
    element: Option<Element>,
    poison: Option<Poison>,
    /// The slots on the lent slot's chain, the lent slot first. A lent body
    /// copies at a change after which one of them may be read.
    chain: Vec<OwnedSlot>,
}

/// Where a lent property's poison goes ([`InstKind::LendPoison`]): after the
/// slot's read at `read` until just before the call, when `before` holds, and
/// from the call's normal exit to the store. `before` holds only where nothing
/// in that window can fail, so a `catch` never finds the slot poisoned. `ty`
/// is the slot's representation, whose tag the restore writes back.
struct Poison {
    read: usize,
    before: bool,
    ty: Ty,
}

/// Whether a debug build lends with no regard to effects, so the lend check's
/// trap can be tested on a lend the pass would refuse. Read once per program
/// from `NOVIS_LEND_IGNORE_EFFECTS`; a release build never reads it.
fn ignore_effects() -> bool {
    cfg!(debug_assertions) && std::env::var_os("NOVIS_LEND_IGNORE_EFFECTS").is_some()
}

/// The [`InstKind::LendPoison`] for the property the read `kind` reads.
fn poison_of(kind: &InstKind, ty: Ty, on: bool) -> Option<Inst> {
    let (object, class, name) = match kind {
        InstKind::FieldGet {
            object,
            class,
            field,
        } => (Some(*object), class.clone(), field.clone()),
        InstKind::StaticGet { class, name } => (None, class.clone(), name.clone()),
        _ => return None,
    };
    Some(Inst {
        result: None,
        ty: None,
        kind: InstKind::LendPoison {
            object,
            class,
            name,
            ty,
            on,
        },
        on_error: None,
        raise_site: None,
    })
}

/// Where an element's lend rewrites the function. The container's read gets
/// an [`InstKind::ArraySole`] and a store of the sole array after it, the
/// element's read reads the sole array, and the write is lent.
struct Element {
    container: At,
    get: At,
    set: At,
}

/// One function's view for the lend: `slot = M(slot, …)`, where `M` is an
/// owning `Core` member and the slot a plain property, a static property, a
/// property reached through a chain of plain properties, or an element of an
/// array one of those holds ([`Lend::of_element`]).
///
/// The lowering emits the slot's read, the call on the borrowed entry, a
/// second read of the slot's old value, the store of the result and a
/// release of the old value. Where the check holds, the call takes the slot's
/// own reference through the owning entry, and the second read and its
/// release go. The slot keeps its value when the call fails, because an
/// owning entry leaves its subject with the caller on an error.
struct Lend<'a> {
    f: &'a Function,
    defs: &'a FxHashMap<ValueId, At>,
    tys: &'a FxHashMap<ValueId, Ty>,
    summaries: &'a Summaries,
    removed: &'a FxHashSet<At>,
    /// [`ignore_effects`], read once per program.
    ignore: bool,
}

impl Lend<'_> {
    /// The owning symbol and the instructions to remove, when the call at
    /// `(b, c)` can be lent its subject's slot. `None` keeps it as it is.
    ///
    /// Every condition must hold:
    ///
    /// - The subject is read from a slot in the call's own block, and is read
    ///   by nothing but the call, one retain and the releases that pair with
    ///   that retain.
    /// - The call's result is stored back into the same slot, through the
    ///   same object, later in the block.
    /// - From the slot's read to the store, no instruction other than the
    ///   call may run user code, write the slot or a container on its chain,
    ///   or read the slot. Between the call and the store nothing can fail, so
    ///   the slot is never left without a value it owns.
    /// - Each callable the call takes is an anonymous function whose effect
    ///   summary writes none of those slots. Its body runs
    ///   before anything changes, so a read of the slot sees the old value.
    fn of(&self, (b, c): At) -> Option<Lent> {
        let insts = &self.f.blocks[b].insts;
        let call = &insts[c];
        // The subject, the owning symbol to switch to, and the arguments whose
        // callables the call may run. An instruction that always consumes its
        // operand needs no switch and runs no callable.
        let (subject, symbol, args): (ValueId, Option<&'static str>, &[ValueId]) = match &call.kind
        {
            InstKind::CoreCall {
                symbol,
                args,
                subject: Some(subject),
            } => {
                let entry = nvs_stdlib::owning_entry(symbol)?;
                let at = *subject as usize;
                if entry.slot != at {
                    return None;
                }
                (*args.get(at)?, Some(entry.symbol), args)
            }
            kind => (kind.consumes()?, None, &[]),
        };
        self.of_subject((b, c), subject, symbol, args, None)
    }

    /// [`Self::of`] for a call to a known Novis callee, `summary` its effect
    /// summary, lending the argument `subject`. The callee must write no slot
    /// on the chain, and an element is not lent to one.
    fn of_call(&self, at: At, subject: ValueId, summary: &Summary) -> Option<Lent> {
        self.of_subject(at, subject, None, &[], Some(summary))
    }

    /// The lend of `subject` to the call at `(b, c)`, with the owning
    /// `symbol` it switches to and the arguments whose callables it may run.
    /// `callee` is a Novis callee's summary, which must write no slot on the
    /// chain.
    fn of_subject(
        &self,
        (b, c): At,
        subject: ValueId,
        symbol: Option<&'static str>,
        args: &[ValueId],
        callee: Option<&Summary>,
    ) -> Option<Lent> {
        let insts = &self.f.blocks[b].insts;
        let call = &insts[c];
        let result = call.result?;
        let mut mentions = 0;
        call.kind
            .for_each_operand(|v| mentions += usize::from(v == subject));
        if mentions != 1
            || !matches!(
                self.tys.get(&subject),
                Some(Ty::Array | Ty::Str | Ty::Bytes)
            )
        {
            return None;
        }
        let (rb, read) = *self.defs.get(&subject)?;
        if rb != b || read >= c {
            return None;
        }
        if matches!(insts[read].kind, InstKind::ArrayGet { .. }) {
            if callee.is_some() {
                return None;
            }
            let (gone, element) = self.of_element((b, c), subject, result, read, args)?;
            // An element has no tag of its own to poison yet.
            return Some(Lent {
                symbol,
                gone,
                element: Some(element),
                poison: None,
                chain: Vec::new(),
            });
        }
        let (slot, object) = slot_of(&insts[read].kind)?;
        // The containers on the chain, and where the first of them is read.
        let (chain, mut start) = self.chain(b, slot, object, read)?;
        // The store, and the second read of the old value between.
        let mut gone = Vec::new();
        let mut old = None;
        let mut store = None;
        for (j, inst) in insts.iter().enumerate().skip(c + 1) {
            if self.removed.contains(&(b, j)) {
                continue;
            }
            if let Some((stored, into, value)) = store_of(&inst.kind)
                && value == result
                && stored == slot
                && let Some(from) = self.same_object(into, object, b, &chain)
            {
                start = start.min(from);
                store = Some(j);
                break;
            }
            if inst.on_error.is_some() || reads(&inst.kind, result) {
                return None;
            }
            if let Some((read_slot, from)) = slot_of(&inst.kind)
                && read_slot == slot
                && old.is_none()
                && let Some(earliest) = self.same_object(from, object, b, &chain)
            {
                start = start.min(earliest);
                old = Some((j, inst.result?));
                continue;
            }
            if matches!(inst.kind, InstKind::Release { operand } if operand == subject) {
                continue;
            }
            if reads(&inst.kind, subject) || !self.quiet(&inst.kind, &chain, true) {
                return None;
            }
        }
        let store = store?;
        let (old_at, old) = old?;
        gone.push((b, old_at));
        gone.push(self.old_release(b, old_at, old, store)?);
        self.window_is_quiet(b, start, c, &[read], subject, &chain)?;
        gone.extend(self.subject_counts((b, c), subject)?);
        self.callbacks_are_quiet(args, subject, &chain)?;
        if let Some(callee) = callee
            && !self.ignore
            && chain.iter().any(|&s| callee.writes.reaches(s))
        {
            return None;
        }
        let before = self.ignore
            || (read + 1..c).all(|i| self.removed.contains(&(b, i)) || insts[i].on_error.is_none());
        Some(Lent {
            symbol,
            gone,
            element: None,
            chain: chain.iter().map(|&s| own(s)).collect(),
            poison: Some(Poison {
                read,
                before,
                ty: *self.tys.get(&subject)?,
            }),
        })
    }

    /// The release of `old`, the slot's old value read at `old_at`, when it is
    /// released once, after the store at `store`, and read by nothing else.
    fn old_release(&self, b: usize, old_at: usize, old: ValueId, store: usize) -> Option<At> {
        let mut found = None;
        for (ib, block) in self.f.blocks.iter().enumerate() {
            for (i, inst) in block.insts.iter().enumerate() {
                if (ib, i) == (b, old_at) || !reads(&inst.kind, old) {
                    continue;
                }
                if ib != b
                    || i <= store
                    || found.is_some()
                    || !matches!(inst.kind, InstKind::Release { operand } if operand == old)
                {
                    return None;
                }
                found = Some((ib, i));
            }
        }
        found
    }

    /// The lend of an array element, `slot[k] = M(slot[k], …)`, where the
    /// subject is the [`InstKind::ArrayGet`] at `get`. The slot holding the
    /// array is plain, and `k` is the same `string` or `int` key in the read
    /// and in the write.
    ///
    /// The array must be the entry's only holder, so the pass makes it sole
    /// right after its read ([`InstKind::ArraySole`]) and stores it back. The
    /// write then puts the result over the entry without releasing it,
    /// because the call took that reference ([`InstKind::ArraySet`]'s
    /// `lent`). Between the sole array's read and the write nothing may read
    /// or write the slot, run user code or fail after the call, exactly as
    /// for a plain slot, so nothing can share the array again.
    fn of_element(
        &self,
        (b, c): At,
        subject: ValueId,
        result: ValueId,
        get: usize,
        args: &[ValueId],
    ) -> Option<(Vec<At>, Element)> {
        let insts = &self.f.blocks[b].insts;
        let InstKind::ArrayGet {
            array: container,
            key,
            absent: crate::ir::AbsentKey::Throws,
        } = insts[get].kind
        else {
            return None;
        };
        if !matches!(self.tys.get(&key), Some(Ty::Str | Ty::Int)) {
            return None;
        }
        let (cb, ci) = *self.defs.get(&container)?;
        if cb != b || ci >= get || self.uses(container) != 1 {
            return None;
        }
        let (slot, object) = slot_of(&insts[ci].kind)?;
        let (chain, mut start) = self.chain(b, slot, object, ci)?;
        // The second read of the array, the write into it, and the store.
        // The lowering retains the second read for the write and reads the
        // slot's old value again around the store (`write_back_array`); the
        // write is lent the slot's reference instead, so those three go.
        let mut reread = None;
        let mut retain = None;
        let mut set = None;
        let mut old = None;
        let mut store = None;
        for (j, inst) in insts.iter().enumerate().skip(c + 1) {
            if self.removed.contains(&(b, j)) {
                continue;
            }
            if let Some((_, written)) = set
                && let Some((stored, into, value)) = store_of(&inst.kind)
                && value == written
                && stored == slot
                && let Some(from) = self.same_object(into, object, b, &chain)
            {
                start = start.min(from);
                store = Some(j);
                break;
            }
            if let Some((_, again)) = reread
                && set.is_none()
                && retain.is_none()
                && matches!(inst.kind, InstKind::Retain { operand } if operand == again)
            {
                retain = Some(j);
                continue;
            }
            if set.is_some()
                && old.is_none()
                && let Some((read_slot, from)) = slot_of(&inst.kind)
                && read_slot == slot
                && let Some(earliest) = self.same_object(from, object, b, &chain)
            {
                start = start.min(earliest);
                old = Some((j, inst.result?));
                continue;
            }
            if let Some((_, again)) = reread
                && set.is_none()
                && let InstKind::ArraySet {
                    array,
                    key: written_key,
                    value,
                    lent: false,
                } = inst.kind
                && array == again
                && value == result
            {
                if !self.same_key(key, written_key)
                    || self.uses(again) != 1 + usize::from(retain.is_some())
                {
                    return None;
                }
                let written = inst.result?;
                if self.uses(written) != 1 {
                    return None;
                }
                set = Some((j, written));
                continue;
            }
            if inst.on_error.is_some() || reads(&inst.kind, result) {
                return None;
            }
            if reread.is_none()
                && let Some((read_slot, from)) = slot_of(&inst.kind)
                && read_slot == slot
                && let Some(earliest) = self.same_object(from, object, b, &chain)
            {
                start = start.min(earliest);
                reread = Some((j, inst.result?));
                continue;
            }
            if matches!(inst.kind, InstKind::Release { operand } if operand == subject) {
                continue;
            }
            if reads(&inst.kind, subject) || !self.quiet(&inst.kind, &chain, true) {
                return None;
            }
        }
        let store = store?;
        let (set, _) = set?;
        let mut gone = match (retain, old) {
            (None, None) => Vec::new(),
            (Some(retain), Some((old_at, old))) => {
                vec![
                    (b, retain),
                    (b, old_at),
                    self.old_release(b, old_at, old, store)?,
                ]
            }
            _ => return None,
        };
        self.window_is_quiet(b, start, c, &[ci, get], subject, &chain)?;
        gone.extend(self.subject_counts((b, c), subject)?);
        self.callbacks_are_quiet(args, subject, &chain)?;
        Some((
            gone,
            Element {
                container: (b, ci),
                get: (b, get),
                set: (b, set),
            },
        ))
    }

    /// The slots on the chain from `slot` out, and the index of the first
    /// container's read, starting at `read`. Every container is read in
    /// block `b`.
    fn chain<'s>(
        &'s self,
        b: usize,
        slot: Slot<'s>,
        object: Option<ValueId>,
        read: usize,
    ) -> Option<(Vec<Slot<'s>>, usize)> {
        let mut chain = vec![slot];
        let mut start = read;
        let mut on = object;
        while let Some(o) = on {
            let Some(&(ob, oi)) = self.defs.get(&o) else {
                break;
            };
            let Some((container, outer)) = slot_of(&self.f.blocks[ob].insts[oi].kind) else {
                break;
            };
            if ob != b {
                return None;
            }
            chain.push(container);
            start = start.min(oi);
            on = outer;
        }
        Some((chain, start))
    }

    /// Whether nothing from `start` to the call at `c` touches the slots on
    /// `chain`, other than the reads at `skip`, and the subject is read only
    /// by its retain, its releases and element reads whose rows are read only
    /// before the call, such as a nested `unset`'s descent.
    fn window_is_quiet(
        &self,
        b: usize,
        start: usize,
        c: usize,
        skip: &[usize],
        subject: ValueId,
        chain: &[Slot<'_>],
    ) -> Option<()> {
        let insts = &self.f.blocks[b].insts;
        let read = *skip.last()?;
        for (i, inst) in insts.iter().enumerate().take(c).skip(start) {
            if skip.contains(&i) || self.removed.contains(&(b, i)) {
                continue;
            }
            if i > read && reads(&inst.kind, subject) {
                if !self.borrows_a_row(inst, subject, (b, c))
                    && !matches!(
                        inst.kind,
                        InstKind::Retain { .. } | InstKind::Release { .. }
                    )
                {
                    return None;
                }
                continue;
            }
            if !self.quiet(&inst.kind, chain, true) {
                return None;
            }
        }
        Some(())
    }

    /// Every retain and release of the subject in the function, outside the
    /// call at `call`, when they pair up, so removing all of them keeps the
    /// counts. `None` when another instruction reads the subject.
    fn subject_counts(&self, call: At, subject: ValueId) -> Option<Vec<At>> {
        let mut found = Vec::new();
        let mut retains = 0;
        for (ib, block) in self.f.blocks.iter().enumerate() {
            for (i, inst) in block.insts.iter().enumerate() {
                if (ib, i) == call || self.removed.contains(&(ib, i)) {
                    continue;
                }
                match inst.kind {
                    InstKind::Retain { operand } if operand == subject => {
                        retains += 1;
                        found.push((ib, i));
                    }
                    InstKind::Release { operand } if operand == subject => found.push((ib, i)),
                    _ if self.borrows_a_row(inst, subject, call) => {}
                    ref kind if reads(kind, subject) => return None,
                    _ => {}
                }
            }
            let mut read_by_term = false;
            block
                .term
                .for_each_operand(|v| read_by_term |= v == subject);
            if read_by_term {
                return None;
            }
        }
        let releases = found.len() - retains;
        if retains > 1 || (retains == 0 && releases > 0) {
            return None;
        }
        Some(found)
    }

    /// Whether every callable among `args` is an anonymous function whose
    /// effect summary, its callees' included, writes no slot on `chain`.
    fn callbacks_are_quiet(
        &self,
        args: &[ValueId],
        subject: ValueId,
        chain: &[Slot<'_>],
    ) -> Option<()> {
        if self.ignore {
            return Some(());
        }
        for &arg in args {
            if arg == subject || !takes_an_object(&InstKind::Retain { operand: arg }, self.tys) {
                continue;
            }
            let &(ab, ai) = self.defs.get(&arg)?;
            let body = callback(&self.f.blocks[ab].insts[ai].kind)?;
            let writes = &self.summaries.get(&body)?.writes;
            if chain.iter().any(|&s| writes.reaches(s)) {
                return None;
            }
        }
        Some(())
    }

    /// How many instructions and terminators read `value`.
    /// Whether `inst` is an element read of `subject` whose row is read only
    /// before the call at `call`, such as a nested `unset`'s descent. It
    /// borrows the subject, and the call changes the subject only after the
    /// row's last read.
    fn borrows_a_row(&self, inst: &Inst, subject: ValueId, call: At) -> bool {
        matches!(inst.kind, InstKind::ArrayGet { array, .. } if array == subject)
            && inst
                .result
                .is_some_and(|row| self.used_only_before(row, call))
    }

    /// Whether every read of `value` is in the call's block before the call
    /// at `(b, c)`. A row read from the lent subject is then never read after
    /// the call changes the subject in place.
    fn used_only_before(&self, value: ValueId, (b, c): At) -> bool {
        for (ib, block) in self.f.blocks.iter().enumerate() {
            for (i, inst) in block.insts.iter().enumerate() {
                if !self.removed.contains(&(ib, i))
                    && reads(&inst.kind, value)
                    && (ib != b || i >= c)
                {
                    return false;
                }
            }
            let mut read_by_term = false;
            block.term.for_each_operand(|v| read_by_term |= v == value);
            if read_by_term {
                return false;
            }
        }
        true
    }

    fn uses(&self, value: ValueId) -> usize {
        let mut count = 0;
        for (ib, block) in self.f.blocks.iter().enumerate() {
            for (i, inst) in block.insts.iter().enumerate() {
                if !self.removed.contains(&(ib, i)) && reads(&inst.kind, value) {
                    count += 1;
                }
            }
            block
                .term
                .for_each_operand(|v| count += usize::from(v == value));
        }
        count
    }

    /// Whether `x` and `y` are the same key: one value, or two constants of
    /// the same text or number.
    fn same_key(&self, x: ValueId, y: ValueId) -> bool {
        if x == y {
            return true;
        }
        let (Some(&(xb, xi)), Some(&(yb, yi))) = (self.defs.get(&x), self.defs.get(&y)) else {
            return false;
        };
        match (
            &self.f.blocks[xb].insts[xi].kind,
            &self.f.blocks[yb].insts[yi].kind,
        ) {
            (InstKind::ConstStr(a), InstKind::ConstStr(b)) => a == b,
            (InstKind::ConstInt(a), InstKind::ConstInt(b)) => a == b,
            _ => false,
        }
    }

    /// Whether `kind` runs no user code, writes no slot on `chain`, and, with
    /// `reading`, does not read the lent slot.
    fn quiet(&self, kind: &InstKind, chain: &[Slot<'_>], reading: bool) -> bool {
        if self.ignore {
            return true;
        }
        let effects = kind.effects();
        let user_code = match effects.user_code {
            UserCode::No => false,
            UserCode::ItsCallables => takes_an_object(kind, self.tys),
            UserCode::Yes => true,
        };
        !user_code
            && !chain.iter().any(|&s| effects.writes.reaches(s))
            && !(reading && effects.reads.reaches(chain[0]))
    }

    /// Whether `x` and `y` are the same object: the same value, or two reads
    /// of one container slot of the same object, both in block `b`. Gives
    /// the index of the earliest read it followed, and `usize::MAX` for the
    /// same value. Every container is on `chain`, so the window check from
    /// that index on finds any write between the two reads.
    fn same_object(
        &self,
        x: Option<ValueId>,
        y: Option<ValueId>,
        b: usize,
        chain: &[Slot<'_>],
    ) -> Option<usize> {
        match (x, y) {
            (None, None) => Some(usize::MAX),
            (Some(x), Some(y)) if x == y => Some(usize::MAX),
            (Some(x), Some(y)) => {
                let (&(xb, xi), &(yb, yi)) = (self.defs.get(&x)?, self.defs.get(&y)?);
                if xb != b || yb != b {
                    return None;
                }
                let insts = &self.f.blocks[b].insts;
                let (xs, xo) = slot_of(&insts[xi].kind)?;
                let (ys, yo) = slot_of(&insts[yi].kind)?;
                if xs != ys || !chain.contains(&xs) {
                    return None;
                }
                Some(self.same_object(xo, yo, b, chain)?.min(xi).min(yi))
            }
            _ => None,
        }
    }
}

/// The slot a plain read reads, and the object it reads it from.
fn slot_of(kind: &InstKind) -> Option<(Slot<'_>, Option<ValueId>)> {
    match kind {
        InstKind::FieldGet {
            object,
            class,
            field,
        } => Some((Slot::Field { class, field }, Some(*object))),
        InstKind::StaticGet { class, name } => Some((Slot::Static { class, name }, None)),
        _ => None,
    }
}

/// The slot a plain store writes, the object it writes it on, and the value.
fn store_of(kind: &InstKind) -> Option<(Slot<'_>, Option<ValueId>, ValueId)> {
    match kind {
        InstKind::FieldSet {
            object,
            class,
            field,
            value,
        } => Some((Slot::Field { class, field }, Some(*object), *value)),
        InstKind::StaticSet { class, name, value } => {
            Some((Slot::Static { class, name }, None, *value))
        }
        _ => None,
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
        // A literal that opens with a spread, `$a = [...$a, …]`, may take the
        // subject's reference. The spread writes into the literal's empty
        // array, so a sole list subject can be the literal itself.
        InstKind::ArraySpread {
            array,
            subject,
            owned: false,
        } if array != subject
            && matches!(tys.get(subject), Some(Ty::Array))
            && is_empty_literal(f, *array) =>
        {
            vec![(*subject, Move::Spread)]
        }
        // A concatenation of strings takes its first piece's reference and
        // cannot fail, so a local it opens with moves in as into a Novis
        // callee: `$s = $s . $x` and `$s = "{$s}x"` write into `$s`'s buffer.
        kind @ InstKind::Concat { .. } => {
            let Some(value) = kind.consumes() else {
                return Vec::new();
            };
            let mut mentions = 0;
            kind.for_each_operand(|v| mentions += usize::from(v == value));
            if mentions != 1 {
                return Vec::new();
            }
            retain_before(insts, c, value, b, removed)
                .map(|retain| {
                    vec![(
                        value,
                        Move::Novis {
                            retain: (b, retain),
                        },
                    )]
                })
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

/// Whether `array` is made by an [`InstKind::ArrayNew`] with no entries.
fn is_empty_literal(f: &Function, array: ValueId) -> bool {
    f.blocks.iter().flat_map(|block| &block.insts).any(|inst| {
        inst.result == Some(array)
            && matches!(&inst.kind, InstKind::ArrayNew { entries } if entries.is_empty())
    })
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
    ///
    /// `need` is how many releases each path must have. With `1`, a second
    /// release fails the walk. With more, a path may have more, and each
    /// release found is paired with how many came before it on its path,
    /// capped at `need - 1`.
    fn releases_after(&self, error_edge: bool, need: u8) -> Option<Vec<(At, u8)>> {
        let (cb, ci) = self.call;
        let mut releases = Vec::new();
        let mut seen: FxHashSet<(usize, u8)> = FxHashSet::default();
        let mut work: Vec<(usize, usize, u8)> = vec![(cb, ci + 1, 0)];
        if error_edge && let Some(landing) = self.f.blocks[cb].insts[ci].on_error {
            self.edge(cb, landing, 0, &mut seen, &mut work)?;
        }
        while let Some((b, start, mut released)) = work.pop() {
            let block = &self.f.blocks[b];
            for (i, inst) in block.insts.iter().enumerate().skip(start) {
                if self.removed.contains(&(b, i)) || matches!(inst.kind, InstKind::Phi { .. }) {
                    continue;
                }
                match inst.kind {
                    InstKind::Release { operand } if operand == self.value => {
                        if released == need {
                            if need == 1 {
                                return None;
                            }
                        } else {
                            released += 1;
                        }
                        releases.push(((b, i), released - 1));
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
            ) && released < need
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

    /// A reference the program holds past the call but never needs, as the
    /// first of the releases when every path from the call has two or more
    /// and reads the value no other way. Each path's first release is then
    /// moved to just before the call, which is right after the last use that
    /// needs the reference: the other holder's reference still covers the
    /// call. So `$b = $a; $a = M($a);` leaves `M` the only reference when `$b`
    /// is not read again. The call's error edge is one of the paths, because
    /// the release comes before the call fails. `None` when one release is
    /// first on one path and later on another, or when a path from the entry
    /// that avoids the call reaches one.
    fn excess_releases(&self, entry: usize) -> Option<Vec<At>> {
        let (cb, ci) = self.call;
        let found = self.releases_after(true, 2)?;
        let firsts: FxHashSet<At> = found
            .iter()
            .filter(|(_, before)| *before == 0)
            .map(|(at, _)| *at)
            .collect();
        if found
            .iter()
            .any(|(at, before)| *before > 0 && firsts.contains(at))
        {
            return None;
        }
        let avoid = self.reachable_avoiding(entry, false);
        if firsts
            .iter()
            .any(|&(rb, ri)| !(rb == cb && ri > ci) && avoid.contains(&rb))
        {
            return None;
        }
        let mut firsts: Vec<At> = firsts.into_iter().collect();
        firsts.sort_unstable();
        Some(firsts)
    }

    /// The first release of the value on every path from `call`, or `None`
    /// when a path reads it another way first or ends without releasing it.
    /// What a path does after its first release is not walked.
    fn first_releases(&self) -> Option<Vec<At>> {
        let (cb, ci) = self.call;
        let mut releases = Vec::new();
        let mut seen: FxHashSet<(usize, u8)> = FxHashSet::default();
        let mut work: Vec<(usize, usize, u8)> = vec![(cb, ci + 1, 0)];
        'paths: while let Some((b, start, _)) = work.pop() {
            let block = &self.f.blocks[b];
            for (i, inst) in block.insts.iter().enumerate().skip(start) {
                if self.removed.contains(&(b, i)) || matches!(inst.kind, InstKind::Phi { .. }) {
                    continue;
                }
                match inst.kind {
                    InstKind::Release { operand } if operand == self.value => {
                        releases.push((b, i));
                        continue 'paths;
                    }
                    ref kind if reads(kind, self.value) => return None,
                    _ => {}
                }
                if let Some(landing) = inst.on_error {
                    self.edge(b, landing, 0, &mut seen, &mut work)?;
                }
            }
            let mut read = false;
            block.term.for_each_operand(|v| read |= v == self.value);
            if read
                || matches!(
                    block.term,
                    Terminator::Return(_) | Terminator::Propagate { .. }
                )
            {
                return None;
            }
            for target in terminator_targets(&block.term) {
                self.edge(b, target, 0, &mut seen, &mut work)?;
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
        released: u8,
        seen: &mut FxHashSet<(usize, u8)>,
        work: &mut Vec<(usize, usize, u8)>,
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
