//! Each function's effect summary, solved over the whole program's call graph
//! (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! A [`Summary`] says which properties and static properties a call to a
//! function may read and write, by declaring class and name. It covers
//! everything the function calls, so the ownership pass can ask one question
//! of a call it lends a slot to.
//!
//! A summary does not say whether a call may fail. Every Novis function may
//! throw a catchable `RecursionError` at its entry (`rule:errors/stack-depth`),
//! so every call to one may fail, and the instruction's own error edge already
//! says so.
//!
//! # The resolver
//!
//! [`callee`] names the one body a call reaches while compiling, or `None`.
//! An [`InstKind::Call`] binds to a label only when nothing in the program
//! overrides its declaration (`nvs_types::expr_table::ResolvedCall::overridden`),
//! so its target is the one body. `new` with a constructor reaches that
//! constructor. A virtual call, `new static`, a callable and a name-keyed call
//! may reach more than one body, and have no callee. A callback a `Core`
//! member runs is resolved where the argument is an anonymous function made in
//! the same function: its body is `Class::invoke` ([`callback`]).
//!
//! # The fixed point
//!
//! Every summary starts as its function's own instructions. A call with a
//! callee adds that callee's summary, and any other call that may run user
//! code makes the summary [`Touches::Anything`]. The pass repeats until no
//! summary grows, so a recursive group reaches one summary: each grows only
//! to the union of the group's own instructions.
//!
//! **What it spends:** while compiling, one walk per function per round, and
//! the rounds are bounded by the depth of the call graph plus its cycles.
//! Nothing at run time.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::ValueId;
use crate::ir::{Function, InstKind, Slot, Touch, UserCode};
use crate::ty::Ty;

/// A slot by value, so a summary outlives the function it was read from:
/// whether it is static, the declaring class, and the property's name.
pub(crate) type OwnedSlot = (bool, String, String);

/// `slot` by value.
pub(crate) fn own(slot: Slot<'_>) -> OwnedSlot {
    match slot {
        Slot::Field { class, field } => (false, class.to_owned(), field.to_owned()),
        Slot::Static { class, name } => (true, class.to_owned(), name.to_owned()),
    }
}

/// The storage a summary may read, or may write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Touches {
    /// Any slot: it may run code no summary covers, or touch a slot by name.
    Anything,
    /// These slots and no others.
    Slots(FxHashSet<OwnedSlot>),
}

impl Default for Touches {
    fn default() -> Self {
        Touches::Slots(FxHashSet::default())
    }
}

impl Touches {
    /// Whether this may reach `slot`.
    pub(crate) fn reaches(&self, slot: Slot<'_>) -> bool {
        match self {
            Touches::Anything => true,
            Touches::Slots(slots) => slots.contains(&own(slot)),
        }
    }

    fn add(&mut self, touch: Touch<'_>) {
        match touch {
            Touch::Nothing => {}
            Touch::Slot(slot) => {
                if let Touches::Slots(slots) = self {
                    slots.insert(own(slot));
                }
            }
            Touch::Anything => *self = Touches::Anything,
        }
    }

    fn join(&mut self, other: &Touches) {
        match (&mut *self, other) {
            (Touches::Anything, _) => {}
            (_, Touches::Anything) => *self = Touches::Anything,
            (Touches::Slots(own), Touches::Slots(more)) => own.extend(more.iter().cloned()),
        }
    }
}

/// What a call to one function may do: everything it and its callees may
/// read and write.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Summary {
    /// The storage it may read.
    pub(crate) reads: Touches,
    /// The storage it may write.
    pub(crate) writes: Touches,
}

impl Summary {
    const ANYTHING: Self = Self {
        reads: Touches::Anything,
        writes: Touches::Anything,
    };

    fn join(&mut self, other: &Summary) {
        self.reads.join(&other.reads);
        self.writes.join(&other.writes);
    }
}

/// Every function's [`Summary`], by name.
pub(crate) struct Summaries {
    by_name: FxHashMap<String, Summary>,
}

impl Summaries {
    /// Solves every function's summary to a fixed point over the call graph.
    pub(crate) fn of(functions: &[Function]) -> Self {
        let bodies: Vec<Body<'_>> = functions.iter().map(Body::new).collect();
        let mut by_name: FxHashMap<String, Summary> = FxHashMap::default();
        for body in &bodies {
            by_name.insert(body.f.name.clone(), Summary::default());
        }
        let mut summaries = Self { by_name };
        loop {
            let mut grew = false;
            for body in &bodies {
                let next = body.summary(&summaries);
                let slot = summaries
                    .by_name
                    .get_mut(&body.f.name)
                    .expect("every function has a summary");
                if *slot != next {
                    *slot = next;
                    grew = true;
                }
            }
            if !grew {
                return summaries;
            }
        }
    }

    /// The summary of the function named `name`, or `None` when the program
    /// has no such function.
    pub(crate) fn get(&self, name: &str) -> Option<&Summary> {
        self.by_name.get(name)
    }

    /// What `kind` may do, with its callee's summary in place of "any user
    /// code". `callables` names the body of a callback argument, as
    /// [`callback`] does. A call no summary covers is [`Summary::ANYTHING`].
    pub(crate) fn of_inst(
        &self,
        kind: &InstKind,
        tys: &FxHashMap<ValueId, Ty>,
        callables: impl Fn(ValueId) -> Option<String>,
    ) -> Summary {
        let effects = kind.effects();
        let mut summary = Summary::default();
        summary.reads.add(effects.reads);
        summary.writes.add(effects.writes);
        match effects.user_code {
            UserCode::No => {}
            UserCode::Yes => match callee(kind).and_then(|name| self.get(name)) {
                Some(callee) => {
                    summary.reads = Touches::default();
                    summary.writes = Touches::default();
                    summary.join(callee);
                }
                None => return Summary::ANYTHING,
            },
            UserCode::ItsCallables => {
                let mut bodies = Vec::new();
                let mut unknown = false;
                kind.for_each_operand(|v| {
                    if takes_an_object(&InstKind::Retain { operand: v }, tys) {
                        match callables(v) {
                            Some(name) => bodies.push(name),
                            None => unknown = true,
                        }
                    }
                });
                if unknown {
                    return Summary::ANYTHING;
                }
                for name in bodies {
                    match self.get(&name) {
                        Some(callee) => summary.join(callee),
                        None => return Summary::ANYTHING,
                    }
                }
            }
        }
        summary
    }
}

/// The one body `kind` reaches while compiling: a bound call's target or the
/// constructor `new` runs. `None` for every other instruction, and for a call
/// that may reach more than one body.
pub(crate) fn callee(kind: &InstKind) -> Option<&str> {
    match kind {
        InstKind::Call { target, .. } => Some(target),
        InstKind::New {
            ctor: Some(ctor), ..
        } => Some(ctor),
        _ => None,
    }
}

/// The body a callback reaches, when `value` is an anonymous function made
/// by an [`InstKind::New`] with no constructor: `Class::invoke`.
pub(crate) fn callback(kind: &InstKind) -> Option<String> {
    match kind {
        InstKind::New {
            class, ctor: None, ..
        } => Some(format!("{class}::invoke")),
        _ => None,
    }
}

/// Every value's type in `f`.
pub(crate) fn types(f: &Function) -> FxHashMap<ValueId, Ty> {
    f.blocks
        .iter()
        .flat_map(|block| &block.insts)
        .filter_map(|inst| Some((inst.result?, inst.ty?)))
        .collect()
}

/// Whether any argument of `kind` may be a callable. A callable is an
/// object, and a tagged value may hold one.
pub(crate) fn takes_an_object(kind: &InstKind, tys: &FxHashMap<ValueId, Ty>) -> bool {
    let mut found = false;
    kind.for_each_operand(|v| {
        found |= !matches!(
            tys.get(&v),
            Some(
                Ty::Bool
                    | Ty::Int
                    | Ty::Uint
                    | Ty::Float
                    | Ty::Decimal
                    | Ty::Null
                    | Ty::Str
                    | Ty::Bytes
                    | Ty::Array
                    | Ty::Enum(_)
            )
        );
    });
    found
}

/// One function with the tables its summary reads.
struct Body<'a> {
    f: &'a Function,
    tys: FxHashMap<ValueId, Ty>,
    /// Each value's defining instruction.
    defs: FxHashMap<ValueId, &'a InstKind>,
}

impl<'a> Body<'a> {
    fn new(f: &'a Function) -> Self {
        Self {
            f,
            tys: types(f),
            defs: f
                .blocks
                .iter()
                .flat_map(|block| &block.insts)
                .filter_map(|inst| Some((inst.result?, &inst.kind)))
                .collect(),
        }
    }

    /// This function's summary, with its callees' summaries as they stand.
    fn summary(&self, summaries: &Summaries) -> Summary {
        let mut summary = Summary::default();
        for inst in self.f.blocks.iter().flat_map(|block| &block.insts) {
            let one = summaries.of_inst(&inst.kind, &self.tys, |v| {
                self.defs.get(&v).and_then(|kind| callback(kind))
            });
            summary.join(&one);
            if summary.reads == Touches::Anything && summary.writes == Touches::Anything {
                break;
            }
        }
        summary
    }
}
