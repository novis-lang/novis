//! `Core\Task` — `rule:concurrency/one-scheduler`'s
//! structured concurrency: two members that hand their host a group, and a
//! third that hands the request one closure to run once it is over.
//!
//! §§ 1 and 2's typing is the half that reaches furthest. `Task::all` takes a
//! shape of zero-argument callables and answers a shape with the same field
//! names, each field carrying *that field's* declared return type.
//! [`crate::registry::CoreTy::ShapeOfCallables`] is the mechanism and owns why
//! no writable type at that position could say it; `nvs_types::generics`'
//! `bind` is the one place the answer's shape is assembled.
//!
//! `Task::map` needs none of that, and the contrast is § 2's whole argument for
//! two members rather than one. Its subject is an `array<T>` and its callback
//! is written once for every element, so one ordinary
//! [`crate::registry::CoreTy::CallableSig`] binds `U` from that callback's own
//! written return type and the answer is `array<U>` — the same three-line
//! shape `Core\Arr::map` already has, plus § 3's options bag. A shape cannot
//! express "one per element of a runtime array" and an array cannot carry a
//! per-element type; each member is the cheap spelling of exactly the job the
//! other cannot state.
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
//! **`all`'s result is the argument's own class, and the tags it promises are
//! the result's.** Step 3 builds the answer from the argument's class
//! descriptor, because `rule:types/object-top`'s shape class is named for its
//! field names alone (`nvs_ir::lower::shape_class_label`) and those are
//! identical on both sides. That descriptor's per-slot tags would otherwise be
//! the *literal's*, where every field holds a closure, and `$page->count = 5`
//! on a field declared `fn (): int` would be refused for writing an `int` to a
//! slot promising `object`. So the call site records the **result** shape's
//! representations against that same label
//! (`nvs_ir::lower::Lowering::record_core_result_shape`), which degrades every
//! slot the two spell differently to unchecked, exactly as two disagreeing
//! literals of one shape already do — `nvs_runtime::object`'s module doc
//! § *What a shape write checks*, case 4.
//!
//! # `{limit, deadline}` is the only optioned spelling
//!
//! § 3: one trailing options shape (`rule:core-api/shape-rules` R2), the same two fields on `all`
//! and on the `map` that follows it. Both default to [`Const::Null`] and both
//! mean "unbounded" there — `limit` because a shape literal is already bounded
//! by its field count, and `deadline` because a call that names none is bounded
//! by the request tree's own `wall_time`, which `rule:config/three-changeability-classes` makes finite. There is
//! no `timeout` member and no `race`: a timeout on a group *is* the `deadline`
//! option, and § 3 defers `race` under the future spelling `Task::first`.
//!
//! # `afterResponse` is a registration, not a group
//!
//! § 6's member shares the class and nothing else: it starts no child, waits
//! for nothing, and returns while the request is still running. Its whole body
//! is a retain and a push, and everything that makes it a *task* — when the
//! closure runs, what a host with no response does about "after the response",
//! why the queue seals while it drains, where a throw out of it goes, and why
//! § 7's `max_concurrent` is a gap rather than a count — belongs to
//! [`nvs_runtime::deferred`], which is that behaviour's one home.
//! Its options bag is [`DEFERRED_OPTIONS`] rather than [`OPTIONS`] for the same
//! reason: a single closure has no concurrency for a `limit` to shape.

use std::time::Duration;

use nvs_runtime::host::{Bounds, Job, Outcome};
use nvs_runtime::{Ctx, Fault, NvsArray, NvsObj, Tag, ThrownClass, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
};
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

/// § 6's own options bag: one field, because a single deferred closure has no
/// concurrency for a `limit` to shape. `deadline` means what § 7 says rather
/// than what [`OPTIONS`] says — omitted, the call inherits `[deferred]
/// deadline` and not the tree's `wall_time`, which is over by then.
const DEFERRED_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "deadline",
    ty: CoreTy::Instance(DURATION_NAME),
    default: Const::Null,
}];

/// `Core\Task`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Runs work as tasks. `all()` and `map()` run several tasks at the same time and return \
            when every one has finished. `afterResponse()` runs one closure after the response is \
            sent.",
};

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "all",
            names: &["tasks"],
            params: &[CoreTy::ShapeOfCallables("S"), CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Var("S"),
            symbol: "nvs_core_task_all",
            doc: Some(&ALL_DOC),
        },
        CoreMethod {
            name: "map",
            names: &["items", "fn"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(&[CoreTy::Var("T"), CoreTy::Str], &CoreTy::Var("U")),
                CoreTy::Options(OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("U")),
            symbol: "nvs_core_task_map",
            doc: Some(&MAP_DOC),
        },
        CoreMethod {
            name: "afterResponse",
            names: &["fn"],
            params: &[
                CoreTy::CallableSig(&[], &CoreTy::Mixed),
                CoreTy::Options(DEFERRED_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_task_after_response",
            doc: Some(&AFTER_RESPONSE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// § 3's two options, documented once — both members carry the same two.
const LIMIT_DOC: ParamDoc = ParamDoc {
    name: "limit",
    desc: "The most tasks that run at the same time. The other tasks wait until one finishes. \
           Without it, every task starts at once.",
    shape: &[],
};

/// See [`LIMIT_DOC`].
const DEADLINE_DOC: ParamDoc = ParamDoc {
    name: "deadline",
    desc: "The time limit for the whole call, not for each task. Without it, the request's \
           `[limits] wall_time` setting is the limit.",
    shape: &[],
};

/// `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s table, as the two rows a card names — the same on both
/// members, because [`run_group`] is.
const GROUP_ERRORS: &[ErrorDoc] = &[
    ErrorDoc {
        error: "LogicError",
        desc: "When `limit` is `0`. No task could start, so the call could never finish.",
    },
    ErrorDoc {
        error: "TimeoutError",
        desc: "When `deadline` runs out before every task has finished. Every task is stopped \
               first, and the call waits until they have stopped.",
    },
];

/// `Core\Task::all`'s reference card — `rule:core-api/reference-card`.
const ALL_DOC: MethodDoc = MethodDoc {
    short: "Runs every closure in the `$tasks` shape at the same time, and waits until all of \
            them have finished. The result is a shape with the same field names, and each field \
            has the type its closure returns.",
    params: &[
        ParamDoc {
            name: "tasks",
            desc: "A shape whose fields are closures with no parameters, written as `fn` literals \
                   in the call. A variable of type `callable` in a field does not compile.",
            shape: &[],
        },
        LIMIT_DOC,
        DEADLINE_DOC,
    ],
    ret: "A shape with the value each closure returned. No task is still running when the call \
          returns. If a task throws an error, the other tasks are stopped, and the call throws \
          that same error.",
    errors: GROUP_ERRORS,
};

/// `Core\Task::map`'s reference card — `rule:core-api/reference-card`.
const MAP_DOC: MethodDoc = MethodDoc {
    short: "Calls `$fn` once for each element of `$items`, and runs the calls at the same time. \
            The result has the same keys in the same order as `$items`, whichever call finishes \
            first.",
    params: &[
        ParamDoc {
            name: "items",
            desc: "The array. Each element is given to `$fn` in a task of its own.",
            shape: &[],
        },
        ParamDoc {
            name: "fn",
            desc: "The function to call. It gets `($value, $key)`, and it may declare only \
                   `$value`. The key is always a `string`. Its return type is the type of each \
                   element of the result.",
            shape: &[],
        },
        LIMIT_DOC,
        DEADLINE_DOC,
    ],
    ret: "An array with what `$fn` returned for each element, under the element's own key. An \
          empty `$items` gives an empty array. No task is still running when the call returns. \
          If a task throws an error, the other tasks are stopped, and the call throws that same \
          error.",
    errors: GROUP_ERRORS,
};

/// `Core\Task::afterResponse`'s reference card — `rule:core-api/reference-card`.
const AFTER_RESPONSE_DOC: MethodDoc = MethodDoc {
    short: "Runs `$fn` after the response is sent, so the client does not wait for it. Use it for \
            receipts, webhooks and audit logs. The work is not saved anywhere: if the process \
            stops, the work is lost and nothing tries it again.",
    params: &[
        ParamDoc {
            name: "fn",
            desc: "The closure to run. It takes no arguments, and its return value is ignored. An \
                   error it throws is written to the log, and no `catch` in the request sees it.",
            shape: &[],
        },
        ParamDoc {
            name: "deadline",
            desc: "The longest time the closure may run. Without it, the `[deferred] deadline` \
                   setting is the limit. The request's `[limits] wall_time` does not apply here, \
                   but its other `[limits]` settings do.",
            shape: &[],
        },
    ],
    ret: "Nothing. The closures run one at a time, in the order you added them. They do not run \
          if the request ends with an uncaught error, an `exit` or a fatal error.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "When the call is made inside a task that `Core\\Task::all`, `Core\\Task::map` or \
               `afterResponse` started. Only the request itself may call it, so return the work \
               to the request and call `afterResponse` there.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_task_all" => (nvs_core_task_all as *const ()).cast(),
        "nvs_core_task_map" => (nvs_core_task_map as *const ()).cast(),
        "nvs_core_task_after_response" => (nvs_core_task_after_response as *const ()).cast(),
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
/// (`AGENTS.md`'s priority 1), and a program can write it, so `rule:errors/propagation` makes it
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
    match nvs_runtime::call_callable(ctx, callback, args) {
        Ok(answer) => answer,
        // The callee already recorded what failed. Which tier it was is the
        // status, and a `FATAL` is said on the context because a job has no
        // status to return it in.
        Err(Fault::Pending(status)) => {
            if status == nvs_runtime::FATAL {
                ctx.mark_pending_fatal();
            }
            Value::null()
        }
        Err(Fault::Thrown(class, message)) => {
            ctx.set_pending_as(class, message);
            Value::null()
        }
        Err(Fault::ThrownWithSlots(class, message, slots)) => {
            #[expect(
                unsafe_code,
                reason = "the fault transferred these references, and \
                          `raise_with_slots` transfers each on into the \
                          exception object's slot or releases it"
            )]
            unsafe {
                ctx.raise_with_slots(class, &message, &slots);
            }
            Value::null()
        }
        Err(Fault::Fatal(message)) => {
            ctx.set_pending_fatal(message);
            Value::null()
        }
        // `Fault` is `#[non_exhaustive]`: a variant added later still has to
        // reach the child's context as *something*, and the tier that cannot
        // be silently wrong is the fatal one.
        Err(other) => {
            ctx.set_pending_fatal(format!(
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
        Some(Outcome::Fatal(message)) => Err(Fault::fatal(message)),
        Some(Outcome::TimedOut) => Err(Fault::thrown_as(
            ThrownClass::Timeout,
            format!("{member}: the deadline expired before every child returned"),
        )),
        // The *calling* task was cancelled while it waited here — § 4's last
        // row. Not a throw and not a value: the request is over, so the answer
        // is the flag `nvs_safepoint` already reports as a `FATAL` no `catch`
        // sees, and the member returns ordinarily to let the frames between
        // here and that poll unwind by `rule:errors/propagation`'s status. `Host::sleep`'s
        // `Woken::Cancelled` takes the identical route.
        Some(Outcome::Cancelled) => Err(ctx.cancel()),
        None => Err(Fault::fatal(format!(
            "{member} needs a scheduler on this thread and there is none"
        ))),
    }
}

// ============================================================================
// The two members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Task::all({...}, {limit?, deadline?}): S` — `rule:concurrency/all-answers-a-typed-shape`'s fixed,
    /// heterogeneous set.
    ///
    /// The argument is an `rule:types/object-top` shape value, which is an ordinary object
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
                let callable = shape.field(slot);
                Box::new(move |ctx: &mut Ctx| call_child(ctx, callable, &[])) as Job
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
    /// — `rule:concurrency/map-preserves-keys-and-order`'s homogeneous case.
    ///
    /// The keys and the order are the subject's, "regardless of completion
    /// order" (§ 2): the jobs are built in the subject's own slot order, the
    /// answers come back in that same order, and each is stored under the key
    /// its entry already had. Nothing sorts and nothing renumbers.
    ///
    /// The callback receives `($value, $key)` and may declare fewer parameters,
    /// the same rule `Core\Arr::map` documents — and the same reason to read
    /// `callable_arity` once before the walk rather than build a key per element
    /// for `nvs_runtime::call_callable` to trim away.
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
        let wants_key = nvs_runtime::callable_arity(args[1])? >= 2;
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
                // One reference for the duration of the call; `call_callable`
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

nvs_runtime::nvs_helper! {
    /// `Core\Task::afterResponse(callable $fn, {deadline?}): void` — `rule:concurrency/after-response-outlives-the-connection`
    /// 's deferred work.
    ///
    /// The registration is the whole body, and the two things it does that a
    /// caller could not are the retain and the cap. `nvs_runtime::deferred` is
    /// the one home for *when* the closure runs — a host with no response has
    /// to answer that too — and for why the cap counts request trees rather
    /// than closures.
    fn nvs_core_task_after_response(ctx, args: [2]) {
        // The row's first parameter is `CoreTy::Callable`, so `E0401` refuses a
        // `null` at the call and this guard is unreachable from source: what it
        // catches is a lowering bug, and it is here because the slot it would
        // otherwise write is one nothing looks at again until the request is
        // over.
        if args[0].tag() == Some(Tag::Null) {
            return Err(Fault::fatal(
                "Core\\Task::afterResponse expected a closure, got null".to_string(),
            ));
        }
        // § 7: the option the call named, or the tree's own default, both in
        // nanoseconds, where the queue reads `0` as "no deadline". A named
        // deadline of zero or less is out of time before the closure starts,
        // as [`bounds`] reads it for a group, so it becomes the smallest one
        // there is rather than that `0`.
        let deadline = if args[1].obj_ptr().is_some() {
            let nanos = crate::time::nanos_of(args, 1, "deadline")?;
            u64::try_from(nanos).unwrap_or(0).max(1)
        } else {
            ctx.deferred_deadline()
        };
        #[expect(
            unsafe_code,
            reason = "the argument is borrowed from the caller's frame and the \
                      context keeps it past this call, so it needs a reference \
                      of its own — which `Ctx::defer` then owns, or hands back \
                      by refusing"
        )]
        // SAFETY: the value is owned by the caller's argument slot, which
        // outlives this call; the reference taken here is the one the queue
        // releases when the work runs or the request ends.
        unsafe {
            args[0].retain();
        }
        let refused = match ctx.defer(args[0], deadline) {
            Ok(()) => return Ok(Value::null()),
            Err(refused) => refused,
        };
        #[expect(
            unsafe_code,
            reason = "a refused registration kept nothing, so the reference \
                      retained just above is this frame's to release"
        )]
        // SAFETY: `defer` took no ownership on the refusal path, and this is the
        // reference taken immediately above.
        unsafe {
            args[0].release();
        }
        Err(Fault::thrown(match refused {
            // § 6's last bullet: retrying cannot help, so the message says what
            // to do instead rather than what went wrong.
            nvs_runtime::deferred::DeferError::Sealed => {
                "Core\\Task::afterResponse: only the request's own task may defer work, and this \
                 is a child task — a `Core\\Task` child or deferred work itself; \
                 hand it back to the request that started you"
                    .to_string()
            }
            // § 7: load rather than a mistake, so the message names the
            // directive an operator would change and the two things the
            // program can still do about it.
            nvs_runtime::deferred::DeferError::AtCapacity { cap } => format!(
                "Core\\Task::afterResponse: this core is already holding {cap} request trees for \
                 after-response work, which is `[deferred] max_concurrent`; do the work inline, \
                 respond without it, or ask the caller to retry"
            ),
        }))
    }
}

/// The shape literal in argument slot 0, borrowed.
///
/// # Errors
///
/// A [`Fault::fatal`] for a non-object: [`CoreTy::ShapeOfCallables`] accepts a
/// shape and nothing else, and `nvs_types` has already refused everything
/// else where it was written, so a wrong tag here is compiled code disagreeing
/// with the registry rather than anything a program can write.
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use nvs_runtime::host::{
        Bounds, Entry, Host, Job, Narrowing, Outcome, Output, Placement, Running, StartError,
        Waker, Woken,
    };
    use nvs_runtime::{
        CALLABLE_ARITY_SLOT, CALLABLE_INVOKE, CALLABLE_PARAM_TAG_ANY, CALLABLE_PARAM_TAGS_SLOT,
        ClassTable, Ctx, MethodRow, NvsArray, NvsFn, NvsObj, NvsStr, Value,
    };

    /// How many times [`counts_a_run`] was called. Registering must never call
    /// it: the closure runs at the drain, which no test here reaches.
    static RUNS: AtomicUsize = AtomicUsize::new(0);

    /// The registered closure, as the callee side of `call_callable`'s contract:
    /// it counts, sweeps the receiver it was handed and answers nothing.
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes one live value this callee owes a release, \
                  and the address of a live `Value` for the result"
    )]
    unsafe extern "C" fn counts_a_run(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        RUNS.fetch_add(1, Ordering::SeqCst);
        unsafe {
            (*args).release();
            *out = Value::null();
        }
        nvs_runtime::OK
    }

    /// A closure declaring `arity` parameters of any type and calling `invoke`,
    /// owned by the caller — `crates/nvs-stdlib/tests/allocation_policy.rs`'s
    /// `callable_of`, whose doc comment says why this is a whole closure and why
    /// the table leaks.
    fn callable_of(arity: u32, invoke: NvsFn) -> Value {
        let mut table = ClassTable::new();
        let id = table.define("{callable}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![MethodRow {
                name: CALLABLE_INVOKE.to_owned(),
                code: invoke as *const u8,
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_callable(id);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { NvsObj::new(table.desc(id)) };
        let mut tags: u64 = 0;
        for parameter in 0..arity {
            tags |= u64::from(CALLABLE_PARAM_TAG_ANY) << (parameter * 4);
        }
        object.set_field(CALLABLE_ARITY_SLOT, Value::int(i64::from(arity)));
        object.set_field(
            CALLABLE_PARAM_TAGS_SLOT,
            Value::int(i64::from_ne_bytes(tags.to_ne_bytes())),
        );
        Value::object(object)
    }

    /// A registration on the request's own context keeps one reference of its
    /// own, holds one of the core's tree slots, and runs nothing; the context
    /// going down unrun releases both. The same call on a child context is
    /// refused with the sentence that says what to do instead, and keeps
    /// nothing — the queue it would have written is nobody's to drain.
    // covers: Core\Task::afterResponse
    #[test]
    fn after_response_queues_one_reference_on_the_request_and_refuses_a_child() {
        let callable = callable_of(0, counts_a_run);
        let header = callable.obj_ptr().expect("the closure is an object");
        let trees = nvs_runtime::deferred::trees_in_flight();
        let refcount = || {
            #[expect(
                unsafe_code,
                reason = "`closure` keeps the allocation live until the last line of this test"
            )]
            unsafe {
                NvsObj::refcount_of(header)
            }
        };

        let mut request = Ctx::buffered();
        for _ in 0..2 {
            let answer = nvs_runtime::call(
                super::nvs_core_task_after_response,
                &mut request,
                &[callable, Value::null()],
            )
            .expect("the request's own task may defer work");
            assert_eq!(
                answer.tag(),
                Some(nvs_runtime::Tag::Null),
                "`afterResponse` returns nothing"
            );
        }
        assert!(request.has_deferred(), "both registrations are queued");
        assert_eq!(refcount(), 3, "each registration is one more owner");
        assert_eq!(
            nvs_runtime::deferred::trees_in_flight(),
            trees + 1,
            "two registrations hold one tree slot, not two"
        );

        {
            #[expect(
                unsafe_code,
                reason = "`request` outlives the child, which is dropped at the end of this block"
            )]
            let mut child = unsafe { request.child() };
            let refused = nvs_runtime::call(
                super::nvs_core_task_after_response,
                &mut child,
                &[callable, Value::null()],
            );
            assert!(refused.is_err(), "a child task may not defer work");
            let message = child.take_pending().expect("the refusal has a sentence");
            assert!(
                message.contains("only the request's own task may defer work")
                    && message.contains("hand it back to the request that started you"),
                "the refusal says what to do instead, got {message:?}"
            );
            assert!(!child.has_deferred(), "the child queued nothing");
        }
        assert_eq!(refcount(), 3, "a refused registration keeps no reference");

        drop(request);
        assert_eq!(
            refcount(),
            1,
            "a context going down releases its unrun work"
        );
        assert_eq!(
            nvs_runtime::deferred::trees_in_flight(),
            trees,
            "and gives its tree slot back"
        );
        assert_eq!(RUNS.load(Ordering::SeqCst), 0, "registering runs nothing");
        #[expect(
            unsafe_code,
            reason = "the test built the closure and owns its last reference"
        )]
        unsafe {
            callable.release();
        }
    }

    thread_local! {
        /// How many groups [`Serial`] was handed on this thread. A host is
        /// installed per thread, and the test binary runs `all`'s test and
        /// `map`'s at the same time, so one shared count would mix the two.
        static GROUPS: Cell<usize> = const { Cell::new(0) };
    }

    /// A host that runs every job of a group on the caller's own context, one
    /// after the other and in the order given. That order is the only thing
    /// `all` and `map` read their answers back in, so it is all a scheduler
    /// has to be here.
    #[derive(Debug)]
    struct Serial;

    static SERIAL: Serial = Serial;

    impl Host for Serial {
        fn run_group(&self, ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds) -> Outcome {
            GROUPS.with(|groups| groups.set(groups.get() + 1));
            assert_eq!(bounds, Bounds::default(), "no option was named");
            Outcome::Completed(jobs.into_iter().map(|job| job(ctx)).collect())
        }

        fn sleep(&self, _duration: Duration) -> Woken {
            Woken::Elapsed
        }

        fn waker(&self) -> Option<Waker> {
            None
        }

        fn park(&self, _deadline: Option<Instant>) -> Woken {
            Woken::Elapsed
        }

        fn start_isolate(
            &self,
            _ctx: &mut Ctx,
            _entry: Entry,
            _args: Value,
            _output: Output,
            _placement: Placement,
            _narrowing: Narrowing,
        ) -> Result<Box<dyn Running>, StartError> {
            unreachable!("a task group starts no isolate")
        }
    }

    /// The first field's closure: sweeps its receiver and answers `7`.
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes one live value this callee owes a release, \
                  and the address of a live `Value` for the result"
    )]
    unsafe extern "C" fn answers_seven(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe {
            (*args).release();
            *out = Value::int(7);
        }
        nvs_runtime::OK
    }

    /// The second field's closure: sweeps its receiver and answers `11`.
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes one live value this callee owes a release, \
                  and the address of a live `Value` for the result"
    )]
    unsafe extern "C" fn answers_eleven(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        unsafe {
            (*args).release();
            *out = Value::int(11);
        }
        nvs_runtime::OK
    }

    /// `all` answers an object of the argument's own class, each slot holding
    /// what that slot's closure returned, and borrows the closures rather than
    /// keeping them. A `limit` of `0` is refused before any host is asked, and
    /// a thread with no host is a fatal that names the member.
    // covers: Core\Task::all
    #[test]
    fn all_answers_the_arguments_shape_in_slot_order_and_refuses_a_limit_of_zero() {
        let mut table = ClassTable::new();
        let id = table.define("{shape}", &["size", "total"], &[]);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives the shape"
        )]
        let shape = unsafe { NvsObj::new(table.desc(id)) };
        let seven = callable_of(0, answers_seven);
        let eleven = callable_of(0, answers_eleven);
        shape.set_field(0, seven);
        shape.set_field(1, eleven);
        let shape = Value::object(shape);
        let refcounts = || {
            #[expect(
                unsafe_code,
                reason = "`shape` holds both closures until the last line of this test"
            )]
            unsafe {
                [
                    NvsObj::refcount_of(seven.obj_ptr().expect("a closure is an object")),
                    NvsObj::refcount_of(eleven.obj_ptr().expect("a closure is an object")),
                ]
            }
        };

        let mut ctx = Ctx::buffered();
        let no_host = nvs_runtime::call(
            super::nvs_core_task_all,
            &mut ctx,
            &[shape, Value::null(), Value::null()],
        );
        assert!(no_host.is_err(), "a thread with no host runs no group");
        let message = ctx.take_pending().expect("the fatal has a sentence");
        assert!(
            message.contains("Core\\Task::all needs a scheduler"),
            "the fatal names the member, got {message:?}"
        );

        let _installed = nvs_runtime::host::install(&SERIAL);
        let mut ctx = Ctx::buffered();
        let zero = nvs_runtime::call(
            super::nvs_core_task_all,
            &mut ctx,
            &[shape, Value::uint(0), Value::null()],
        );
        assert!(zero.is_err(), "a limit of zero is refused");
        let message = ctx.take_pending().expect("the refusal has a sentence");
        assert!(
            message.contains("`limit` is how many children may run at once"),
            "the refusal says what `limit` means, got {message:?}"
        );
        assert_eq!(GROUPS.with(Cell::get), 0, "no group was started");

        let answer = nvs_runtime::call(
            super::nvs_core_task_all,
            &mut ctx,
            &[shape, Value::null(), Value::null()],
        )
        .expect("a group whose children all return completes");
        assert_eq!(GROUPS.with(Cell::get), 1, "one call is one group");
        {
            let result = super::receiver(&[answer], "test").expect("the answer is an object");
            let argument = super::receiver(&[shape], "test").expect("the shape is an object");
            assert!(
                std::ptr::eq(result.class(), argument.class()),
                "the answer is the argument's own shape class"
            );
            assert_eq!(
                result.field(0).as_int(),
                Some(7),
                "`size` is its closure's answer"
            );
            assert_eq!(
                result.field(1).as_int(),
                Some(11),
                "`total` is its closure's answer"
            );
        }
        assert_eq!(refcounts(), [1, 1], "the call kept no closure");

        #[expect(
            unsafe_code,
            reason = "the test owns the answer and the shape, and the shape owns both closures"
        )]
        unsafe {
            answer.release();
            shape.release();
        }
    }

    /// A one-parameter callback: sweeps its receiver and its entry, and
    /// answers ten times the entry.
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes the receiver and one argument, each \
                  retained for this callee to release, and the address of a \
                  live `Value` for the result"
    )]
    unsafe extern "C" fn times_ten(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        let entry = unsafe { *args.add(1) }
            .as_int()
            .expect("this test's entries are ints");
        for index in 0..2 {
            unsafe { (*args.add(index)).release() };
        }
        unsafe {
            *out = Value::int(entry * 10);
        }
        nvs_runtime::OK
    }

    /// A two-parameter callback: sweeps its receiver and both arguments, and
    /// answers the key it was shown, a `=` and the entry, as one text.
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes the receiver and two arguments, each \
                  retained for this callee to release, and the address of a \
                  live `Value` for the result"
    )]
    unsafe extern "C" fn names_the_key(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        let entry = unsafe { *args.add(1) }
            .as_int()
            .expect("this test's entries are ints");
        let mut named = unsafe { *args.add(2) }
            .as_str_bytes()
            .expect("the key arrives as a text")
            .to_vec();
        named.extend_from_slice(format!("={entry}").as_bytes());
        for index in 0..3 {
            unsafe { (*args.add(index)).release() };
        }
        unsafe {
            *out = Value::str(NvsStr::new(&named));
        }
        nvs_runtime::OK
    }

    /// Every entry of the array `answer` as `key => value`, in the array's own
    /// order, releasing the one reference the member answered with.
    fn entries_of(answer: Value) -> Vec<String> {
        #[expect(
            unsafe_code,
            reason = "the handle takes over the one reference the member \
                      answered with, and releases it on drop"
        )]
        let array =
            unsafe { NvsArray::from_raw(answer.array_ptr().expect("map answers an array")) };
        let mut entries = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let key = array.key_at(slot).expect("a live entry has a key");
            let value = array.value_at(slot).expect("a live entry has a value");
            let value = match value.as_int() {
                Some(number) => number.to_string(),
                None => String::from_utf8_lossy(value.as_str_bytes().expect("an int or a text"))
                    .into_owned(),
            };
            entries.push(format!(
                "{} => {value}",
                String::from_utf8_lossy(key.as_bytes())
            ));
            from = slot + 1;
        }
        entries
    }

    /// `map` answers each callback's result under the subject's own key and in
    /// the subject's own order, shows the key, as a text, only to a callback
    /// that declares a second parameter, and answers an empty array for an
    /// empty subject. It borrows the callback rather than keeping it, and a
    /// `limit` of `0` is refused before any host is asked.
    // covers: Core\Task::map
    #[test]
    fn map_keeps_the_subjects_keys_and_order_and_shows_the_key_only_when_asked() {
        let mut subject = NvsArray::new();
        for (key, value) in [(&b"b"[..], 2), (&b"a"[..], 1), (&b"10"[..], 5)] {
            subject.set(NvsStr::new(key), Value::int(value));
        }
        let subject = Value::array(subject);
        let empty = Value::array(NvsArray::new());
        let by_value = callable_of(1, times_ten);
        let by_key = callable_of(2, names_the_key);
        let refcounts = || {
            #[expect(
                unsafe_code,
                reason = "the test holds both closures until its last line"
            )]
            unsafe {
                [
                    NvsObj::refcount_of(by_value.obj_ptr().expect("a closure is an object")),
                    NvsObj::refcount_of(by_key.obj_ptr().expect("a closure is an object")),
                ]
            }
        };

        let _installed = nvs_runtime::host::install(&SERIAL);
        let mut ctx = Ctx::buffered();
        let zero = nvs_runtime::call(
            super::nvs_core_task_map,
            &mut ctx,
            &[subject, by_value, Value::uint(0), Value::null()],
        );
        assert!(zero.is_err(), "a limit of zero is refused");
        let message = ctx.take_pending().expect("the refusal has a sentence");
        assert!(
            message.contains("Core\\Task::map: `limit` is how many children may run at once"),
            "the refusal names the member and says what `limit` means, got {message:?}"
        );
        assert_eq!(GROUPS.with(Cell::get), 0, "no group was started");

        let mut map = |items: Value, callback: Value| {
            entries_of(
                nvs_runtime::call(
                    super::nvs_core_task_map,
                    &mut ctx,
                    &[items, callback, Value::null(), Value::null()],
                )
                .expect("a group whose children all return completes"),
            )
        };
        assert_eq!(map(subject, by_value), ["b => 20", "a => 10", "10 => 50"]);
        assert_eq!(map(subject, by_key), ["b => b=2", "a => a=1", "10 => 10=5"]);
        assert!(
            map(empty, by_value).is_empty(),
            "an empty subject answers `[]`"
        );
        assert_eq!(
            GROUPS.with(Cell::get),
            3,
            "one call is one group, an empty one included"
        );
        assert_eq!(refcounts(), [1, 1], "the call kept no callback");

        #[expect(
            unsafe_code,
            reason = "the test built all four values and owns their last references"
        )]
        unsafe {
            subject.release();
            empty.release();
            by_value.release();
            by_key.release();
        }
    }
}
