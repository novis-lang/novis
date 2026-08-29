//! `Core\Task` — [ADR 0072](../../../../docs/adr/0072-core-task-structured-concurrency.md)'s
//! structured concurrency: two members, and the group each one hands its host.
//!
//! §§ 1 and 2's typing is the half that reaches furthest. `Task::all`
//! takes a shape literal of zero-argument `fn` literals and answers a shape
//! with the same field names, each field carrying *that field's* declared
//! return type. [`crate::registry::CoreTy::CallableShapeTo`] is the mechanism
//! and owns why no ordinary type at that position could say it;
//! `nvs_types::expr::args` is the one place a field is read, and the one place
//! `E0773`/`E0774` are reported.
//!
//! `Task::map` needs none of that, and the contrast is § 2's whole argument for
//! two members rather than one. Its subject is an `array<T>` and its callback
//! is written once for every element, so one ordinary
//! [`crate::registry::CoreTy::CallableTo`] binds `U` from that one callback's
//! declared return type and the answer is `array<U>` — the same three-line
//! shape `Core\Arr::map` already has, plus § 3's options bag. A shape literal
//! cannot express "one per element of a runtime array" and an array cannot
//! carry a per-element type; each member is the cheap spelling of exactly the
//! job the other cannot state.
//!
//! # Both bodies are the same three steps
//!
//! Neither member runs a child itself. [`nvs_runtime::host`] is the seam, and
//! that module's docs are the one home for why it is a thread-local declared in
//! `nvs-runtime` rather than a dependency on `nvs-host`, and why what crosses
//! it is a **whole group** rather than a `spawn`/`wait`/`cancel` for this
//! member to sequence. So each body is:
//!
//! 1. Turn its argument into one [`Job`](nvs_runtime::host::Job) per child —
//!    the one thing the two members do differently, since a shape literal
//!    carries a different closure per field and an array shares one callback.
//! 2. Hand the jobs and § 3's [`Bounds`](nvs_runtime::host::Bounds) to the host
//!    through [`run_group`](nvs_runtime::host::Host::run_group), which returns
//!    only with nothing still running (§ 4).
//! 3. Put the answers back into the shape the signature promised — the same
//!    field names for `all`, the same keys and order for `map`.
//!
//! **A job's captures are borrowed, not owned**, which is what discharges the
//! release obligation [`Job`](nvs_runtime::host::Job) documents for a job that
//! is dropped without ever being called: the closure a field holds and the
//! element `map` passes both belong to *this* frame's arguments, which outlive
//! the group by construction — the call does not return while a child is still
//! running, and a cancelled parent's children are unwound without being
//! resumed. The one thing a job owns is `map`'s rendered `$key`, and it owns it
//! as a Rust `String` rather than as a [`Value`] precisely so that dropping the
//! job releases it with no `Drop` of our own to remember.
//!
//! **Known gap: a write to a field of `all`'s result is checked against the
//! wrong tag.** The result is built with the argument's own class descriptor,
//! because ADR 0036's shape class is named for its field names alone
//! (`nvs_ir::lower::shape_class_label`) and those are identical on both sides —
//! but that descriptor's per-slot tags come from the *literal*, where every
//! field is a closure. Reading is unaffected (`SlotGet` keys on the name);
//! `$page->count = 5` on a result whose `count` field came from
//! `fn (): int` is refused with a message naming `object`. The fix is for
//! `nvs-ir` to record the *result* shape's representations at the call site,
//! which degrades the shared class's tags to unchecked exactly as two
//! disagreeing literals of the same shape already do —
//! `nvs_runtime::object`'s module doc § *What a shape write checks* owns that
//! mechanism and its other known gaps.
//!
//! # `{limit, deadline}` is the only optioned spelling
//!
//! § 3: one trailing options shape (ADR 0063 R2), the same two fields on `all`
//! and on the `map` that follows it. Both default to [`Const::Null`] and both
//! mean "unbounded" there — `limit` because a shape literal is already bounded
//! by its field count, and `deadline` because a call that names none is bounded
//! by the request tree's own `wall_time`, which ADR 0005 makes finite. There is
//! no `timeout` member and no `race`: a timeout on a group *is* the `deadline`
//! option, and § 3 defers `race` under the future spelling `Task::first`.

use std::time::Duration;

use nvs_runtime::host::{Bounds, Job, Outcome};
use nvs_runtime::{Ctx, Fault, NvsArray, NvsObj, ThrownClass, Value};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};
use crate::time::DURATION_NAME;

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Task";

/// § 3's `{limit?: uint, deadline?: Duration}`, in the order the ABI passes
/// them.
///
/// Both are [`Const::Null`] rather than a number, for that variant's own
/// stated reason: neither type has an "unbounded" value in it, and a `limit`
/// of `0` would be a real bound meaning "run nothing". The module doc above is
/// the one home for what each default means.
const OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "limit",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "deadline",
        ty: CoreTy::Instance(DURATION_NAME),
        default: Const::Null,
    },
];

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "all",
            params: &[CoreTy::CallableShapeTo("S"), CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Var("S"),
            symbol: "nvs_core_task_all",
        },
        CoreMethod {
            name: "map",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableTo("U"),
                CoreTy::Options(OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("U")),
            symbol: "nvs_core_task_map",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_task_all" => (nvs_core_task_all as *const ()).cast(),
        "nvs_core_task_map" => (nvs_core_task_map as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The three steps both members share
// ============================================================================

/// § 3's two options, decoded off the trailing slots that start at `at`.
///
/// `null` is "unspecified" in both, which is [`Const::Null`]'s own meaning
/// here and [`Bounds`]' `None` on the other side — the module doc above is the
/// one home for why neither default is a number.
///
/// # Errors
///
/// A `limit` of `0`, as a catchable `LogicError`. § 3 makes exceeding the limit
/// *schedule* rather than throw, so every positive value is a shaper and none
/// of them is an error; zero is the one value that is not a shaper at all — it
/// admits no child, so the group could never reach § 4's "every child
/// returned" and the call would wait for a completion nothing can produce.
/// Refusing it where it is written is the only answer that is not a wedge
/// (`AGENTS.md`'s priority 1), and a program can write it, so ADR 0002 makes it
/// a throw rather than a fault.
fn bounds(args: &[Value], at: usize, member: &str) -> Result<Bounds, Fault> {
    let limit = match args[at].as_uint() {
        None => None,
        Some(0) => {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{member}: `limit` is how many children may run at once, so `0` is a group \
                     that can never finish; omit the option for unbounded"
                ),
            ));
        }
        // Saturating rather than refusing: a limit past `u32::MAX` is one no
        // shape literal and no array can reach, so it means the same thing the
        // largest representable one does.
        Some(count) => Some(u32::try_from(count).unwrap_or(u32::MAX)),
    };
    let deadline = if args[at + 1].obj_ptr().is_some() {
        let nanos = crate::time::nanos_of(args, at + 1, "deadline")?;
        // A deadline already in the past is a group that is out of time before
        // it starts, which `Duration::ZERO` says exactly.
        Some(Duration::from_nanos(u64::try_from(nanos).unwrap_or(0)))
    } else {
        None
    };
    Ok(Bounds { limit, deadline })
}

/// Runs one child's closure and leaves any failure where the host reads it.
///
/// A [`Job`] answers with a [`Value`] and has no error channel, because the
/// host takes a child's failure off the child's own context — `Ctx::pending`
/// is what `nvs_host::group` tests, and § 4's *first* throw is decided from
/// there. So a fault that would ordinarily become this helper's return status
/// is recorded on the child's context instead, which is the same translation
/// `nvs_runtime`'s helper ABI performs one frame further out.
fn call_child(ctx: &mut Ctx, callback: Value, args: &[Value]) -> Value {
    match nvs_runtime::call_closure(ctx, callback, args) {
        Ok(answer) => answer,
        // The callee already recorded what failed; that is the whole of what
        // this variant means.
        Err(Fault::Pending(_)) => Value::null(),
        Err(Fault::Thrown(class, message)) => {
            ctx.set_pending_as(class, message);
            Value::null()
        }
        Err(Fault::ThrownWithIssues(class, message, issues)) => {
            #[expect(
                unsafe_code,
                reason = "the fault transferred this reference, and \
                          `raise_with_issues` transfers it on into the exception \
                          object's slot or releases it"
            )]
            unsafe {
                ctx.raise_with_issues(class, &message, issues);
            }
            Value::null()
        }
        Err(Fault::Fatal(message)) => {
            ctx.set_pending(message);
            Value::null()
        }
        // `Fault` is `#[non_exhaustive]`: a variant added later still has to
        // reach the child's context as *something*, and the tier that cannot
        // be silently wrong is the fatal one.
        Err(other) => {
            ctx.set_pending(format!(
                "internal error: an unhandled fault reached a task child: {other:?}"
            ));
            Value::null()
        }
    }
}

/// Hands `jobs` to this thread's host and turns § 4's table into what a member
/// returns.
///
/// # Errors
///
/// The child's throw, re-raised on this context so the class and the trace a
/// child built are the ones that propagate rather than a message copied out of
/// them; `TimeoutError` for an expired deadline, which is the one row of § 4's
/// table the host cannot name because it does not know the class; and a
/// [`Fault::fatal`] for a thread with no host at all, which is an embedder that
/// installed none — `nvs run` and every `.nvst` case go through
/// `nvs_host::Scheduler::run`, which installs one.
fn run_group(
    ctx: &mut Ctx,
    jobs: Vec<Job>,
    bounds: Bounds,
    member: &str,
) -> Result<Vec<Value>, Fault> {
    let outcome = nvs_runtime::host::with_current(|host| host.run_group(ctx, jobs, bounds));
    match outcome {
        Some(Outcome::Completed(answers)) => Ok(answers),
        Some(Outcome::Threw(thrown)) => {
            ctx.raise(thrown);
            Err(Fault::Pending(nvs_runtime::THROWN))
        }
        Some(Outcome::TimedOut) => Err(Fault::thrown_as(
            ThrownClass::Timeout,
            format!("{member}: the deadline expired before every child returned"),
        )),
        None => Err(Fault::fatal(format!(
            "{member} needs a scheduler on this thread and there is none"
        ))),
    }
}

// ============================================================================
// The two members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Task::all({...}, {limit?, deadline?}): S` — ADR 0072 § 1's fixed,
    /// heterogeneous set.
    ///
    /// The argument is an ADR 0036 shape value, which is an ordinary object
    /// whose slots are its fields in sorted name order, so a job per slot is a
    /// job per field and the *order* the host is given is the same order the
    /// answers come back in. The result reuses the argument's class descriptor:
    /// a shape class is named for its field names alone and the two shapes
    /// share them, which is also where this module's one known gap is.
    fn nvs_core_task_all(ctx, args: [3]) {
        let shape = receiver(args, "Core\\Task::all")?;
        let bounds = bounds(args, 1, "Core\\Task::all")?;

        // Every closure is *borrowed* from the argument's slot — the module
        // doc's paragraph on what a job captures owns why nothing is retained.
        let jobs: Vec<Job> = (0..shape.field_count())
            .map(|slot| {
                let closure = shape.field(slot);
                Box::new(move |ctx: &mut Ctx| call_child(ctx, closure, &[])) as Job
            })
            .collect();
        let answers = run_group(ctx, jobs, bounds, "Core\\Task::all")?;

        #[expect(
            unsafe_code,
            reason = "the descriptor is the argument's own, and the context \
                      shares ownership of the table it lives in — `Unit::install_in` \
                      is what makes it outlive both this call and the unit"
        )]
        let result = unsafe { NvsObj::new(shape.class()) };
        for (slot, answer) in answers.into_iter().enumerate() {
            // Each answer is a child's own reference, transferred into the slot.
            result.set_field(slot, answer);
        }
        Ok(Value::object(result))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Task::map(array<T> $items, callable $fn, {limit?, deadline?}): array<U>`
    /// — ADR 0072 § 2's homogeneous case.
    ///
    /// The keys and the order are the subject's, "regardless of completion
    /// order" (§ 2): the jobs are built in the subject's own slot order, the
    /// answers come back in that same order, and each is stored under the key
    /// its entry already had. Nothing sorts and nothing renumbers.
    ///
    /// The callback receives `($value, $key)` and may declare fewer parameters,
    /// the same rule `Core\Arr::map` documents — and the same reason to read
    /// `closure_arity` once before the walk rather than build a key per element
    /// for `nvs_runtime::call_closure` to trim away.
    fn nvs_core_task_map(ctx, args: [4]) {
        // Unreachable from source: the subject is a `CoreTy::Array` parameter,
        // so anything else is `E0401: expected array<T>, found …` at the call.
        // A wrong tag here is compiled code disagreeing with the registry.
        let subject = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Task::map expected an `array`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let base = crate::arr::borrowed(subject);
        let bounds = bounds(args, 2, "Core\\Task::map")?;
        let wants_key = nvs_runtime::closure_arity(args[1])? >= 2;
        let callback = args[1];

        // The keys stay here, borrowed from the subject, because a job is
        // `'static` and cannot hold one; what a job carries is the *rendered*
        // key, and only where the callback declared a parameter for it.
        let mut keys = Vec::new();
        let mut jobs: Vec<Job> = Vec::new();
        let mut from = 0usize;
        let slots = std::iter::from_fn(|| {
            let slot = base.next_slot(from)?;
            from = slot + 1;
            Some(slot)
        });
        nvs_runtime::bounded_loop(ctx, "Core\\Task::map", slots, |_ctx, slot| {
            let value = base
                .value_at(slot)
                .expect("next_slot only names live entries");
            let key = base
                .slot_key(slot)
                .expect("next_slot only names live entries");
            // A `String` rather than a `Value`, so that a job the group never
            // reaches releases it by being dropped — the module doc's own
            // paragraph on what a job owns.
            let rendered = wants_key.then(|| key.to_str().to_owned());
            keys.push(key);
            jobs.push(Box::new(move |ctx: &mut Ctx| {
                let Some(text) = rendered else {
                    return call_child(ctx, callback, &[value]);
                };
                // One reference for the duration of the call; `call_closure`
                // takes its own.
                let key_arg = Value::str(nvs_runtime::NvsStr::new(text.as_bytes()));
                let answer = call_child(ctx, callback, &[value, key_arg]);
                #[expect(
                    unsafe_code,
                    reason = "this job owns exactly the reference `Value::str` \
                              just produced"
                )]
                unsafe {
                    key_arg.release();
                }
                answer
            }));
            Ok(())
        })?;

        let answers = run_group(ctx, jobs, bounds, "Core\\Task::map")?;
        let mut out = NvsArray::new();
        for (key, answer) in keys.into_iter().zip(answers) {
            crate::arr::store_at(&mut out, key, answer);
        }
        Ok(Value::array(out))
    }
}

/// The shape literal in argument slot 0, borrowed.
///
/// # Errors
///
/// A [`Fault::fatal`] for a non-object: `nvs_types` has already checked that
/// the argument is a shape literal of `fn` literals (`E0773`/`E0774`), so a
/// wrong tag here is compiled code disagreeing with the registry rather than
/// anything a program can write.
fn receiver(args: &[Value], member: &str) -> Result<std::mem::ManuallyDrop<NvsObj>, Fault> {
    let ptr = args[0].obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a shape, got tag {}",
            args[0].tag_byte()
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "an object argument owns a reference to a live allocation, so \
                  it is live for the length of this call, and the handle is \
                  never dropped"
    )]
    Ok(std::mem::ManuallyDrop::new(unsafe {
        NvsObj::from_raw(ptr)
    }))
}
