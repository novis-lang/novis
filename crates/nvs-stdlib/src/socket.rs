//! `Core\Socket` — [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md)
//! §§ 1-2's WebSocket upgrade, as the signature a program writes.
//!
//! § 1 makes a connection a **root isolate** rather than a suspended request:
//! its own arena, its own `[limits]` budget, its own grants, and none of the
//! upgrading request's heap. § 2 makes opening one `spawn script`-shaped, so
//! the operand here is that construct's operand under exactly
//! [ADR 0006](/docs/adr/0006-isolated-script-execution.md)'s rule — a path, or
//! a static method written `Chat::run(...)`, and never a closure.
//!
//! # What is here, and what is not
//!
//! The row, its signature, and **the body that fills § 1's slot**: a path
//! entry resolved into a `nvs_runtime::script::Program`, `args:` crossed by ADR
//! 0023 § 2's graph copy, and the pair left on the request's carrier for the
//! connection to start. Behind it the framing has landed — `nvs_server::serve`
//! answers RFC 6455's `101` and `nvs_server::socket` hands the framed socket to
//! the isolate as a [`nvs_runtime::PeerSocket`] — so an upgrade prepared here
//! opens a root isolate that **has** a peer.
//!
//! What is missing is the surface over it: § 3's
//! `current()`/`receive()`/`send()` are unwritten, so a connection isolate can
//! reach its socket from Rust and not yet from Novis, and `Core\Topic` (§ 4) is
//! unregistered because `receive()` is the one wait a topic delivery arrives
//! through. Those two are this module's known gap. `nvs_server::socket`'s own
//! docs § *What is not here yet* carry the other half of it — § 1's `[limits]
//! idle` and § 3's send timeout — which is the same list read from under the
//! seam.
//!
//! `Core\Sse` (§ 5) is [`crate::sse`], and it is a module of its own rather
//! than a second class here because what § 5 splits is the **door** and never
//! the isolate: an SSE connection takes no socket, so it is offered a second
//! cell on the carrier — [`nvs_runtime::SseSlot`], one every request the server
//! runs gets — rather than [`nvs_runtime::UpgradeSlot`], which exists to carry
//! a socket hand-over the server framed. Everything *behind* the door is one
//! thing and is shared outright: [`entry_program`] resolves either member's
//! operand, and [`retained`] and [`release_crossed`] are the two halves of
//! either one's crossing. Which is why the sections below say `this member`
//! where they mean both, and are the home of the reasoning for both.
//!
//! There is no [`crate::registry::CAPABILITIES`] row, and that is a statement
//! about where the grant is asked rather than about the member: the entry path
//! reaches the operating system through `nvs_runtime::script::resolve`, which
//! is ADR 0118 § 2's own door for `script.spawn` and asks the question with the
//! path as its scope, and the sibling construct's helper — `crate::script`'s
//! `nvs_core_script_spawn` — declares none for that same reason. Nothing here
//! names an operating-system spelling of its own, which is what
//! `nvs_stdlib_reaches_the_os_only_through_the_gate` holds mechanically.
//!
//! The `entry` parameter is a [`CoreTy::Entry`](crate::registry::CoreTy::Entry)
//! and that is the whole of how § 2's operand rule reaches a call site: the
//! mark is what `nvs_types::expr::isolate` finds, so `Chat::run(...)` is
//! accepted here and a `callable` in a variable is refused with the same
//! `E0802` a `spawn script` reports. Its variant doc is the home of why the
//! rule cannot be a parameter type.
//!
//! # Decision: this member spawns nothing, and the connection starts it
//!
//! § 1's root isolate is not started here, and it cannot be. A `Core` member
//! runs on the request isolate's own task holding the request's
//! `nvs_runtime::Ctx`, so every route out of this body reaches
//! `nvs_runtime::host::Host::start_isolate` with *that* context — which is
//! `Ctx::isolate`'s tree: the child's memory is charged to the request, its
//! deadline is the same word the request's `wall_time` expires, and it is a
//! task under the request's, so the request cannot return while it runs
//! ([ADR 0072](/docs/adr/0072-core-task-structured-concurrency.md) § 4). Each
//! of those is the opposite of what § 1 states, and none of them is a builder
//! away.
//!
//! So the member **prepares** an isolate and records it; the **connection**
//! starts it. What is prepared is what `nvs_host::Isolate::new` takes — a
//! `nvs_runtime::script::Program` and the argument value that has already
//! crossed — and it is prepared here, inside the request, because all three
//! things that can refuse belong to the request:
//!
//! - **The capability.** ADR 0006's `script.spawn` grant, and the root check
//!   under it, are asked against the request's own configuration overlay, so a
//!   request that narrowed its grants cannot upgrade into a connection holding
//!   the ones it gave up — § 1's "narrowed from the request's, never widened".
//! - **The argument.** ADR 0023 § 2's refusal is the *parent's* fault, and
//!   `nvs_host`'s `isolate` module doc owns that asymmetry; here is the one
//!   point at which a `secret` passed to a socket is still a throw the program
//!   can catch rather than a connection that closes after its `101`.
//! - **The code.** A path is resolved through `nvs_runtime::script`; a static
//!   method is code the request's *own* unit already holds, and the request's
//!   context is the only place that unit's statics recipes and class table can
//!   be taken from.
//!
//! Both of § 2's entry forms therefore collapse to one prepared `Program`, and
//! that is what makes them one isolate at the far end rather than two shapes to
//! keep in step. A path is the resolver's program unchanged, and that is the
//! form the body below prepares.
//!
//! # The method form carries its names on the value
//!
//! A method arrives as a **first-class callable value** — the entry interns as
//! `mixed`, so `Chat::run(...)` reaches this body as the closure `nvs_ir`'s
//! `lower_callable_ref` built. ADR 0006 § *Decision* binds `args:` to the
//! entry's parameters **by name**, and the sibling construct has those names as
//! a constant its lowering wrote into the call (`nvs_ir::lower`'s
//! `spawn_method_entry`, read by `crate::script`'s `entry_names_agree` and
//! `bound_arguments`). A `Core` call has no such constant, because its entry is
//! one ordinary argument — so the names ride on the value instead, in a third
//! reserved field beside the arity and the parameter tags:
//! `nvs_ir::lower`'s `FN_PARAM_NAMES` writes it and
//! [`nvs_runtime::closure_param_names`] reads it back.
//!
//! Binding by *position* was the alternative and is the wrong one: `{room: …,
//! userId: …}` binding correctly because the program happened to write the map
//! in declaration order is by-name spelling over by-position meaning, and it
//! fails silently the first time somebody reorders a literal. The field is the
//! general fix rather than this member's, and serves `Core\Sse::upgrade` and
//! every later `CoreTy::Entry` row the same way.
//!
//! [`method_program`] is what it buys, and how that form differs from a path's:
//! its program is a closure over the retained callable, beside the statics
//! recipes and the class table the request's context is holding, and it arms
//! the child itself exactly as a path entry's `install_in` does.
//! `nvs_host::Isolate`'s own method entry is *not* what a connection uses, and
//! that is the same fact from the other end: that builder re-materializes the
//! recipes off the **spawning** context, which for a connection is a context
//! that never ran the unit.
//!
//! **The preparation rides on `nvs_runtime::Inbound`**, the request carrier,
//! and that answers two questions at once. Both halves of that are on disk —
//! `nvs_runtime::Upgrade` is the prepared pair and `nvs_runtime::UpgradeSlot`
//! the cell it is left in, offered by `nvs_server::serve_connection` to a
//! request `hyper` framed an upgrade for and to no other — and the body below
//! is the filling: it resolves the entry into one `Program`, crosses `args`,
//! and leaves the pair in the slot. A connection is the only thing an
//! upgrade can happen to, so a request that did not arrive on one — a CLI
//! program, a `spawn script` child, a request the server could offer no
//! upgrade for — has no slot to write into and this member throws, for the
//! reason `Core\Request::method()` throws there
//! ([ADR 0012](/docs/adr/0012-no-superglobals.md) § 7). And the server holds
//! the other half of that slot, so § 1's ordering is what the code can express
//! rather than what it must remember: the request is joined, its context is
//! dropped and its arena with it, and only then is there a caller left holding
//! the program.
//!
//! What the connection starts it over is its **own** context — the one
//! `nvs-server`'s `serve` module already starts the request isolate from. The
//! connection isolate is that request's *sibling* rather than its child, which
//! is where its own budget, its own deadline and its own `spawn script` depth
//! come from.
//!
//! **What it spends:** the argument graph is copied twice per upgrade — once
//! here, and once by the spawn at the far end — because it crosses two
//! boundaries and the refusal has to land on this side of the first one. That
//! is one extra copy of a value an application chose to hand a connection, once
//! per connection opened, and it buys a throw the program can still catch. The
//! first copy is allocated before the connection's context takes its zero point
//! and released after, so that context reads its own share a little low — the
//! balance is a signed `Ctx::memory_base` for exactly this reason, and the
//! error is bounded by the size of one `args` graph.
//!
//! Two alternatives were refused. **The request isolate becoming the
//! connection** keeps the arena § 1 says is released and hands the connection
//! the request's session, headers and statics. **Riding out on the isolate's
//! completion** instead of on the carrier makes a value boundary carry a
//! connection, and gives a `spawn script` child a completion its parent would
//! then have to forward.
//!
//! # Why the member answers `void`
//!
//! Because calling it performs the upgrade — ADR 0083 § 2's own bullet, which
//! is the home of the reasoning and of what the alternative would cost. The
//! short of it: nothing in this language reads a handler's return, so
//! [`crate::response`] is a class of `void` members writing the response the
//! request already has, and this is one more of them.
//!
//! # Why `limits:`, `grants:` and `on:` are not parameters
//!
//! § 2 lists four options and calls them "0006's, spelled as ordinary named
//! arguments". Three of the four are **refused** at the sibling site:
//! `nvs_types::expr::isolate` reports `E_SPAWN_OPTION_UNSUPPORTED` for
//! `spawn script`'s `limits:`, `grants:` and `on:`, because a `grants:`
//! narrowing that were silently dropped would hand the child the parent's
//! authority. Declaring them here would be that same drop with a different
//! spelling — the row would accept them and the body would ignore them — so
//! they are simply not declared, and a program writing `grants: {}` gets the
//! unknown-argument refusal rather than a silent widening. They arrive here
//! when they are enforced there; that refusal is the rule's one home and this
//! module does not restate it.
//!
//! `output:` is the fifth and does not apply: an isolate's `output:` chooses
//! between its own buffer and the parent's stream
//! ([ADR 0088](/docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
//! § 3), and a connection's output is the socket.

use nvs_runtime::script::{Program, ResolveError};
use nvs_runtime::{Ctx, Fault, ThrownClass, Upgrade, Value, copy_graph};

use crate::registry::{Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc};

/// `Core\Socket`'s registry rows — ADR 0083 § 2's `upgrade`, and so far
/// nothing else. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Socket",
    methods: &[CoreMethod {
        name: "upgrade",
        names: &["entry", "args"],
        // [`CoreTy::Entry`] is § 2's "0006's operand", as the one mark that
        // carries that ADR's whole rule to a call site: a path or a static
        // method written `Chat::run(...)`, and never a `callable` in a
        // variable. It classifies as a sink for the reason a path always does
        // — its content becomes the instruction "execute this file", ADR 0088
        // § 1's definition, and
        // [ADR 0097](/docs/adr/0097-development-server-and-proxied-origin.md)
        // § 2 is the same rule written for the server, a filesystem path never
        // derived from a URL at request time. `args` takes no expected type at
        // all, for the reason the sibling site takes none: ADR 0023 § 2's walk
        // decides what may cross, and that is a run-time question for
        // everything a declared type does not already settle.
        params: &[CoreTy::Entry, CoreTy::Mixed],
        defaults: &[Const::Null],
        return_ty: CoreTy::Void,
        symbol: UPGRADE_SYMBOL,
        doc: Some(&UPGRADE_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The symbol [`CLASS`]'s `upgrade` row is reached through.
const UPGRADE_SYMBOL: &str = "nvs_core_socket_upgrade";

/// `Core\Socket::upgrade`'s reference card — ADR 0117.
const UPGRADE_DOC: MethodDoc = MethodDoc {
    short: "Turns this request into a WebSocket connection running `$entry` as a root isolate — \
            its own arena, its own budget and its own grants, sharing nothing with the request \
            that opened it but the values `$args` copied in.",
    params: &[
        ParamDoc {
            name: "entry",
            desc: "What the connection runs: a file path, resolved and root-checked exactly as \
                   `spawn script`'s operand is, or a static method written `Chat::run(...)`. \
                   Never a closure — an isolate shares nothing but compiled code, so a capture \
                   would cross the boundary the isolate exists to be.",
            shape: &[],
        },
        ParamDoc {
            name: "args",
            desc: "The values the connection starts with, bound to the entry's parameters by \
                   name. They cross by the graph copy an isolate boundary already uses, so what \
                   arrives is a value and never a shared reference; a `secret` may not cross and \
                   a `tainted` value stays `tainted` on the other side.",
            shape: &[],
        },
    ],
    ret: "Nothing. Calling it performs the upgrade — this is not a response value a handler \
          hands back, because nothing interprets a handler's return.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "A request that arrived on no connection a server could upgrade; an `$entry` \
                   path `script.spawn` does not grant or that does not compile; a second call on \
                   one request.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "An `$args` value with no meaning on the other side of an isolate boundary — a \
                   resource, or a `secret` the call site could not see through; and, for a static \
                   method entry, an `$args` map that omits a parameter the method declares or \
                   names one it does not.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        UPGRADE_SYMBOL => (nvs_core_socket_upgrade as *const ()).cast(),
        _ => return None,
    })
}

/// ADR 0083 § 2's entry, resolved into the one thing a connection runs.
///
/// The two written forms are told apart by the value's own tag and by nothing
/// else, which is what the module doc's "the entry interns as `mixed`" costs
/// and buys: a path arrives as text, and a `Class::method(...)` reference
/// arrives as the closure `nvs_ir`'s `lower_callable_ref` built. Anything else
/// is unreachable from source — `nvs_types::expr::isolate`'s `entry_operand`
/// refuses every other spelling with `E0802` where it is written — so a third
/// tag here is a compiler bug and not a program's.
///
/// `member` is the caller's own spelling — `Core\Socket::upgrade` or
/// `Core\Sse::upgrade` — because a refusal names the member a program wrote and
/// the two doors share this body.
///
/// `args` is the **parent's** `args:` map, judged here rather than in the
/// child for `crate::script`'s `entry_names_agree` reason: this frame is the
/// call ADR 0006 names as where a named-argument mismatch is reported, and it
/// is the last one that still has a `catch` above it. It is *only* judged —
/// what crosses is the caller's own copy, made after this returns.
///
/// # Errors
///
/// Everything [`nvs_runtime::script::resolve`] refuses, in `spawn script`'s own
/// wording with `member` in front of it; ADR 0006's named-argument mismatch for
/// a method entry; and a `callable` recording no parameter names, which the
/// checker refuses at every spelling that could produce one.
pub(crate) fn entry_program(
    ctx: &Ctx,
    entry: Value,
    args: Value,
    member: &str,
) -> Result<Program, Fault> {
    // No case can reach this, and no case can reach any refusal below it: every
    // one of them stands *after* the slot, and a `.nvst` case runs a script
    // nothing offered a connection to, so the missing slot is the only report
    // that ever arrives from a corpus run. The `#[test]`s that assert them
    // instead are `an_upgrade_by_static_method_is_the_same_isolate_as_an_upgrade_by_path`
    // and `a_method_entry_whose_args_do_not_name_its_parameters_is_refused` for
    // the method form, and `an_entry_a_resolver_refuses_is_a_throw_the_program_catches`
    // for the two below.
    let Some(path) = entry.as_text() else {
        return method_program(ctx, entry, args, member);
    };
    // ADR 0118 § 2's door is inside `resolve` and not here, exactly as it is
    // for the sibling construct: a `Program` is what a spawn was after, so the
    // function that produces one is the effect the grant guards.
    // No case can reach this — see above — and
    // `an_entry_a_resolver_refuses_is_a_throw_the_program_catches` is the
    // `#[test]` that asserts it instead.
    nvs_runtime::script::resolve(ctx, path).map_err(|error| match error {
        // An embedder that installed none — `nvs_core_script_spawn`'s reading
        // unchanged, and not something a `catch` should paper over.
        ResolveError::NoResolver => Fault::fatal(format!(
            "`{member}('{path}')` needs a script resolver on this thread and there is none"
        )),
        // No case can reach this — see above — and
        // `an_entry_a_resolver_refuses_is_a_throw_the_program_catches` is the
        // `#[test]` that asserts it instead.
        ResolveError::Refused(message) => Fault::thrown_as(
            ThrownClass::Runtime,
            format!("`{member}('{path}')`: {message}"),
        ),
        // Already a whole sentence naming the capability and the path (ADR 0118
        // § 5), so it is thrown as written rather than framed twice.
        ResolveError::Denied(message) => Fault::thrown_as(ThrownClass::Runtime, message),
    })
}

/// The callable a method entry's program runs, owning the one reference that
/// keeps it alive from the request that prepared the upgrade to the connection
/// that starts it.
///
/// A [`Program`] is a boxed closure, and a [`Value`] captured in one is sixteen
/// plain bytes: dropping the box would drop the handle and release nothing. So
/// the reference [`retained`] took is held *by a type with a destructor*, which
/// is what makes both endings correct with no rule to remember — the connection
/// runs the program and the drop at the end of the call releases it, or
/// [`nvs_runtime::Upgrade::discard`] drops the program unrun and the same drop
/// releases it. The second ending is the reachable one: a request that filled
/// both cells is refused with neither isolate started
/// (`nvs_server::serve_connection` owns that reading), and a leak there would
/// be one object per refused request.
struct HeldCallable(Value);

impl Drop for HeldCallable {
    #[expect(
        unsafe_code,
        reason = "this type exists to own exactly the reference `retained` \
                  took, and nothing else points at it by the time it drops"
    )]
    fn drop(&mut self) {
        // SAFETY: the one reference `retained` answered with, held by this
        // value for its whole life and released exactly once here.
        unsafe { self.0.release() };
    }
}

/// ADR 0083 § 2's **method entry**, prepared: the callable the request is
/// holding, the names its target declares, and the recipes that arm the child.
///
/// The connection's context is not this one's child — § 1 makes it a *root*
/// isolate started from the connection's own context, which never ran the unit
/// — so `nvs_runtime::Ctx::method_isolate`, which re-materializes off the
/// spawning context, has nothing to work from at the far end. What replaces it
/// is this: the recipes and the error-class table are taken **here**, where the
/// unit is in hand, and the program arms the child with them before it calls,
/// which is exactly what a path entry's `install_in` does from inside the
/// resolver's program. `nvs_runtime::Ctx::unit_statics` is the one home of why
/// that handle exists.
///
/// The child is armed with a **fresh** store materialized from those recipes,
/// never the parent's slots, so ADR 0006's unshared class statics hold whichever
/// form named the entry — the same rule `method_isolate` keeps for a `spawn
/// script`.
///
/// **What it spends:** one retained reference to a thunk object for the length
/// of the connection's preparation, plus one materialized slot per static
/// property the unit declares, released with the connection. The thunk is the
/// object `lower_callable_ref` built and holds no capture — § 2's entry rule
/// admits only a *static* method, so there is no receiver field and nothing of
/// the request's heap rides across inside it.
///
/// # Errors
///
/// A `callable` recording no parameter names at all, and ADR 0006's
/// named-argument mismatch between the target's parameters and `args`.
fn method_program(ctx: &Ctx, entry: Value, args: Value, member: &str) -> Result<Program, Fault> {
    // `None` is a closure with no `fn#names` field, which is an `fn` literal:
    // `nvs_types::expr::isolate` refuses one where an entry is expected, so
    // this is a compiler disagreement rather than a program's — but it is
    // reported as a throw and not a fatal, because a wrong refusal is
    // recoverable and a wrong `FATAL` ends the request.
    let Some(names) = nvs_runtime::closure_param_names(entry)? else {
        return Err(Fault::thrown(format!(
            "`{member}` was handed a `callable` that records no parameter names, so ADR 0006's \
             `args:` binding has nothing to bind by. Name a static method — `Chat::run(...)` — \
             or the file the connection runs"
        )));
    };
    if let Err(message) = crate::script::entry_names_agree(member, &names, args) {
        return Err(Fault::thrown_as(ThrownClass::Logic, message));
    }
    let held = HeldCallable(retained(entry));
    let statics = ctx.unit_statics();
    let errors = ctx.class_table();
    Ok(Box::new(move |child: &mut Ctx, argument: Value| {
        // Before the call and before the binding, in the order a path entry's
        // `install_in` runs in: a `static` the entry touches is read out of the
        // store this arms, and a throw raised by the call needs the tree to
        // build an object from.
        if let Some(defaults) = statics {
            child.install_statics(defaults);
        }
        if let Some(class) = errors {
            child.set_runtime_error_class(class);
        }
        // Ownership discharged into the isolate's own root, exactly as a path
        // entry's program does it, and **before** the binding — which is what
        // makes every value that binding reads live for the length of the call:
        // the root owns the map, and the map owns them.
        child.set_isolate_argument(argument);
        let bound = crate::script::bound_arguments(&names, child.isolate_argument());
        match nvs_runtime::call_closure(child, held.0, &bound) {
            Ok(value) => value,
            // The judgement `call_closure` makes on this frame's behalf — an
            // argument whose tag the parameter does not admit, ADR 0006's
            // "typed at the boundary". There is no frame above it inside the
            // connection, so it is recorded as the isolate's pending throw.
            Err(Fault::Thrown(class, message)) => {
                child.set_pending_as(class, message);
                Value::null()
            }
            // A thunk that is not callable at the arity it recorded: an engine
            // fault at the far end, and the connection is the unit a fatal
            // tears down (§ 3's escalation). Recorded rather than panicked,
            // because a connection may not end the process.
            Err(Fault::Fatal(message)) => {
                child.set_pending(message);
                Value::null()
            }
            // The remaining `Err` is `Fault::Pending`, which means the throw is
            // already on this context — where `nvs_host::Isolate`'s `finish`
            // reads it from.
            Err(_) => Value::null(),
        }
    }))
}

/// One more reference to `value`, for the crossing below to consume.
///
/// [`copy_graph`] takes ownership of what it is handed, and a `Core` helper
/// *borrows* its arguments — so the retain is what reconciles the two
/// conventions, and it is also what keeps the copy a copy: a value at refcount
/// one is moved rather than walked, which would hand the connection the
/// request's own graph.
#[expect(
    unsafe_code,
    reason = "the argument is live for this frame, being one the caller is \
              holding a reference to for the length of the call"
)]
pub(crate) fn retained(value: Value) -> Value {
    // SAFETY: `value` is an argument slot of a running helper frame, so the
    // caller's own reference is what keeps the payload alive across this call.
    unsafe { value.retain() };
    value
}

/// Releases the crossed argument on the one path that has nowhere to put it.
#[expect(
    unsafe_code,
    reason = "the reference released is the one `copy_graph` answered with and \
              handed to this frame, which no slot took"
)]
pub(crate) fn release_crossed(value: Value) {
    // SAFETY: this frame owns the reference `copy_graph` returned and the slot
    // refused, and nothing else points at it.
    unsafe { value.release() };
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket::upgrade(string $entry, mixed $args = null): void` — ADR
    /// 0083 § 2's upgrade, prepared here and started by the connection.
    ///
    /// Three refusals in the order they cost the least, and each is a fact
    /// about a different thing. **No slot** is a request no connection offered
    /// one for, and it is the whole of how this member refuses a CLI program, a
    /// `spawn script` child and an ordinary HTTP request alike — the module doc
    /// owns why that question is asked of `nvs_runtime::Inbound` rather than of
    /// a header. **The entry** is the code, and [`entry_program`] is where its
    /// two forms become one. **The argument** is ADR 0023 § 2's crossing, and
    /// it is made on this side of the boundary so that a `secret` handed to a
    /// connection is still a throw the program catches.
    ///
    /// A second call is refused by [`nvs_runtime::UpgradeSlot::fill`] rather
    /// than by a question asked before the work: that method hands the upgrade
    /// back precisely so the refusal lands where there is still a context to
    /// release the argument with, and asking twice would put the rule in two
    /// places to save a program that is already wrong one resolve.
    fn nvs_core_socket_upgrade(ctx, args: [2]) {
        // Cloned rather than borrowed, because `entry_program` below takes the
        // context: both halves of the cell are shared handles by construction,
        // so a clone is one refcount and no borrow held across a call.
        let slot = ctx
            .inbound()
            .and_then(nvs_runtime::Inbound::upgrade_slot)
            .cloned()
            .ok_or_else(|| {
                Fault::thrown(
                    "`Core\\Socket::upgrade` needs a connection to upgrade and this request \
                     arrived on none: only a request a server framed an upgrade for is offered \
                     one",
                )
            })?;
        let program = entry_program(ctx, args[0], args[1], "Core\\Socket::upgrade")?;
        // No case can reach this: it stands after the slot, and a `.nvst` case
        // runs a script no connection offered one to.
        // `an_args_value_that_cannot_cross_is_refused_before_the_slot_is_filled`
        // is the `#[test]` that asserts it instead.
        let crossed = copy_graph(retained(args[1])).map_err(|refused| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("`Core\\Socket::upgrade`'s `args:` cannot cross into a connection: {refused}"),
            )
        })?;
        if let Err(returned) = slot.fill(Upgrade::new(program, crossed)) {
            let (_program, crossed) = returned.into_parts();
            release_crossed(crossed);
            // No case can reach this: a second upgrade needs a first, and a
            // first needs a slot no `.nvst` case is offered.
            // `a_second_upgrade_on_one_request_is_refused_and_the_first_still_stands`
            // is the `#[test]` that asserts it instead.
            return Err(Fault::thrown(
                "`Core\\Socket::upgrade` was called twice on one request, and a request opens \
                 at most one connection",
            ));
        }
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::script::{Installed, Program, Resolver, install};
    use nvs_runtime::{Ctx, Inbound, NvsArray, NvsStr, UpgradeSlot, Value};

    use super::{nvs_core_socket_upgrade, release_crossed};

    /// A resolver answering with the length of the path it was asked for.
    ///
    /// `nvs_runtime::script`'s own tests use this shape for the reason it is
    /// borrowed here: there is no compiler on this side of the seam, so the
    /// only thing a program has to prove is that it is *the one the resolver
    /// answered for the path that was written*, and a length says that with
    /// nothing installed.
    #[derive(Debug)]
    struct Fixed;

    impl Resolver for Fixed {
        fn resolve(&self, path: &str) -> Result<Program, String> {
            let len = i64::try_from(path.len()).unwrap_or(-1);
            Ok(Box::new(move |ctx, args| {
                ctx.set_isolate_argument(args);
                Value::int(len)
            }))
        }
    }

    /// One that answers nothing, so the seam's two ways of failing — a refusal
    /// and no resolver at all — are both reachable from here.
    #[derive(Debug)]
    struct Refusing;

    impl Resolver for Refusing {
        fn resolve(&self, path: &str) -> Result<Program, String> {
            Err(format!("`{path}` is not a script"))
        }
    }

    // `install` takes a `&'static dyn Resolver`, so a `static` is the only way
    // to reach it directly.
    static FIXED: Fixed = Fixed;
    static REFUSING: Refusing = Refusing;

    fn resolving() -> Installed {
        install(&FIXED)
    }

    fn refusing() -> Installed {
        install(&REFUSING)
    }

    /// A context granting `script.spawn` for everything, because ADR 0118 § 2's
    /// door is inside `resolve` and a bare context grants nothing — every case
    /// below is about the slot rather than about the grant.
    fn granting() -> Ctx {
        let mut snapshot = nvs_config::Snapshot::default();
        snapshot.config.capabilities = Some(nvs_config::tree::Capabilities {
            script: Some(nvs_config::tree::CapScript {
                spawn: Some(nvs_config::tree::Setting::Bool(true)),
            }),
            ..nvs_config::tree::Capabilities::default()
        });
        let mut ctx = Ctx::buffered();
        ctx.set_config(std::sync::Arc::new(snapshot));
        ctx
    }

    /// A carrier for a request a connection offered `slot` for.
    fn upgradable(ctx: &mut Ctx, slot: &UpgradeSlot) {
        let mut inbound = Inbound::new("GET", "/live/chat", "");
        inbound.offer_upgrade(slot.clone());
        ctx.set_inbound(inbound);
    }

    /// `{room: "lobby"}`, as the one refcounted argument a crossing can be
    /// observed on: an `int` would cross as itself and prove nothing.
    fn a_room() -> Value {
        let mut map = NvsArray::new();
        map.set(NvsStr::new(b"room"), Value::int(7));
        Value::array(map)
    }

    /// The object `nvs_ir::lower`'s `lower_callable_ref` builds for a written
    /// `Chat::run(...)`: the two reserved fields every closure carries, plus
    /// the third only a first-class callable has — ADR 0006's parameter names,
    /// comma-joined in declaration order.
    ///
    /// Built by hand rather than compiled, because there is no compiler on this
    /// side of the seam and the *writing* side is asserted where it is written,
    /// by `nvs-ir`'s `a_first_class_callable_records_its_targets_parameter_names`.
    /// What is pinned here is what this member does with one.
    ///
    /// The class is leaked, exactly as the playbook's `closure_of` leaks its
    /// table: the descriptor has to outlive the object, which is `NvsObj::new`'s
    /// whole obligation.
    fn a_callable(names: &str, invoke: nvs_runtime::NvsFn) -> Value {
        let declared: Vec<&str> = names.split(',').filter(|n| !n.is_empty()).collect();
        let mut table = nvs_runtime::ClassTable::new();
        let id = table.define(
            "Chat$fcc0",
            &["fn#arity", "fn#params", nvs_runtime::CLOSURE_PARAM_NAMES],
            &[],
        );
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
                code: invoke as *const u8,
                // `call_closure` reads the arity and the tags off the object's
                // own slots below rather than off this row — see
                // `nvs_runtime::MethodRow`.
                arity: 0,
                param_tags: 0,
                public: true,
                native: false,
            }],
        );
        let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
        object.set_field(
            nvs_runtime::CLOSURE_ARITY_SLOT,
            Value::int(i64::try_from(declared.len()).expect("a small arity")),
        );
        let mut tags: u64 = 0;
        for parameter in 0..declared.len() {
            tags |= u64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY) << (parameter * 4);
        }
        object.set_field(
            nvs_runtime::CLOSURE_PARAM_TAGS_SLOT,
            Value::int(i64::from_ne_bytes(tags.to_ne_bytes())),
        );
        object.set_field(
            nvs_runtime::CLOSURE_PARAM_NAMES_SLOT,
            Value::str(NvsStr::new(names.as_bytes())),
        );
        Value::object(object)
    }

    /// A one-parameter entry answering with the `int` it was passed, after the
    /// exit sweep a compiled callee performs.
    ///
    /// The sweep is not decoration: `call_closure` retains the receiver and
    /// every argument on the way in *because* the callee releases them, so a
    /// fixture that skipped it would leak one reference per call and the
    /// valgrind leg would report it against this member rather than against the
    /// test.
    ///
    /// # Safety
    ///
    /// The callee half of `nvs_runtime::NvsFn`'s contract, which `call_closure`
    /// satisfies: `args` points at the receiver plus one live retained value,
    /// and `out` is writable.
    #[expect(
        unsafe_code,
        reason = "this is a compiled callee's own contract, discharged where a \
                  compiled callee would discharge it"
    )]
    unsafe extern "C" fn echoes_room(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        // SAFETY: the receiver and one argument, both live for this call.
        let room = unsafe { *args.add(1) }.as_int().unwrap_or(-1);
        for slot in 0..2 {
            // SAFETY: each is a reference `call_closure` retained for this
            // callee to release, which is what a compiled exit sweep does.
            unsafe { (*args.add(slot)).release() };
        }
        // SAFETY: the caller's `out` is one writable `Value`.
        unsafe { *out = Value::int(room) };
        nvs_runtime::OK
    }

    /// ADR 0083 § 1 gives the connection the slot and § 2's member fills it, so
    /// what this asserts is the hand-over itself: after the call the slot holds
    /// the resolver's program for the path that was *written*, and the argument
    /// beside it is a **copy** rather than the request's own graph.
    ///
    /// The copy is the half that could go wrong silently. ADR 0023 § 2's
    /// crossing is what makes an isolate share nothing, and a member that left
    /// the caller's array in the slot would pass every test that only looked at
    /// the values in it — so the assertion is on the allocation, which is the
    /// one thing a shared graph and a copied one disagree about.
    #[test]
    fn an_upgrade_leaves_the_resolvers_program_and_a_copy_of_its_argument_in_the_slot() {
        let _resolver = resolving();
        let mut ctx = granting();
        let slot = UpgradeSlot::new();
        upgradable(&mut ctx, &slot);

        let path = Value::str(NvsStr::new(b"sockets/chat.nvs"));
        let mine = a_room();
        nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[path, mine])
            .expect("an offered slot takes the upgrade");

        let (program, crossed) = slot
            .take()
            .expect("the member filled the slot the connection left")
            .into_parts();
        assert_ne!(
            crossed.array_ptr(),
            mine.array_ptr(),
            "the connection was handed the request's own array rather than a copy of it"
        );

        // Run it the way the connection does — on a context of its own, with
        // the crossed argument transferred into it — so the program is proved
        // to be the resolver's answer for the written path and the crossed
        // reference lands in an ownership root that will release it.
        let mut connection = Ctx::buffered();
        assert_eq!(
            program(&mut connection, crossed).as_int(),
            Some(16),
            "the slot holds a program for some other path"
        );
        release_crossed(mine);
    }

    /// The refusal that makes this member callable only where ADR 0083 § 1's
    /// ordering can hold: a request no connection offered a slot for has
    /// nowhere to leave an isolate, so it is told so rather than answered as
    /// though a peer were attached.
    ///
    /// The same throw is what a CLI program and a `spawn script` child get, and
    /// for the same reason — none of the three is a connection — which is why
    /// the case is written over a carrier that exists and simply carries no
    /// slot rather than over a context with no carrier at all.
    #[test]
    fn an_upgrade_on_a_request_no_connection_offered_a_slot_for_is_refused() {
        let _resolver = resolving();
        let mut ctx = granting();
        ctx.set_inbound(Inbound::new("GET", "/live/chat", ""));

        let path = Value::str(NvsStr::new(b"sockets/chat.nvs"));
        nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[path, Value::null()])
            .expect_err("a request with no slot cannot upgrade");
        let reported = ctx
            .pending()
            .expect("the throw is on the context")
            .into_owned();
        assert!(
            reported.contains("needs a connection to upgrade"),
            "the refusal did not name the missing connection: {reported}"
        );
        release_crossed(path);
    }

    /// A path the resolver will not answer for is a **throw in the request**,
    /// never an upgrade that fails later: ADR 0006's boundary turns only what
    /// the *child* produced into a value, and there is no child here yet.
    ///
    /// Both of the seam's ways of not answering are asked, because they are two
    /// different things and only one of them is the program's: a resolver that
    /// **refused** the path is a `RuntimeError` a `catch` sees, and **no
    /// resolver at all** is an embedder's mistake and a fatal. What the pair
    /// asserts is that each report names the member and the path, and that the
    /// slot is empty either way — a member that filled it with something it
    /// never got would open a connection on nothing.
    #[test]
    fn an_entry_a_resolver_refuses_is_a_throw_the_program_catches() {
        let path = Value::str(NvsStr::new(b"sockets/chat.nvs"));
        for installed in [true, false] {
            let refusing = installed.then(refusing);
            let mut ctx = granting();
            let slot = UpgradeSlot::new();
            upgradable(&mut ctx, &slot);

            nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[path, Value::null()])
                .expect_err("a path nothing can resolve does not open a connection");
            let reported = ctx
                .pending()
                .expect("the failure is on the context")
                .into_owned();
            assert!(
                reported.contains("`Core\\Socket::upgrade('sockets/chat.nvs')`"),
                "the report did not name the member and the path: {reported}"
            );
            assert!(
                !slot.is_filled(),
                "a path that never resolved still left a program for the connection"
            );
            drop(refusing);
        }
        release_crossed(path);
    }

    /// ADR 0023 § 2's refusal is the **parent's**, and this is where the module
    /// doc's "copied twice per upgrade" earns its cost: the crossing is made
    /// inside the request so a value with no meaning on the other side is a
    /// throw a `catch` sees, rather than a connection that closes after its
    /// `101` with nobody left to report to.
    ///
    /// `unset` is the cheapest of the three tags the walk refuses — the others
    /// are a closure and a resource, and neither is buildable here without a
    /// fixture that would pin itself rather than the rule. What matters is the
    /// **order**: the slot is still empty afterwards, so a request whose
    /// argument was refused has not half-upgraded.
    #[test]
    fn an_args_value_that_cannot_cross_is_refused_before_the_slot_is_filled() {
        let _resolver = resolving();
        let mut ctx = granting();
        let slot = UpgradeSlot::new();
        upgradable(&mut ctx, &slot);

        let path = Value::str(NvsStr::new(b"sockets/chat.nvs"));
        nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[path, Value::unset()])
            .expect_err("a value with no meaning on the other side does not cross");
        let reported = ctx
            .pending()
            .expect("the throw is on the context")
            .into_owned();
        assert!(
            reported.contains("cannot cross into a connection"),
            "the refusal did not name the crossing: {reported}"
        );
        assert!(
            !slot.is_filled(),
            "an argument that could not cross still opened a connection"
        );
        release_crossed(path);
    }

    /// ADR 0083 § 2's two written entry forms, prepared side by side and
    /// asserted to differ **only** in what the connection runs.
    ///
    /// § 2's rule is that the target is `spawn script`'s operand — a path or a
    /// static method — and § 1's is that either one opens the same kind of
    /// thing. A member that grew a second hand-over for the method form would
    /// pass a test that only asked whether each form worked, so what is
    /// asserted here is the *sameness*: the same cell filled, the same crossing
    /// (a copy, never the request's own graph), the same argument reaching the
    /// connection's ownership root, and the answer being each entry's own and
    /// nothing else.
    ///
    /// The method form's answer is `room` read back out of its **first
    /// positional slot**, which is ADR 0006 § *Decision*'s by-name binding
    /// arriving as a positional call. A member that bound by position would
    /// give the same answer here for a one-parameter entry, which is why
    /// `a_method_entry_whose_args_do_not_name_its_parameters_is_refused`
    /// stands beside this one.
    #[test]
    fn an_upgrade_by_static_method_is_the_same_isolate_as_an_upgrade_by_path() {
        let _resolver = resolving();

        let mut by_path = granting();
        let path_slot = UpgradeSlot::new();
        upgradable(&mut by_path, &path_slot);
        let path = Value::str(NvsStr::new(b"sockets/chat.nvs"));
        let path_args = a_room();
        nvs_runtime::call(nvs_core_socket_upgrade, &mut by_path, &[path, path_args])
            .expect("a path entry fills the slot");

        let mut by_method = granting();
        let method_slot = UpgradeSlot::new();
        upgradable(&mut by_method, &method_slot);
        let entry = a_callable("room", echoes_room);
        let method_args = a_room();
        nvs_runtime::call(
            nvs_core_socket_upgrade,
            &mut by_method,
            &[entry, method_args],
        )
        .expect("a static method entry fills the same slot");

        // 16 is the `Fixed` resolver's answer for the written path; 7 is the
        // `room` the entry was bound with. Each is its own entry's, and every
        // other fact below is asserted to be identical.
        for (form, slot, mine, answer) in [
            ("a path", &path_slot, path_args, 16),
            ("a static method", &method_slot, method_args, 7),
        ] {
            let (program, crossed) = slot
                .take()
                .unwrap_or_else(|| panic!("{form} left the connection nothing to start"))
                .into_parts();
            assert_ne!(
                crossed.array_ptr(),
                mine.array_ptr(),
                "{form} handed the connection the request's own array rather than a copy"
            );

            let mut connection = Ctx::buffered();
            assert_eq!(
                program(&mut connection, crossed).as_int(),
                Some(answer),
                "{form} ran something other than the entry that was written"
            );
            assert_eq!(
                connection.isolate_argument().array_ptr(),
                crossed.array_ptr(),
                "{form} did not leave the crossed argument in the connection's own root"
            );
            release_crossed(mine);
        }

        release_crossed(entry);
    }

    /// ADR 0006 § *Decision* binds `args:` **by name**, so a map that names
    /// something the entry does not declare — or omits something it does — is
    /// the ordinary named-argument error, reported at the call while there is
    /// still a `catch` above it.
    ///
    /// The `LogicError` and the untouched slot are the two halves: a member
    /// that judged the names inside the connection would have filled the cell
    /// first, and the program would already have been handed over by the time
    /// anything could refuse.
    #[test]
    fn a_method_entry_whose_args_do_not_name_its_parameters_is_refused() {
        let _resolver = resolving();
        let mut ctx = granting();
        let slot = UpgradeSlot::new();
        upgradable(&mut ctx, &slot);

        // The entry declares `userId`; the map names `room`, which is both a
        // parameter with no entry and an entry naming no parameter.
        let entry = a_callable("userId", echoes_room);
        let mine = a_room();
        nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[entry, mine])
            .expect_err("`args:` that does not name the entry's parameters is refused");
        let reported = ctx
            .pending()
            .expect("the throw is on the context")
            .into_owned();
        assert!(
            reported.contains("userId") && reported.contains("Core\\Socket::upgrade"),
            "the refusal named neither the parameter nor the member: {reported}"
        );
        assert!(
            !slot.is_filled(),
            "a refused entry still left something for the connection to start"
        );

        release_crossed(entry);
        release_crossed(mine);
    }

    /// `nvs_runtime::UpgradeSlot::fill` refuses a second upgrade, and this is
    /// that rule read from the member's side: the second call throws and **the
    /// first one still stands**, because a fill that overwrote would open a
    /// connection the program did not think it had asked for.
    #[test]
    fn a_second_upgrade_on_one_request_is_refused_and_the_first_still_stands() {
        let _resolver = resolving();
        let mut ctx = granting();
        let slot = UpgradeSlot::new();
        upgradable(&mut ctx, &slot);

        let first = Value::str(NvsStr::new(b"sockets/chat.nvs"));
        let second = Value::str(NvsStr::new(b"sockets/other-and-longer.nvs"));
        nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[first, Value::null()])
            .expect("the first upgrade fills the slot");
        nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[second, Value::null()])
            .expect_err("one request opens at most one connection");
        let reported = ctx
            .pending()
            .expect("the throw is on the context")
            .into_owned();
        assert!(
            reported.contains("called twice"),
            "the refusal did not name the second call: {reported}"
        );

        let (program, crossed) = slot
            .take()
            .expect("the first upgrade is still there")
            .into_parts();
        let mut connection = Ctx::buffered();
        assert_eq!(
            program(&mut connection, crossed).as_int(),
            Some(16),
            "the second call overwrote the first upgrade"
        );
        release_crossed(first);
        release_crossed(second);
    }
}
