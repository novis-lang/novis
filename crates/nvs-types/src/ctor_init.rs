//! `rule:classes/definite-property-initialization` — definite property initialization: every constructor a
//! class declares must assign, on every path out of it, every property the
//! class declares itself ([`crate::signatures::own_required_properties`]),
//! and — when the class `extends` another *that declares a constructor*
//! ([`parent_declares_constructor`]) — call `parent::constructor(...)` on
//! every path too,
//! discharging the inherited properties without re-deriving what the
//! parent's own constructor already assigns (the parent was checked against
//! this same rule when it was compiled, exactly as an ordinary call's callee
//! is trusted rather than re-verified at every call site — `rule:classes/definite-property-initialization`,
//! second bullet). A class with no constructor of its own and an
//! unassigned required property is refused at the property's own
//! declaration instead, since there is no constructor body to attach the
//! diagnostic to (`rule:classes/definite-property-initialization`, third bullet).
//!
//! This is a second, narrower flow-analysis pass over a constructor's body,
//! separate from [`crate::locals`]'s: it never checks an expression's
//! *type* (`crate::check::check_method` already walks every method body,
//! constructors included, for that), only whether `$this->prop = ...`
//! (plain `=`, never a compound operator — mirroring
//! `crate::expr::assign::check_assign`'s own "only `=` counts" rule for locals)
//! and `parent::constructor(...)` are reached on every path. [`walk_stmt`]
//! threads one [`InitState`] through the same control-flow shape
//! [`crate::locals::check_block`] does — `if`/`else`, `switch` and `try`/
//! `catch` all join by intersecting every branch that can finish normally
//! (a `do`-`while` body, which always runs, needs no join at all) — see that
//! module's own docs for the reasoning, which carries over unchanged with
//! [`InitState`] standing in for `live`.
//! Rather than computing a whole-body "definitely assigned" set once and
//! diagnosing at the end the way [`crate::locals`] does for a local read,
//! [`finish`] runs at every point a path can leave the constructor (an
//! explicit `return`, and — if some path never returns — the implicit one
//! at the end of the body), since `rule:classes/definite-property-initialization` is stated per-return, not
//! per-body.
//!
//! **A promoted constructor parameter carries no obligation here**, although it
//! is a property everywhere else: [`crate::signatures`] records its type and
//! its visibility and `crate::layout` gives it a slot. Its store is emitted
//! from the binding by `nvs_ir::lower::promoted_stores` rather than written in
//! the body, so `rule:classes/definite-property-initialization` is discharged
//! by construction and a constructor body has nothing to be checked against.
//!
//! **Every composite expression is descended into**, through
//! [`nvs_syntax::visit::each_child_expr`] — the one match over the productions
//! [`nvs_syntax::ast::ExprKind`] holds, written in the crate that owns the
//! enum so a form added later is a build error there rather than one this pass
//! quietly walks past. The forms whose operands do not all run are
//! [`scan_branches`]'s, and it joins them exactly as [`walk_stmt`] joins an
//! `if`/`else`: the right operand of a short-circuiting `&&`, `||` or `??`
//! ([`nvs_syntax::ast::BinaryOp::short_circuits`]) proves nothing, and a
//! ternary, a `match` and an expression-level `catch` intersect their
//! alternatives — so `$flag ? ($this->count = 1) : 0` leaves `$count`
//! unassigned, which is what `rule:classes/definite-property-initialization`'s
//! "on every path" says it is.
//!
//! **An anonymous function's body is not descended into**, and neither is an anonymous
//! class's: a body written inside an expression runs when it is *called*,
//! which is not where it is written and need not be ever, so
//! `$this->prop = ...` inside one is not an assignment the constructor made.
//! A constructor that assigns a property only from an anonymous function it stores is
//! therefore refused, and `rule:classes/lateinit` is the spelling for a
//! property that really is written after construction.
//!
//! **A hooked property whose backing slot is read carries the obligation**,
//! and [`crate::signatures::ClassSignature::required_properties`] says which
//! those are. A write in the constructor runs the `set` hook, so it assigns
//! the property only when the hook stores it. [`check_set_hooks_commit`]
//! walks each such hook with this same walker, the hook's own property as its
//! one obligation, and refuses a hook a path can leave without storing. Every
//! constructor write then counts as an assignment, and the constructor is
//! checked as it would be with no hook.
//!
//! **A read before the assignment is refused too.** [`InitState::read`] sees
//! every `$this->prop` the walk evaluates other than the target of a plain
//! `=`, and a required property no path has assigned yet is `E0409` at the
//! read, where the zeroed slot would otherwise be read. An inherited required
//! property is read too early before `parent::constructor(...)` is called. A
//! read through a method call is not followed, as no call is.
//!
//! **Construction is two-phase, as Swift has it**, which is what lets the
//! untested slot reads of compiled code hold without following calls. `$this`
//! escapes the constructor wherever it is used other than as `$this->prop`:
//! a method call on it, passing or storing it, a `throw` that carries it, an
//! anonymous function that captures it, and an instance method called through
//! `parent::`, `self::` or `static::`. [`InitState::escape`] refuses an escape
//! while a path has not yet assigned an own required property, or has not yet
//! called an owed `parent::constructor(...)`, and that call itself is refused
//! while an own required property is unassigned: the parent's constructor may
//! call a method this class overrides. The diagnostic names the first
//! property in declaration order still unassigned. A `set` hook of a property
//! the constructor writes is not followed, as no call is.
//!
//! **A `readonly` property is written at most once on each path**, which is
//! the "assigned exactly once" `rule:classes/lateinit-restrictions` states for
//! the modifier. [`InitState`] carries a second set beside the assigned one,
//! the class's own `readonly` properties that *may* have been written, joined
//! by union. A write through `$this` to a name already in it is `E0782`, and
//! so is one inside a loop, and one to a promoted parameter, which the
//! binding wrote before the body ran. Every write spelling counts here — `=`,
//! a compound operator, `++`/`--`, `unset`, and each of them on an element of
//! the property (`$this->list[0] = v`) — and a short-circuit operand,
//! a `catch` and a `finally` start from every write that may have run before
//! them. A write through another receiver, and one from outside the
//! constructor, are refused by `crate::expr::assign` instead.
//!
//! **`parent::constructor(...)` runs at most once on each path**, tracked in
//! the same may-have-run set under [`PARENT_CALL`]: a second call, or one
//! inside a loop, is `E0885`, since it would write the parent's `readonly`
//! properties again. Every other call of `constructor` by name is refused
//! where it is checked, in `crate::expr::calls`.
//!
//! **Inheriting a constructor is not declaring one.** A class that writes no
//! `constructor` of its own runs its parent's unchanged, and that body cannot
//! assign a property the subclass declared after it — so every own required
//! property without a default is refused at its own declaration, exactly as
//! for a class with no parent at all. The `parent::constructor(...)`
//! obligation is not owed either, and there is nowhere for such a call to go:
//! the inherited constructor runs by inheritance rather than from a body this
//! class wrote. That is the whole of what this pass models about an inherited
//! constructor, and it is what `rule:classes/definite-property-initialization`
//! states.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    AssignOp, Block, ClassDecl, ClassMemberKind, Expr, ExprKind, MemberName, Stmt, StmtKind,
};
use nvs_syntax::visit::each_child_expr;

use crate::expr::is_this_receiver;
use crate::expr_table::{CtorAccess, CtorAccesses};
use crate::live::{Live, Mark, assigned_on_every_way};
use crate::signatures::{own_required_properties, resolve_method};
use crate::{Env, span_text, strip_sigil};

/// The `$this->prop = ...`/`parent::constructor(...)` obligations one class
/// declaration's constructor is checked against, bundled so [`walk_stmt`]
/// and [`scan_expr`] thread one argument instead of several.
struct CtorObligations<'a> {
    qname: &'a QName,
    required: &'a [(String, Span)],
    ctor_name: Span,
    needs_parent_call: bool,
    /// Which body is walked, for the wording of what [`finish`] reports.
    body: Body,
    /// Where a constructor's missing assignment is written, and the
    /// parameters it may be written from. `None` for a `set` hook.
    fix: Option<AssignFix<'a>>,
}

/// The quick fix on a property a constructor does not assign
/// (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`): `$this->name =
/// $name;` as the body's first statement, offered only where the constructor
/// has a parameter of that name that is not promoted. With no such parameter
/// nothing says what the value is, so nothing is offered.
struct AssignFix<'a> {
    /// An empty span where the statement is inserted.
    at: Span,
    /// What goes before and after the statement, so the statement after it
    /// keeps its line.
    around: (&'static str, String),
    /// The names of the parameters that are not promoted, without `$`.
    params: &'a [String],
}

impl<'a> AssignFix<'a> {
    /// Before the first statement of `body`, indented as it is, or inside an
    /// empty body.
    fn new(body: &Block, params: &'a [String], src: &str) -> Self {
        let (at, around) = match body.stmts.first() {
            Some(first) => {
                let start = first.span.start as usize;
                let line = src[..start].rfind('\n').map_or(0, |i| i + 1);
                let indent = &src[line..start];
                let after = if indent.trim().is_empty() {
                    format!("\n{indent}")
                } else {
                    " ".to_owned()
                };
                (first.span.shrink_to_start(), ("", after))
            }
            None => (
                Span::at(body.span.file, body.span.start + 1),
                (" ", " ".to_owned()),
            ),
        };
        Self { at, around, params }
    }

    /// `diag` with `$this->name = $name;` for a property `name`, or `diag`
    /// alone where no parameter has that name.
    fn suggest(&self, diag: Diagnostic, name: &str) -> Diagnostic {
        if !self.params.iter().any(|param| param == name) {
            return diag;
        }
        let (before, after) = &self.around;
        // Not safe: the parameter's type may not be the property's, and a
        // `readonly` property some path assigns would be written twice.
        diag.with_unsafe_fix(
            self.at,
            format!("{before}$this->{name} = ${name};{after}"),
            format!("assign `$this->{name}` from `${name}`"),
        )
    }
}

/// The two bodies [`walk_stmts`] checks the obligations of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    /// A constructor, which assigns every required property.
    Constructor,
    /// A `set` hook, which stores its own property on every path that does
    /// not throw — see [`check_set_hooks_commit`].
    SetHook,
}

/// Per-path state while walking a constructor body: which required
/// properties have definitely been assigned so far, and whether
/// `parent::constructor(...)` has definitely been called so far.
///
/// A branch is walked on the same state as the code around it, never on a
/// copy: [`InitState::mark`] records where it starts and
/// [`InitState::rewind`] takes back what it added, the way
/// [`crate::live::Live`] does for [`crate::locals`]. So a branch costs what
/// it assigns, and a constructor checks in time linear in its length however
/// many properties are assigned before each branch. The undo log spends one
/// more copy of each assigned property name while the constructor is checked.
///
/// `written` is the other direction: the class's own `readonly` properties
/// that *may* have been written so far, on some path that reaches this point.
/// Its join is a union where `assigned`'s is an intersection, and a write to a
/// name already in it is a second write (`rule:classes/lateinit-restrictions` states the contract).
#[derive(Default)]
pub(crate) struct InitState {
    assigned: Live,
    written: Live,
    parent_called: bool,
    /// The class's own `readonly` properties — the names `written` tracks.
    readonly: Vec<String>,
    /// The `readonly` properties a promoted parameter already wrote before
    /// the body runs.
    promoted: Vec<String>,
    /// How many loops the walk is inside: a write there can run again.
    loop_depth: u32,
    /// Every second write found, reported once the walk is over.
    refused: Vec<SecondWrite>,
    /// The class's own required properties, whose reads `read` checks
    /// against `assigned`. Empty for a `set` hook's walk.
    own_required: Vec<String>,
    /// The required properties of the class's ancestors, which a read before
    /// `parent::constructor(...)` finds unassigned.
    inherited_required: Vec<String>,
    /// Every read of a property no path has assigned yet, reported once the
    /// walk is over.
    early_reads: Vec<EarlyRead>,
    /// The constructor's class, whose escapes [`InitState::escape`] checks.
    /// `None` for a `set` hook's walk, which checks none.
    class: Option<QName>,
    /// Whether the constructor owes a `parent::constructor(...)` call.
    needs_parent_call: bool,
    /// Every use of `$this` that a path reaches too early, reported once the
    /// walk is over.
    escapes: Vec<Escape>,
    /// Every `parent::constructor(...)` that a path may reach a second time,
    /// reported once the walk is over.
    second_parent_calls: Vec<Span>,
    /// The class's own slots that start unset and that the body writes,
    /// which [`InitState::access`] checks against `assigned`. Empty for a
    /// `set` hook's walk.
    starts_unset: Vec<String>,
    /// What each `$this->prop` access finds unset, by property.
    accesses: CtorAccesses,
}

/// The name `written` tracks a `parent::constructor(...)` call under. No
/// property name has `::` in it, so it joins with the `readonly` writes and
/// is refused the way a second write is.
const PARENT_CALL: &str = "parent::constructor";

/// A use of `$this` that a path reaches before construction's first phase is
/// over.
struct Escape {
    span: Span,
    /// The escape is the `parent::constructor(...)` call itself.
    parent_call: bool,
    /// The first own required property not yet assigned, or `None` when every
    /// one is and `parent::constructor(...)` has not run yet.
    missing: Option<String>,
}

/// A read through `$this` of a property no path has assigned yet.
struct EarlyRead {
    name: String,
    span: Span,
    /// The property is an ancestor's, which `parent::constructor(...)` assigns.
    inherited: bool,
}

/// A write to a `readonly` property that may not be the first one on its path.
struct SecondWrite {
    span: Span,
    name: String,
    why: &'static str,
}

/// Where a branch starts, from [`InitState::mark`].
#[derive(Clone, Copy)]
struct InitMark {
    assigned: Mark,
    written: Mark,
    parent_called: bool,
}

/// What one way out of a branch added past its mark: the properties it
/// assigned, the `readonly` properties it may have written, and whether
/// `parent::constructor(...)` had been called by its end.
#[derive(Default)]
struct Added {
    assigned: Vec<String>,
    written: Vec<String>,
    parent_called: bool,
}

impl InitState {
    fn mark(&self) -> InitMark {
        InitMark {
            assigned: self.assigned.mark(),
            written: self.written.mark(),
            parent_called: self.parent_called,
        }
    }

    /// Takes back everything added since `mark` and returns it.
    fn rewind(&mut self, mark: InitMark) -> Added {
        let added = Added {
            assigned: self.assigned.rewind(mark.assigned),
            written: self.written.rewind(mark.written),
            parent_called: self.parent_called,
        };
        self.parent_called = mark.parent_called;
        added
    }

    fn extend(&mut self, added: Added) {
        self.assigned.extend(added.assigned);
        self.written.extend(added.written);
        self.parent_called |= added.parent_called;
    }

    /// Keeps, of what was added since `mark`, only what `other` also added:
    /// the join of two ways out of a branch that both start at `mark`. What
    /// either way may have written is kept from both.
    fn intersect_since(&mut self, mark: InitMark, other: &Added) {
        self.assigned
            .intersect_since(mark.assigned, &other.assigned);
        self.written.extend(other.written.clone());
        self.parent_called &= other.parent_called;
    }

    /// Adds `names` to what may have been written, and nothing else: the
    /// start of a path that runs after another path stopped partway, such as
    /// a `catch` after its `try` threw.
    fn note_written(&mut self, names: &[String]) {
        self.written.extend(names.to_vec());
    }

    /// Records a write through `$this` to the property `name`, and refuses it
    /// where it can be the second write to a `readonly` one.
    fn write(&mut self, name: &str, span: Span) {
        self.access(name);
        if !self.readonly.iter().any(|r| r == name) {
            return;
        }
        let why = if self.loop_depth > 0 {
            Some("this write is inside a loop, so it can run more than once")
        } else if self.promoted.iter().any(|p| p == name) {
            Some("the promoted constructor parameter already sets this property")
        } else if self.written.contains(name) {
            Some("this path through the constructor already wrote this property")
        } else {
            None
        };
        if let Some(why) = why
            && !self.refused.iter().any(|r| r.span == span)
        {
            self.refused.push(SecondWrite {
                span,
                name: name.to_owned(),
                why,
            });
        }
        self.written.insert(name.to_owned());
    }

    /// Records a read through `$this` of the property `name`, and refuses it
    /// where a path reaches it before the property is assigned.
    fn read(&mut self, name: &str, span: Span) {
        self.access(name);
        let inherited = !self.own_required.iter().any(|r| r == name);
        let early = !self.assigned.contains(name)
            && if inherited {
                !self.parent_called && self.inherited_required.iter().any(|r| r == name)
            } else {
                true
            };
        if early && !self.early_reads.iter().any(|read| read.span == span) {
            self.early_reads.push(EarlyRead {
                name: name.to_owned(),
                span,
                inherited,
            });
        }
    }

    /// Records, for a constructor's walk, which of the class's own slots a
    /// path may not have written yet where the body reads or writes `name`
    /// through `$this`, and whether `parent::constructor(...)` may not have
    /// run there. A write is recorded before it assigns, since a `set` hook
    /// runs before its own slot has a value.
    fn access(&mut self, name: &str) {
        if self.class.is_none() {
            return;
        }
        let entry = self.accesses.entry(name.to_owned()).or_default();
        for slot in &self.starts_unset {
            if !self.assigned.contains(slot) && !entry.unassigned.contains(slot) {
                entry.unassigned.push(slot.clone());
            }
        }
        entry.before_parent |= !self.parent_called;
    }

    /// Records a use of `$this` at `span` that lets the object escape, and
    /// refuses it where a path reaches it before every own required property
    /// is assigned. Unless `parent_call` says the use is the
    /// `parent::constructor(...)` call, an owed call must have run too.
    fn escape(&mut self, span: Span, parent_call: bool) {
        if self.class.is_none() || self.escapes.iter().any(|e| e.span == span) {
            return;
        }
        let missing = self
            .own_required
            .iter()
            .find(|name| !self.assigned.contains(name))
            .cloned();
        let parent_owed = !parent_call && self.needs_parent_call && !self.parent_called;
        if missing.is_some() || parent_owed {
            self.escapes.push(Escape {
                span,
                parent_call,
                missing,
            });
        }
    }
}

/// What every way in `ways` added, or `None` when there are no ways at all.
/// A property counts as assigned when every way assigned it, and as written
/// when any way may have written it.
fn added_on_every_way(ways: Vec<Added>) -> Option<Added> {
    let parent_called = ways.iter().all(|way| way.parent_called);
    let written = ways
        .iter()
        .flat_map(|way| way.written.iter().cloned())
        .collect();
    let assigned = assigned_on_every_way(ways.into_iter().map(|way| way.assigned).collect())?;
    Some(Added {
        assigned,
        written,
        parent_called,
    })
}

/// Checks one class declaration against `rule:classes/definite-property-initialization`. Interfaces and enums
/// are never called here — only [`crate::check::check_stmts`]'s `ClassDecl`
/// arm calls this, since only a class is ever instantiated through a
/// constructor.
pub(crate) fn check_class_init(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let mut required = own_required_properties(qname, env.signatures, env.graph);
    check_set_hooks_commit(decl, qname, &required, env);
    // A property declared again over an ancestor's that has no default is
    // the ancestor's to assign, in the constructor this class inherits or
    // calls with `parent::constructor(...)`. A `covariant` one is tested when
    // that constructor writes it.
    let inherited = ancestors_assigned(qname, env);
    required.retain(|(name, _)| !inherited.contains(name));

    let ctor = decl.members.iter().find_map(|m| match &m.kind {
        ClassMemberKind::Method(method) if span_text(env.src, method.name) == "constructor" => {
            Some(method)
        }
        _ => None,
    });

    // A class with no constructor body runs none of its own accesses.
    if ctor.and_then(|ctor| ctor.body.as_ref()).is_none() {
        env.exprs
            .record_ctor_accesses(qname.clone(), CtorAccesses::default());
    }

    let Some(ctor) = ctor else {
        // No constructor at all: a required property has no path to get
        // assigned along, so it is refused right at its own declaration
        // (`rule:classes/definite-property-initialization`, third bullet).
        for (name, span) in &required {
            // A hooked property has no syntax for a default value.
            let hooked = env
                .signatures
                .get(qname)
                .is_some_and(|sig| sig.hooked_properties.contains_key(name));
            let help = if hooked {
                "add a `constructor` that assigns it"
            } else {
                "give it a default value, or add a `constructor` that assigns it. If the value \
                 needs `$this`, declare the property `lateinit` and assign it later"
            };
            env.diags.report(
                Diagnostic::error(
                    code::E_UNINITIALIZED_PROPERTY,
                    format!(
                        "`${name}` has no default value, and `{qname}` declares no constructor \
                         to assign it"
                    ),
                )
                .with_primary(*span, "never assigned")
                .with_help(help),
            );
        }
        return;
    };

    let Some(body) = &ctor.body else {
        return; // an abstract constructor has no body to walk
    };

    let needs_parent_call = decl.extends.is_some() && parent_declares_constructor(qname, env);
    let plain_params: Vec<String> = ctor
        .params
        .iter()
        .filter(|p| !p.is_promoted())
        .map(|p| strip_sigil(span_text(env.src, p.name)).to_owned())
        .collect();
    let obligations = CtorObligations {
        qname,
        required: &required,
        ctor_name: ctor.name,
        needs_parent_call,
        body: Body::Constructor,
        fix: Some(AssignFix::new(body, &plain_params, env.src.text())),
    };
    let readonly: Vec<String> = env
        .signatures
        .get(qname)
        .map(|sig| sig.readonly_properties.iter().cloned().collect())
        .unwrap_or_default();
    let all_promoted: Vec<String> = ctor
        .params
        .iter()
        .filter(|p| p.is_promoted())
        .map(|p| strip_sigil(span_text(env.src, p.name)).to_owned())
        .collect();
    let promoted = all_promoted
        .iter()
        .filter(|name| readonly.contains(name))
        .cloned()
        .collect();
    let own_unset = env
        .signatures
        .get(qname)
        .map(|sig| crate::check::starts_unset(sig, env.interner))
        .unwrap_or_default();
    // Each promotion is a write before the body runs, and before any
    // promotion after it, so it finds every slot unset.
    let mut accesses = CtorAccesses::default();
    for name in &all_promoted {
        let entry: &mut CtorAccess = accesses.entry(name.clone()).or_default();
        entry.unassigned.clone_from(&own_unset);
        entry.before_parent = true;
    }
    let mut state = InitState {
        readonly,
        promoted,
        own_required: required.iter().map(|(name, _)| name.clone()).collect(),
        inherited_required: ancestors_required(qname, env),
        class: Some(qname.clone()),
        needs_parent_call,
        starts_unset: own_unset
            .into_iter()
            .filter(|name| !all_promoted.contains(name))
            .collect(),
        accesses,
        ..InitState::default()
    };
    let terminates = walk_stmts(&body.stmts, &mut state, &obligations, env);
    env.exprs
        .record_ctor_accesses(qname.clone(), std::mem::take(&mut state.accesses));
    if !terminates {
        // Fell off the end of the body without an explicit `return` — the
        // implicit return `rule:classes/definite-property-initialization` also covers.
        finish(&state, &obligations, env);
    }
    for read in &state.early_reads {
        let name = &read.name;
        let help = if read.inherited {
            "call `parent::constructor(...)` before this line on every path".to_owned()
        } else {
            format!("assign `$this->{name}` before this line on every path")
        };
        env.diags.report(
            Diagnostic::error(
                code::E_UNINITIALIZED_PROPERTY,
                format!("`${name}` is read before the constructor assigns it"),
            )
            .with_primary(
                read.span,
                "a path reaches this read before the property has a value",
            )
            .with_help(help),
        );
    }
    for escape in &state.escapes {
        let (message, label, help) = match (&escape.missing, escape.parent_call) {
            (Some(name), true) => (
                format!(
                    "`parent::constructor(...)` is called before the constructor assigns `${name}`"
                ),
                format!("a path reaches this call before `${name}` has a value"),
                format!(
                    "assign `$this->{name}` before this call on every path. The parent \
                     constructor can call a method that this class overrides"
                ),
            ),
            (Some(name), false) => (
                format!("`$this` is used before the constructor assigns `${name}`"),
                format!("a path reaches this use of `$this` before `${name}` has a value"),
                format!(
                    "assign `$this->{name}` before this line on every path. Code that gets \
                     `$this` can read every property. If the value needs `$this`, declare \
                     `${name}` as `lateinit` and assign it after the constructor"
                ),
            ),
            (None, _) => (
                "`$this` is used before `parent::constructor(...)` is called".to_owned(),
                "a path reaches this use of `$this` before the inherited properties have values"
                    .to_owned(),
                "call `parent::constructor(...)` before this line on every path".to_owned(),
            ),
        };
        env.diags.report(
            Diagnostic::error(code::E_UNINITIALIZED_PROPERTY, message)
                .with_primary(escape.span, label)
                .with_help(help),
        );
    }
    for &span in &state.second_parent_calls {
        env.diags.report(
            Diagnostic::error(
                code::E_CONSTRUCTOR_CALLED_DIRECTLY,
                "`parent::constructor(...)` runs once on each path through the constructor",
            )
            .with_primary(span, "a path can reach this call a second time")
            .with_help(
                "call `parent::constructor(...)` once, outside any loop, on each path. Compute \
                 its arguments in local variables first",
            ),
        );
    }
    for refused in &state.refused {
        let name = &refused.name;
        env.diags.report(
            Diagnostic::error(
                code::E_READONLY_WRITE_AFTER_CONSTRUCTION,
                format!("`{qname}::${name}` is `readonly`, so the constructor writes it only once"),
            )
            .with_primary(refused.span, refused.why)
            .with_help(
                "write the property once on each path through the constructor. Compute the value \
                 in a local variable first, and assign the local variable at the end",
            ),
        );
    }
}

/// Whether any ancestor of `qname` declares a `constructor` at all.
///
/// The `parent::constructor(...)` obligation is owed only when there is one
/// to call: an `abstract class Shape` that declares no constructor gives its
/// subclass nothing to reach, and demanding the call anyway made such a class
/// impossible to compile — `parent::constructor()` is itself an `E0309`
/// against a parent with no such method. Nothing is lost by the narrowing:
/// a parent with no constructor has no inherited property to discharge,
/// because `rule:classes/definite-property-initialization`'s third bullet already refuses one at its own
/// declaration.
fn parent_declares_constructor(qname: &QName, env: &Env<'_>) -> bool {
    let Some(links) = env.graph.get(qname) else {
        return false;
    };
    links
        .extends
        .iter()
        .any(|parent| resolve_method(parent, "constructor", env.signatures, env.graph).is_some())
}

/// The required properties of every class `qname` extends, nearest first.
/// Every instance property an ancestor of `qname` declares with no default
/// and not `lateinit`, a promoted constructor parameter included. The
/// ancestor's own constructor assigns each one, which its own check of this
/// rule holds it to. A property with a default is left out on purpose: a
/// `covariant` redeclaration of `?int $n = null` as `int $n` has no default
/// of its own, so the subclass must still assign it, and the parent's `null`
/// never reaches the narrower slot.
fn ancestors_assigned(qname: &QName, env: &Env<'_>) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = rustc_hash::FxHashSet::default();
    let mut next = env
        .graph
        .get(qname)
        .and_then(|links| links.extends.first().cloned());
    while let Some(ancestor) = next {
        if !seen.insert(ancestor.clone()) {
            break;
        }
        if let Some(sig) = env.signatures.get(&ancestor) {
            names.extend(
                sig.properties
                    .keys()
                    .filter(|name| {
                        !sig.lateinit_properties.contains(*name)
                            && !sig.property_defaults.iter().any(|(n, _)| n == *name)
                    })
                    .cloned(),
            );
        }
        next = env
            .graph
            .get(&ancestor)
            .and_then(|links| links.extends.first().cloned());
    }
    names
}

fn ancestors_required(qname: &QName, env: &Env<'_>) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen: Vec<&QName> = Vec::new();
    let mut queue: Vec<&QName> = env
        .graph
        .get(qname)
        .map(|links| links.extends.iter().collect())
        .unwrap_or_default();
    while let Some(ancestor) = queue.pop() {
        if seen.contains(&ancestor) {
            continue;
        }
        seen.push(ancestor);
        names.extend(
            own_required_properties(ancestor, env.signatures, env.graph)
                .into_iter()
                .map(|(name, _)| name),
        );
        if let Some(links) = env.graph.get(ancestor) {
            queue.extend(links.extends.iter());
        }
    }
    names
}

/// Checks that each `set` hook of a property in `required` stores the
/// property on every path that does not throw.
///
/// A write to a hooked property runs its `set` hook, so the constructor's
/// `$this->p = v` assigns `$p` only when the hook stores a value. A hook that
/// can return without storing is refused here, at the hook, and every write
/// in a constructor then counts as an assignment. The short `set => expr;`
/// form always stores, so only a block body is walked, with [`walk_stmts`]
/// and the hook's own property as the one obligation.
fn check_set_hooks_commit(
    decl: &ClassDecl,
    qname: &QName,
    required: &[(String, Span)],
    env: &mut Env<'_>,
) {
    use nvs_syntax::ast::{PropertyHookBody, PropertyHookKind};
    for member in &decl.members {
        let ClassMemberKind::Property(p) = &member.kind else {
            continue;
        };
        let name = strip_sigil(span_text(env.src, p.name));
        let Some(own) = required.iter().find(|(r, _)| r == name) else {
            continue;
        };
        for hook in p.hooks.iter().flatten() {
            let Some(PropertyHookBody::Block(block)) = &hook.body else {
                continue;
            };
            if hook.kind != PropertyHookKind::Set {
                continue;
            }
            let one = std::slice::from_ref(own);
            let obligations = CtorObligations {
                qname,
                required: one,
                ctor_name: crate::returns::closing_brace(block),
                needs_parent_call: false,
                body: Body::SetHook,
                fix: None,
            };
            let mut state = InitState::default();
            if !walk_stmts(&block.stmts, &mut state, &obligations, env) {
                finish(&state, &obligations, env);
            }
        }
    }
}

/// Reports whatever `state` still leaves unsatisfied at one point a
/// constructor, or a `set` hook, can return from.
fn finish(state: &InitState, obligations: &CtorObligations<'_>, env: &mut Env<'_>) {
    for (name, span) in obligations.required {
        if state.assigned.contains(name) {
            continue;
        }
        let diag = match obligations.body {
            Body::Constructor => Diagnostic::error(
                code::E_UNINITIALIZED_PROPERTY,
                format!("`${name}` is not assigned on every path out of the constructor"),
            )
            .with_primary(*span, "declared here")
            .with_secondary(
                obligations.ctor_name,
                "not assigned along every path out of this constructor",
            ),
            Body::SetHook => Diagnostic::error(
                code::E_UNINITIALIZED_PROPERTY,
                format!("the `set` hook of `${name}` does not store a value on every path"),
            )
            .with_primary(
                obligations.ctor_name,
                format!("a path can reach the end of the hook without assigning `$this->{name}`"),
            )
            .with_secondary(*span, "this property has no default value")
            .with_help(format!(
                "assign `$this->{name}` on every path that does not throw"
            )),
        };
        let diag = match &obligations.fix {
            Some(fix) => fix.suggest(diag, name),
            None => diag,
        };
        env.diags.report(diag);
    }
    if obligations.needs_parent_call && !state.parent_called {
        env.diags.report(
            Diagnostic::error(
                code::E_MISSING_PARENT_CONSTRUCTOR_CALL,
                format!(
                    "`{}` extends another class, but a path through its constructor never \
                     calls `parent::constructor(...)`",
                    obligations.qname
                ),
            )
            .with_primary(obligations.ctor_name, "missing call on some path"),
        );
    }
}

/// Walks `stmts` in sequence, updating `state` in place. Returns `true` when
/// every path through `stmts` definitely ends in a `return` or `throw` —
/// [`check_class_init`] uses that to know whether the body's fall-through
/// end still needs its own [`finish`] call.
fn walk_stmts(
    stmts: &[Stmt],
    state: &mut InitState,
    obligations: &CtorObligations<'_>,
    env: &mut Env<'_>,
) -> bool {
    for stmt in stmts {
        if walk_stmt(stmt, state, obligations, env) {
            return true;
        }
    }
    false
}

fn walk_stmt(
    stmt: &Stmt,
    state: &mut InitState,
    obligations: &CtorObligations<'_>,
    env: &mut Env<'_>,
) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) => {
            finish(state, obligations, env);
            true
        }
        StmtKind::Expr(e) => {
            if matches!(&e.kind, ExprKind::Throw(_)) {
                // A throwing path never returns the constructed object, so
                // `rule:classes/definite-property-initialization` has nothing to check on it.
                // What it throws may still carry `$this` out.
                scan_expr(e, state, env);
                return true;
            }
            scan_expr(e, state, env);
            false
        }
        StmtKind::Block(b) => walk_stmts(&b.stmts, state, obligations, env),
        // Each arm's `else` is the rest of the chain: the arms are walked in
        // order, each taken back off `state` once walked and kept as what it
        // added, then joined from the last arm back to the first. A state
        // that ends in a terminating path is never read, so `state` may
        // carry it.
        StmtKind::If { arms, else_ } => {
            let mut joins = Vec::with_capacity(arms.len());
            for (i, arm) in arms.iter().enumerate() {
                scan_expr(&arm.cond, state, env);
                let mark = state.mark();
                let then_terminates = walk_stmt(&arm.then, state, obligations, env);
                let then_added = state.rewind(mark);
                // No `else`: only the pre-existing `state` carries forward,
                // exactly like `crate::locals::check_block`'s own `If` arm.
                if i + 1 < arms.len() || else_.is_some() {
                    joins.push((mark, then_added, then_terminates));
                } else if !then_terminates {
                    // What the arm may have written still may have been.
                    state.note_written(&then_added.written);
                }
            }
            let mut rest_terminates = match else_ {
                Some(else_stmt) => walk_stmt(else_stmt, state, obligations, env),
                None => false,
            };
            while let Some((mark, then_added, then_terminates)) = joins.pop() {
                match (then_terminates, rest_terminates) {
                    (true, _) => {}
                    (false, true) => {
                        state.rewind(mark);
                        state.extend(then_added);
                    }
                    (false, false) => state.intersect_since(mark, &then_added),
                }
                rest_terminates = then_terminates && rest_terminates;
            }
            rest_terminates
        }
        StmtKind::While { cond, body } => {
            state.loop_depth += 1;
            scan_expr(cond, state, env);
            let mark = state.mark();
            walk_stmt(body, state, obligations, env);
            state.loop_depth -= 1;
            // The body may run zero times, so nothing it assigns carries
            // forward — same conservative treatment as every loop here. A
            // `readonly` write it makes was refused where it was written.
            state.rewind(mark);
            false
        }
        StmtKind::DoWhile { body, cond } => {
            // The body runs at least once, so its assignments do carry
            // forward — mirrors `crate::locals`'s own `DoWhile` arm.
            state.loop_depth += 1;
            let terminates = walk_stmt(body, state, obligations, env);
            if !terminates {
                scan_expr(cond, state, env);
            }
            state.loop_depth -= 1;
            terminates
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            // `rule:iteration/for-init-clause`: the init clause runs once, before the loop, in
            // this same state — a declaration there is no different from one
            // written on the line above.
            if let Some(decl) = init.decl() {
                walk_stmt(decl, state, obligations, env);
            }
            for e in init.exprs() {
                scan_expr(e, state, env);
            }
            state.loop_depth += 1;
            for e in cond {
                scan_expr(e, state, env);
            }
            let mark = state.mark();
            walk_stmt(body, state, obligations, env);
            for e in step {
                scan_expr(e, state, env);
            }
            state.loop_depth -= 1;
            state.rewind(mark);
            false
        }
        StmtKind::Foreach { subject, body, .. } => {
            scan_expr(subject, state, env);
            state.loop_depth += 1;
            let mark = state.mark();
            walk_stmt(body, state, obligations, env);
            state.loop_depth -= 1;
            state.rewind(mark);
            false
        }
        StmtKind::Switch { subject, cases } => {
            scan_expr(subject, state, env);
            // Same join as `crate::locals`'s own `Switch` arm: a case
            // contributes only when it definitely exits after the switch —
            // ending in a bare `break`/`continue`, or being the last case and
            // falling off the end — excluding one that always
            // returns/throws instead, and joining in the pre-switch `state`
            // too when there's no `default` (see that module's docs for why).
            let last_index = cases.len().saturating_sub(1);
            let mut candidates: Vec<Added> = Vec::new();
            let mark = state.mark();
            for (i, case) in cases.iter().enumerate() {
                if let Some(c) = &case.cond {
                    scan_expr(c, state, env);
                }
                let terminates = walk_stmts(&case.body, state, obligations, env);
                let case_added = state.rewind(mark);
                if terminates {
                    continue;
                }
                let exits = case.body.last();
                if exits.is_some_and(crate::locals::ends_in_break_or_continue) || i == last_index {
                    candidates.push(case_added);
                }
            }
            if cases.iter().all(|c| c.cond.is_some()) {
                candidates.push(state.rewind(mark));
            }
            if let Some(merged) = added_on_every_way(candidates) {
                state.extend(merged);
            }
            false
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            // Same join as `crate::locals`'s own `Try` arm: `body`/each
            // `catch` start fresh from the pre-`try` `state` (an exception
            // can interrupt `body` before any of its own assignments run),
            // and each contributes to the post-`try` state only when it
            // finishes normally rather than always returning/throwing.
            //
            // What `body` may have written is the opposite case: an exception
            // can stop it after any of its writes, so every `catch` starts
            // from all of them, and `finally` from what any part may write.
            let mut candidates: Vec<Added> = Vec::new();
            let mark = state.mark();
            let body_terminates = walk_stmts(&body.stmts, state, obligations, env);
            let body_added = state.rewind(mark);
            let body_written = body_added.written.clone();
            let mut written = body_written.clone();
            if !body_terminates {
                candidates.push(body_added);
            }
            for catch in catches {
                state.note_written(&body_written);
                let catch_terminates = walk_stmts(&catch.body.stmts, state, obligations, env);
                let catch_added = state.rewind(mark);
                written.extend(catch_added.written.iter().cloned());
                if !catch_terminates {
                    candidates.push(catch_added);
                }
            }
            let merged = added_on_every_way(candidates);
            // `finally` always runs, so it applies on top of whichever
            // candidate above actually happened — union it in rather than
            // discarding the candidates' join.
            if let Some(finally) = finally {
                state.note_written(&written);
                walk_stmts(&finally.stmts, state, obligations, env);
                let finally_added = state.rewind(mark);
                state.extend(merged.unwrap_or_default());
                state.extend(finally_added);
            } else if let Some(m) = merged {
                state.extend(m);
            }
            false
        }
        StmtKind::Echo(xs) => {
            for x in xs {
                scan_expr(x, state, env);
            }
            false
        }
        StmtKind::Unset(xs) => {
            for x in xs {
                scan_expr(x, state, env);
                if let Some((name, span)) = this_property_written(x, env) {
                    state.write(name, span);
                }
            }
            false
        }
        StmtKind::LocalDecl {
            value: Some(value), ..
        } => {
            scan_expr(value, state, env);
            false
        }
        StmtKind::Destructure { value, .. } => {
            scan_expr(value, state, env);
            false
        }
        _ => false,
    }
}

/// Records what `e` is worth to `state` — a `$this->prop = ...` assignment, a
/// `parent::constructor(...)` call — and descends into every expression `e`
/// evaluates, so no composite form hides one.
///
/// The descent is [`each_child_expr`]'s, which is total over the productions
/// [`nvs_syntax::ast::ExprKind`] holds, except for the forms
/// [`scan_branches`] takes first because not all of their operands run.
pub(crate) fn scan_expr(e: &Expr, state: &mut InitState, env: &Env<'_>) {
    if scan_branches(e, state, env) {
        return;
    }
    // The target of a plain `$this->p = v` is the one `$this->p` that is not
    // a read. A compound operator reads the target first.
    match &e.kind {
        ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            value,
            ..
        } if this_property(target, env).is_some() => scan_expr(value, state, env),
        // `$this->prop` reads one property, and `$this` goes nowhere.
        _ if this_property(e, env).is_some() => {}
        ExprKind::Variable(_) if is_this_receiver(e, env.src) => state.escape(e.span, false),
        ExprKind::Fn(_) if captures_this(e, env) => state.escape(e.span, false),
        _ => each_child_expr(e, &mut |child| scan_expr(child, state, env)),
    }
    if let Some(name) = this_property(e, env) {
        state.read(name, e.span);
    }
    match &e.kind {
        ExprKind::Assign { op, target, .. } => {
            if let Some((name, span)) = this_property_written(target, env) {
                state.write(name, span);
            }
            if let Some(name) = this_property(target, env)
                && *op == AssignOp::Assign
            {
                state.assigned.insert(name.to_owned());
            }
        }
        ExprKind::PreIncDec { expr, .. } | ExprKind::PostIncDec { expr, .. } => {
            if let Some((name, span)) = this_property_written(expr, env) {
                state.write(name, span);
            }
        }
        ExprKind::StaticCall {
            class,
            method: MemberName::Ident(name_span),
            ..
        } if matches!(class.kind, ExprKind::ParentExpr)
            && span_text(env.src, *name_span) == "constructor" =>
        {
            state.escape(e.span, true);
            if (state.loop_depth > 0 || state.written.contains(PARENT_CALL))
                && !state.second_parent_calls.contains(&e.span)
            {
                state.second_parent_calls.push(e.span);
            }
            state.written.insert(PARENT_CALL.to_owned());
            state.parent_called = true;
        }
        ExprKind::StaticCall { class, method, .. } if passes_this(class, method, state, env) => {
            state.escape(e.span, false);
        }
        _ => {}
    }
}

/// Whether the anonymous function `e` captures `$this`, as the checker
/// recorded it when it checked the function's body.
fn captures_this(e: &Expr, env: &Env<'_>) -> bool {
    matches!(
        env.exprs.lookup(e.span),
        Some(crate::expr_table::ExprInfo::AnonFn { captures, .. })
            if captures.iter().any(|(name, _)| name == "this")
    )
}

/// Whether `class::method()` calls an instance method with `$this` as its
/// receiver: `class` is `parent`, `self` or `static`, and the method it names
/// is not `static`. A name that does not resolve counts, so an escape the
/// checker cannot see is refused.
fn passes_this(class: &Expr, method: &MemberName, state: &InitState, env: &Env<'_>) -> bool {
    let Some(own) = &state.class else {
        return false;
    };
    let target = match class.kind {
        ExprKind::SelfExpr | ExprKind::StaticExpr => Some(own.clone()),
        ExprKind::ParentExpr => env
            .graph
            .get(own)
            .and_then(|links| links.extends.first().cloned()),
        _ => return false,
    };
    let MemberName::Ident(name) = method else {
        return true;
    };
    target
        .and_then(|target| {
            resolve_method(
                &target,
                span_text(env.src, *name),
                env.signatures,
                env.graph,
            )
        })
        .is_none_or(|(_, sig)| !sig.is_static)
}

/// The property `target` names, when it is `$this->name` written out.
fn this_property<'s>(target: &Expr, env: &Env<'s>) -> Option<&'s str> {
    match &target.kind {
        ExprKind::PropertyAccess {
            object,
            property: MemberName::Ident(name_span),
            ..
        } if is_this_receiver(object, env.src) => Some(strip_sigil(span_text(env.src, *name_span))),
        _ => None,
    }
}

/// The property a write to `target` writes, when its root is `$this->name`,
/// with the span of that root. An element write (`$this->list[0] = v`,
/// `$this->list[] = v`) writes its root property: the changed copy of the
/// array is stored back into it, so it counts against a `readonly` property's
/// one write. It does not assign the property, which [`this_property`] alone
/// decides.
fn this_property_written<'s>(target: &Expr, env: &Env<'s>) -> Option<(&'s str, Span)> {
    let mut root = target.unparenthesized();
    while let ExprKind::Index { base, .. } = &root.kind {
        root = base.unparenthesized();
    }
    this_property(root, env).map(|name| (name, root.span))
}

/// The expression forms whose operands do not all run, joined the way
/// [`walk_stmt`] joins an `if`/`else`: an operand that may be skipped proves
/// nothing, and alternatives are intersected, so an assignment written on one
/// branch is not an assignment on every path out of the constructor.
///
/// Returns whether `e` was one of them, which is [`scan_expr`]'s signal not to
/// also descend into it: a second walk would re-add the very assignments the
/// join just declined.
fn scan_branches(e: &Expr, state: &mut InitState, env: &Env<'_>) -> bool {
    match &e.kind {
        ExprKind::Binary { op, lhs, rhs } if op.short_circuits() => {
            // The right operand of `&&`, `||` and `??` runs only for some
            // values of the left, so nothing it assigns is proven here, and
            // only what it may write is kept.
            scan_expr(lhs, state, env);
            let mark = state.mark();
            scan_expr(rhs, state, env);
            let added = state.rewind(mark);
            state.note_written(&added.written);
        }
        ExprKind::Ternary { cond, then, else_ } => {
            scan_expr(cond, state, env);
            let mark = state.mark();
            // The Elvis form `cond ?: else_` writes no `then` branch of its
            // own: the path that skips `else_` leaves `cond`'s state as it is.
            if let Some(then) = then {
                scan_expr(then, state, env);
            }
            let taken = state.rewind(mark);
            scan_expr(else_, state, env);
            state.intersect_since(mark, &taken);
        }
        ExprKind::Match { subject, arms } => {
            scan_expr(subject, state, env);
            // A `match` that matches no arm throws, so every path that leaves
            // it normally ran exactly one arm — and an arm's conditions are
            // tried until one matches, so none of those is on every path.
            let mark = state.mark();
            let mut ways = Vec::with_capacity(arms.len());
            for arm in arms {
                scan_expr(&arm.body, state, env);
                ways.push(state.rewind(mark));
            }
            if let Some(joined) = added_on_every_way(ways) {
                state.extend(joined);
            }
        }
        ExprKind::Catch { guarded, arms } => {
            // An arm runs on a path where `guarded` threw partway, so what
            // `guarded` assigned is not proven on it: every arm starts from
            // the state this expression was entered in.
            // What `guarded` may have written before it threw is kept on
            // every arm.
            let mark = state.mark();
            scan_expr(guarded, state, env);
            let guarded_added = state.rewind(mark);
            let guarded_written = guarded_added.written.clone();
            let mut ways = vec![guarded_added];
            for arm in arms {
                state.note_written(&guarded_written);
                scan_expr(&arm.body, state, env);
                ways.push(state.rewind(mark));
            }
            if let Some(joined) = added_on_every_way(ways) {
                state.extend(joined);
            }
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap, code};
    use nvs_hir::resolve_file;
    use nvs_syntax::parse_file;

    use crate::expr_table::ExprTypeTable;
    use crate::ty::TypeInterner;

    fn check_src(src: &str) -> Diagnostics {
        check_tables(src).0
    }

    fn check_tables(src: &str) -> (Diagnostics, ExprTypeTable) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        crate::check_program(
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        (diags, exprs)
    }

    #[test]
    fn a_class_with_no_constructor_and_no_default_is_diagnosed() {
        let diags = check_src("<?nvs\nclass Foo {\n  public int $count;\n}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_class_with_no_constructor_but_an_inline_default_is_fine() {
        let diags = check_src("<?nvs\nclass Foo {\n  public int $count = 0;\n}\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_class_with_no_constructor_but_a_nullable_property_is_fine() {
        let diags = check_src("<?nvs\nclass Foo {\n  public ?int $count;\n}\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_constructor_assigning_every_property_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(int $c) {\n    $this->count = $c;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_constructor_leaving_a_property_unassigned_on_one_branch_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(bool $flag) {\n    if ($flag) {\n      $this->count = 1;\n    }\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_constructor_assigning_on_both_if_branches_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(bool $flag) {\n    if ($flag) {\n      $this->count = 1;\n    } else {\n      $this->count = 2;\n    }\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_subclass_constructor_skipping_parent_constructor_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Base {\n  public int $id;\n  function constructor(int $id) {\n    $this->id = $id;\n  }\n}\nclass Sub extends Base {\n  function constructor() {\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_MISSING_PARENT_CONSTRUCTOR_CALL)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_subclass_constructor_calling_parent_constructor_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Base {\n  public int $id;\n  function constructor(int $id) {\n    $this->id = $id;\n  }\n}\nclass Sub extends Base {\n  function constructor(int $id) {\n    parent::constructor($id);\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_hooked_property_that_reads_no_slot_is_exempt_from_the_check() {
        let diags = check_src("<?nvs\nclass Foo {\n  public int $count { get => 1; }\n}\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_set_hook_that_does_not_always_store_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $n { set (int $v) { if ($v > 0) { $this->n = $v; } } }\n  public function constructor() { $this->n = 1; }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_constructor_read_before_the_assignment_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $x;\n  public function constructor() { echo $this->x; $this->x = 1; }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    /// `rule:classes/lateinit-restrictions`: a `lateinit` property is exempt from `rule:classes/definite-property-initialization`'s
    /// constructor-must-assign obligation entirely, whether or not the class
    /// even has a constructor.
    #[test]
    fn a_lateinit_property_is_exempt_from_the_constructor_check() {
        let diags = check_src(
            "<?nvs\nclass Logger {}\nclass Widget {\n  public lateinit Logger $logger;\n  function constructor() {\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `walk_stmt`'s `Switch` arm: every case ends in a `break`, and a
    /// `default` covers "no case matched" — so `$count` is assigned on every
    /// path out of the constructor.
    #[test]
    fn a_switch_with_default_and_a_break_in_every_case_satisfies_the_property() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(int $x) {\n    switch ($x) {\n      case 1:\n        $this->count = 1;\n        break;\n      default:\n        $this->count = 2;\n        break;\n    }\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// Same shape but with no `default` arm: "no case matched" leaves
    /// `$count` unassigned on that path, so it's still diagnosed.
    #[test]
    fn a_switch_with_no_default_does_not_satisfy_the_property() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(int $x) {\n    switch ($x) {\n      case 1:\n        $this->count = 1;\n        break;\n    }\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    /// `walk_stmt`'s `Try` arm: `body` completing and `catch` completing
    /// both assign `$count`, so it's satisfied on every path.
    #[test]
    fn a_try_and_its_catch_both_assigning_satisfies_the_property() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor() {\n    try {\n      $this->count = 1;\n    } catch (LogicError $e) {\n      $this->count = 2;\n    }\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `scan_expr`'s descent is total over the AST, so a form carrying no
    /// meaning of its own — parentheses here — cannot hide the assignment
    /// inside it.
    #[test]
    fn a_constructor_assigning_inside_parentheses_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor() {\n    ($this->count = 1);\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `scan_branches`'s `Match` arm: a `match` that matches no arm throws, so
    /// every path leaving it normally ran one — and every one of these assigns.
    #[test]
    fn a_constructor_assigning_in_every_match_arm_is_fine() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(int $x) {\n    int $y = match ($x) { 1 => $this->count = 1, default => $this->count = 2 };\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The same shape with one arm that assigns nothing: the join intersects
    /// the arms, so `$count` is not proven.
    #[test]
    fn a_constructor_assigning_in_only_one_match_arm_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(int $x) {\n    int $y = match ($x) { 1 => $this->count = 1, default => 2 };\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    /// `scan_branches`'s `Ternary` arm, and the reason the descent alone is
    /// not enough: reaching the assignment is not proving it ran.
    #[test]
    fn a_constructor_assigning_on_one_ternary_branch_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor(bool $flag) {\n    int $y = $flag ? $this->count = 1 : 0;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    /// An anonymous function's body runs when the callable is called, so storing one that
    /// would assign `$count` is not assigning it — the module docs' bound.
    #[test]
    fn a_constructor_assigning_only_inside_an_anon_fn_body_is_diagnosed() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor() {\n    var $later = fn (): int => $this->count = 1;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }

    fn second_writes(src: &str) -> usize {
        check_src(src)
            .iter()
            .filter(|d| d.code == Some(code::E_READONLY_WRITE_AFTER_CONSTRUCTION))
            .count()
    }

    #[test]
    fn a_readonly_property_written_once_on_each_branch_is_fine() {
        assert_eq!(
            second_writes(
                "<?nvs\nclass Foo {\n  public readonly int $id;\n  function constructor(bool $b) {\n    if ($b) {\n      $this->id = 1;\n    } else {\n      $this->id = 2;\n    }\n  }\n}\n",
            ),
            0
        );
    }

    #[test]
    fn a_readonly_property_written_after_a_branch_that_wrote_it_is_refused() {
        assert_eq!(
            second_writes(
                "<?nvs\nclass Foo {\n  public readonly int $id;\n  function constructor(bool $b) {\n    if ($b) {\n      $this->id = 1;\n    }\n    $this->id = 2;\n  }\n}\n",
            ),
            1
        );
    }

    #[test]
    fn a_readonly_property_written_in_a_try_and_its_catch_is_refused() {
        assert_eq!(
            second_writes(
                "<?nvs\nclass Foo {\n  public readonly int $id;\n  function constructor() {\n    try {\n      $this->id = 1;\n    } catch (LogicError $e) {\n      $this->id = 2;\n    }\n  }\n}\n",
            ),
            1
        );
    }

    #[test]
    fn a_readonly_property_written_in_a_loop_or_over_a_promotion_is_refused() {
        assert_eq!(
            second_writes(
                "<?nvs\nclass Foo {\n  public readonly int $id;\n  function constructor(public readonly int $n) {\n    $this->n = 2;\n    foreach ([1, 2] as int $i) {\n      $this->id = $i;\n    }\n    $this->id = 0;\n  }\n}\n",
            ),
            2
        );
    }

    #[test]
    fn a_readonly_property_written_after_a_branch_that_returned_is_fine() {
        assert_eq!(
            second_writes(
                "<?nvs\nclass Foo {\n  public readonly int $id;\n  function constructor(bool $b) {\n    if ($b) {\n      $this->id = 1;\n      return;\n    }\n    $this->id = 2;\n  }\n}\n",
            ),
            0
        );
    }

    /// How many uses of `$this` a constructor makes before construction's
    /// first phase is over.
    fn early_escapes(src: &str) -> usize {
        check_src(src)
            .iter()
            .filter(|d| {
                d.code == Some(code::E_UNINITIALIZED_PROPERTY)
                    && (d.message.contains("`$this` is used before")
                        || d.message
                            .contains("`parent::constructor(...)` is called before"))
            })
            .count()
    }

    const PEEK: &str = "  public function peek(): void { echo $this->name; }\n";

    #[test]
    fn this_used_before_every_property_is_assigned_is_refused() {
        for early in [
            "$this->peek();",
            "Core\\Arr::count([$this]);",
            "var $f = fn (): void => $this->peek();",
            "self::peek();",
            "if ($this->id > 0) { $this->peek(); }",
            "throw new LogicError(\"no\" . Core\\Arr::count([$this]));",
        ] {
            let src = format!(
                "<?nvs\nclass A {{\n  public int $id = 1;\n  public string $name;\n  public function constructor() {{\n    {early}\n    $this->name = \"n\";\n  }}\n{PEEK}}}\n"
            );
            assert_eq!(early_escapes(&src), 1, "{early}");
        }
    }

    #[test]
    fn this_used_once_every_property_is_assigned_is_fine() {
        let diags = check_src(&format!(
            "<?nvs\nclass A {{\n  public string $name;\n  public function constructor() {{\n    $this->name = \"n\";\n    $this->peek();\n    var $f = fn (): void => $this->peek();\n    self::make();\n  }}\n  public static function make(): void {{}}\n{PEEK}}}\n"
        ));
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn this_used_after_a_branch_that_assigned_on_one_side_is_refused() {
        let src = format!(
            "<?nvs\nclass A {{\n  public string $name;\n  public function constructor(bool $b) {{\n    if ($b) {{ $this->name = \"n\"; }}\n    $this->peek();\n    $this->name = \"m\";\n  }}\n{PEEK}}}\n"
        );
        assert_eq!(early_escapes(&src), 1);
    }

    #[test]
    fn a_subclass_assigns_its_own_properties_before_the_parent_constructor() {
        let parent = "class P {\n  public string $name;\n  public function constructor() { $this->name = \"p\"; }\n}\n";
        let late = format!(
            "<?nvs\n{parent}class C extends P {{\n  public int $n;\n  public function constructor() {{\n    parent::constructor();\n    $this->n = 1;\n  }}\n}}\n"
        );
        assert_eq!(early_escapes(&late), 1);
        let first = format!(
            "<?nvs\n{parent}class C extends P {{\n  public int $n;\n  public function constructor() {{\n    $this->n = 1;\n    parent::constructor();\n  }}\n}}\n"
        );
        assert!(!check_src(&first).has_errors(), "{:?}", check_src(&first));
    }

    #[test]
    fn this_used_before_the_parent_constructor_is_refused() {
        let src = format!(
            "<?nvs\nclass P {{\n  public string $name;\n  public function constructor() {{ $this->name = \"p\"; }}\n{PEEK}}}\nclass C extends P {{\n  public function constructor() {{\n    $this->peek();\n    parent::peek();\n    parent::constructor();\n  }}\n}}\n"
        );
        assert_eq!(early_escapes(&src), 2);
    }

    fn direct_constructor_calls(src: &str) -> usize {
        check_src(src)
            .iter()
            .filter(|d| d.code == Some(code::E_CONSTRUCTOR_CALLED_DIRECTLY))
            .count()
    }

    const BOX: &str = "class Box {\n  public readonly int $n;\n  public function constructor(int $n) { $this->n = $n; }\n}\n";

    #[test]
    fn a_constructor_called_by_name_outside_parent_is_refused() {
        for call in [
            "$b->constructor(2);",
            "mixed $m = $b; $m->constructor(2);",
            "Box::constructor(2);",
            "var $f = Box::constructor;",
            "var $f = $b->constructor(...);",
        ] {
            let src = format!("<?nvs\n{BOX}var $b = new Box(1);\n{call}\n");
            assert_eq!(direct_constructor_calls(&src), 1, "{call}");
        }
        let inside = format!(
            "<?nvs\n{BOX}class Sub extends Box {{\n  public function constructor() {{\n    parent::constructor(1);\n    self::constructor(2);\n    var $f = fn (): void => parent::constructor(3);\n  }}\n  public function again(): void {{ parent::constructor(4); }}\n}}\n"
        );
        assert_eq!(direct_constructor_calls(&inside), 3);
        let named =
            format!("<?nvs\n{BOX}?{{}} $none = Core\\Attributes::get<{{}}>(Box::constructor);\n");
        assert_eq!(
            direct_constructor_calls(&named),
            0,
            "{:?}",
            check_src(&named)
        );
    }

    #[test]
    fn a_second_parent_constructor_call_on_a_path_is_refused() {
        for body in [
            "parent::constructor(1);\n    parent::constructor(2);",
            "if ($b) { parent::constructor(1); }\n    parent::constructor(2);",
            "for (var $i = 0; $i < 2; $i++) { parent::constructor($i); }",
        ] {
            let src = format!(
                "<?nvs\n{BOX}class Sub extends Box {{\n  public function constructor(bool $b) {{\n    {body}\n  }}\n}}\n"
            );
            assert_eq!(direct_constructor_calls(&src), 1, "{body}");
        }
        let once = format!(
            "<?nvs\n{BOX}class Sub extends Box {{\n  public function constructor(bool $b) {{\n    if ($b) {{ parent::constructor(1); }} else {{ parent::constructor(2); }}\n  }}\n}}\n"
        );
        assert_eq!(direct_constructor_calls(&once), 0, "{:?}", check_src(&once));
    }

    const HOOKED: &str = "  public int $m {\n    get => $this->m;\n    set => $value;\n  }\n";

    /// The slots `check_tables(src)` arms in `class`, after a clean check.
    fn armed(src: &str, class: &str) -> (Vec<String>, ExprTypeTable) {
        let (diags, exprs) = check_tables(src);
        assert!(!diags.has_errors(), "{diags:?}");
        (exprs.pipeline_armed(class).to_vec(), exprs)
    }

    #[test]
    fn a_property_assigned_before_the_first_hooked_access_starts_set() {
        let src = format!(
            "<?nvs\nclass Cell {{\n  public int $n;\n{HOOKED}  public function constructor(int $n) {{\n    $this->n = $n;\n    $this->m = $n;\n  }}\n}}\n"
        );
        let (armed, exprs) = armed(&src, "Cell");
        assert_eq!(armed, ["m"]);
        assert!(!exprs.read_may_find_unset("Cell", "n"));
        assert!(exprs.read_may_find_unset("Cell", "m"));
    }

    #[test]
    fn a_property_assigned_after_a_hooked_write_starts_unset() {
        let src = format!(
            "<?nvs\nclass Cell {{\n  public int $n;\n{HOOKED}  public function constructor(int $n) {{\n    $this->m = $n;\n    $this->n = $n;\n  }}\n}}\n"
        );
        let (armed, exprs) = armed(&src, "Cell");
        assert_eq!(armed, ["m", "n"]);
        assert!(exprs.read_may_find_unset("Cell", "n"));
    }

    #[test]
    fn a_hooked_write_before_the_parent_constructor_arms_every_inherited_slot() {
        let src = format!(
            "<?nvs\nclass Base {{\n  public int $a;\n  public function constructor(int $a) {{\n    $this->a = $a;\n  }}\n}}\nclass Sub extends Base {{\n{HOOKED}  public function constructor() {{\n    $this->m = 1;\n    parent::constructor(2);\n  }}\n}}\n"
        );
        let (armed, exprs) = armed(&src, "Sub");
        assert_eq!(armed, ["a", "m"]);
        assert!(exprs.read_may_find_unset("Base", "a"));
        assert!(exprs.pipeline_armed("Base").is_empty());
    }

    /// The `catch` doesn't assign `$count`, so it's not satisfied on every
    /// path — the join now depends on what's actually inside `catch`.
    #[test]
    fn a_try_whose_catch_does_not_assign_does_not_satisfy_the_property() {
        let diags = check_src(
            "<?nvs\nclass Foo {\n  public int $count;\n  function constructor() {\n    try {\n      $this->count = 1;\n    } catch (LogicError $e) {\n    }\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNINITIALIZED_PROPERTY)),
            "{diags:?}"
        );
    }
}
