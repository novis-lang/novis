//! `Core\Socket` — `rule:concurrency/a-connection-is-a-root-isolate` and `rule:concurrency/an-upgrade-is-spawn-shaped`
//! 's WebSocket upgrade, as the signature a program writes.
//!
//! § 1 makes a connection a **root isolate** rather than a suspended request:
//! its own arena, its own `[limits]` budget, its own grants, and none of the
//! upgrading request's heap. § 2 makes opening one `spawn script`-shaped, so
//! the operand here is that construct's operand under exactly
//! `rule:security/isolate-shares-nothing`'s rule — a path, or
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
//! § 3's surface is over it as well — `current()`, `receive()` and the two
//! `send` rows, plus the [`MESSAGE`] each wait answers with. [`crate::topic`]
//! is the far half of `receive()`'s second source and is whole: § 4's
//! `subscribe`, `unsubscribe` and the `publish` that fans out across cores.
//! The waits both members sit inside are `nvs_server::bounds`' — § 7's idle,
//! lifetime and send timeout, armed by the framing layer because the deadline
//! belongs to the descriptor. Nothing here can name a duration, which is why
//! `send`'s own doc says only that the failure arrives as a throw.
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
//! is `rule:security/capability-check-at-the-door`'s own door for `script.spawn` and asks the question with the
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
//! (`rule:concurrency/nothing-is-still-running-when-a-call-returns`). Each
//! of those is the opposite of what § 1 states, and none of them is a builder
//! away.
//!
//! So the member **prepares** an isolate and records it; the **connection**
//! starts it. What is prepared is what `nvs_host::Isolate::new` takes — a
//! `nvs_runtime::script::Program` and the argument value that has already
//! crossed — and it is prepared here, inside the request, because all three
//! things that can refuse belong to the request:
//!
//! - **The capability.** `rule:security/isolate-shares-nothing`'s `script.spawn` grant, and the root check
//!   under it, are asked against the request's own configuration overlay, so a
//!   request that narrowed its grants cannot upgrade into a connection holding
//!   the ones it gave up — § 1's "narrowed from the request's, never widened".
//! - **The argument.** `rule:classes/graph-copy`'s refusal is the *parent's* fault, and
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
//! `nvs_runtime::script`'s `bound_arguments`). A `Core` call has no such
//! constant, because its entry is
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
//! request `hyper` framed an upgrade for, and by `nvs run --request` to a
//! request file whose `Upgrade` header names `websocket` — and the body below
//! is the filling: it resolves the entry into one `Program`, crosses `args`,
//! and leaves the pair in the slot. A connection is the only thing an
//! upgrade can happen to, so a request that did not arrive on one — a CLI
//! program, a `spawn script` child, a request the server could offer no
//! upgrade for — has no slot to write into and this member throws, for the
//! reason `Core\Request::method()` throws there
//! (`rule:security/request-state-throws-in-an-isolate`). And the server holds
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
//! Because calling it performs the upgrade — `rule:concurrency/an-upgrade-is-spawn-shaped`'s own bullet, which
//! is the home of the reasoning and of what the alternative would cost. The
//! short of it: nothing in this language reads a handler's return, so
//! [`crate::response`] is a class of `void` members writing the response the
//! request already has, and this is one more of them.
//!
//! # Why `limits:`, `grants:` and `on:` are not parameters
//!
//! § 2 lists four options and calls them "0006's, spelled as ordinary named
//! arguments". Three of the four narrow the child — the budget it may spend,
//! the authorities it inherits, the core it starts on — and this member
//! forwards none of them: an upgrade opens the connection isolate itself, and
//! nothing in the body carries a sub-cap or a grant to it. A row declaring
//! them would accept a narrowing and drop it, which is the one reading of
//! `grants:` that hands the connection the authority its request meant to give
//! up, so they are simply not declared and a program writing `grants: {}` gets
//! the unknown-argument refusal rather than a silent widening. They arrive
//! here when the body forwards them; `rule:concurrency/an-upgrades-options-are-spawn-scripts`
//! is that rule's one home and this module does not restate it.
//!
//! `output:` is the fifth and does not apply: an isolate's `output:` chooses
//! between its own buffer and the parent's stream
//! (`rule:tooling/echo-always-has-a-sink`
//! ), and a connection's output is the socket.

use nvs_runtime::script::{Program, ResolveError};
use nvs_runtime::{
    Closing, Ctx, Delivery, Fault, NvsStr, PeerFrame, ThrownClass, Upgrade, Value, copy_graph,
};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// `Core\Socket`'s fully-qualified name, in one place so the row and every
/// message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Socket";

/// `Core\Socket`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "A WebSocket connection from a client to your server. A request calls `upgrade` to \
            turn itself into a connection that runs a script of its own. That script calls \
            `current` to get the connection, then `receive` and `send` to talk to the client.",
};

/// `Core\Socket`'s registry rows — `rule:concurrency/an-upgrade-is-spawn-shaped`'s `upgrade` at the door, and
/// § 3's three inside. See [`crate::registry::CLASSES`].
///
/// # It is a namespace class and an instance class at once
///
/// `upgrade` is called on the class from the *request*, and `receive`/`send`
/// are called on a value inside the *connection*: two rosters, one name,
/// because § 2 and § 3 are two ends of one thing and a program that had to
/// learn a second class name for the far end would be learning the boundary
/// twice.
///
/// **The instance carries no slots**, which is [`CLASS`]'s one unusual
/// property and the reason [`crate::instance`]'s descriptor table stopped
/// keying on slots alone. A connection's state — the peer, and § 3's second
/// source — lives on [`nvs_runtime::Ctx`], because it is the *isolate* that
/// holds a socket and the isolate outlives every value a program makes from
/// it. So `current()` answers a handle, and two calls to it answer two objects
/// that are not identical: identity is `Core\Socket`'s wrong question, since
/// there is exactly one connection per isolate and no second one to tell it
/// from.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "upgrade",
            names: &["entry", "args"],
            // [`CoreTy::Entry`] is § 2's "0006's operand", as the one mark that
            // carries that ADR's whole rule to a call site: a path or a static
            // method written `Chat::run(...)`, and never a `callable` in a
            // variable. It classifies as a sink for the reason a path always does
            // — its content becomes the instruction "execute this file", `rule:security/sink-predicate`
            // 's definition, and
            // `rule:http-server/a-path-is-never-derived-from-a-url`
            // is the same rule written for the server, a filesystem path never
            // derived from a URL at request time. `args` takes no expected type at
            // all, for the reason the sibling site takes none: `rule:classes/graph-copy`'s walk
            // decides what may cross, and that is a run-time question for
            // everything a declared type does not already settle.
            params: &[CoreTy::Entry, CoreTy::Mixed],
            defaults: &[Const::Null],
            return_ty: CoreTy::Void,
            symbol: UPGRADE_SYMBOL,
            doc: Some(&UPGRADE_DOC),
        },
        CoreMethod {
            name: "current",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: CURRENT_SYMBOL,
            doc: Some(&CURRENT_DOC),
        },
    ],
    instance: &[
        CoreMethod {
            name: "receive",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(MESSAGE_NAME)),
            symbol: RECEIVE_SYMBOL,
            doc: Some(&RECEIVE_DOC),
        },
        // RFC 6455's two payload kinds are two members and not one parameter
        // spelled `string|bytes`: a text frame and a binary frame are two
        // things on the wire, and the opcode a call picks is the member it
        // calls rather than the tag of the value it passes. The qualifier does
        // not argue either way — each parameter carries [`Qual::Neutral`], and
        // a union of the two would answer the same mark
        // ([`CoreTy::classification`]), so § 3's own loop, which forwards what
        // the peer sent, compiles under either spelling.
        CoreMethod {
            name: "send",
            names: &["frame"],
            // [`Qual::Neutral`] because a frame is not an instruction on this
            // side of the wire — `rule:security/sink-predicate`'s test — and because `send`
            // answers `void`, so there is no result for the argument's
            // qualifier to reach.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: SEND_SYMBOL,
            doc: Some(&SEND_DOC),
        },
        CoreMethod {
            name: "sendBytes",
            names: &["frame"],
            params: &[CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: SEND_BYTES_SYMBOL,
            doc: Some(&SEND_BYTES_DOC),
        },
    ],
    slots: &[],
    constants: &[],
};

/// The symbol [`CLASS`]'s `upgrade` row is reached through.
const UPGRADE_SYMBOL: &str = "nvs_core_socket_upgrade";

/// The symbol [`CLASS`]'s `current` row is reached through.
const CURRENT_SYMBOL: &str = "nvs_core_socket_current";

/// The symbol [`CLASS`]'s `receive` row is reached through.
const RECEIVE_SYMBOL: &str = "nvs_core_socket_receive";

/// The symbols [`CLASS`]'s two `send` rows are reached through.
const SEND_SYMBOL: &str = "nvs_core_socket_send";
const SEND_BYTES_SYMBOL: &str = "nvs_core_socket_send_bytes";

/// `Core\Socket::upgrade`'s reference card — `rule:core-api/reference-card`.
const UPGRADE_DOC: MethodDoc = MethodDoc {
    short: "Turns this request into a WebSocket connection that runs `$entry`. The connection \
            starts when the request ends. It runs in its own isolate, which shares no memory \
            with the request.",
    params: &[
        ParamDoc {
            name: "entry",
            desc: "What the connection runs: the path of a file, the same as `spawn script` \
                   takes, or a static method written `Chat::run(...)`. A closure is not \
                   allowed, because the connection shares no values with the request.",
            shape: &[],
        },
        ParamDoc {
            name: "args",
            desc: "The values the connection starts with. They are copied into the connection. \
                   For a method, each value goes to the parameter with the same name. A \
                   `tainted` value is still `tainted` in the connection.",
            shape: &[],
        },
    ],
    ret: "Nothing. The call prepares the connection, and it starts when the request ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The request did not ask for a WebSocket. The `$entry` file is not granted by \
                   `script.spawn` or does not compile. Or this request already called `upgrade`.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "An `$args` value cannot be copied into the connection: a resource, a `secret`, \
                   or an array nested too deep. For a method, `$args` is missing a parameter or \
                   has a name the method does not declare.",
        },
    ],
};

/// `Core\Socket::current`'s reference card — `rule:core-api/reference-card`.
const CURRENT_DOC: MethodDoc = MethodDoc {
    short: "Returns the WebSocket connection this script is running for. It is the first line of \
            every script that `Core\\Socket::upgrade` starts.",
    params: &[],
    ret: "The connection. Call `receive` and `send` on it to talk to the client. Each call \
          returns a new object for the same connection, so compare what the client sends, not \
          the objects.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The script is not running for a connection. An ordinary request, a `spawn script` \
               child and a command line program all throw this.",
    }],
};

/// `Core\Socket::receive`'s reference card — `rule:core-api/reference-card`.
const RECEIVE_DOC: MethodDoc = MethodDoc {
    short: "Waits for the next message and returns it. A message comes from the client, or from \
            a topic this connection subscribed to.",
    params: &[],
    ret: "The next message, or `null` when the client has closed the connection. A connection \
          script reads in a loop that stops at `null`. `topic()` on the message is `null` for a \
          message from the client, and what the client sent is `tainted`. A connection that \
          falls too far behind its topics is closed, and then this returns `null` too.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The script is not running for a connection, so there is no client to wait for.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The connection broke, or the client sent something the WebSocket protocol does \
                   not allow.",
        },
    ],
};

/// `Core\Socket::send`'s reference card — `rule:core-api/reference-card`.
const SEND_DOC: MethodDoc = MethodDoc {
    short: "Sends one text message to the client, and waits until it is written.",
    params: &[ParamDoc {
        name: "frame",
        desc: "The text to send. The client receives it as one message. A `tainted` string is \
               allowed, so you can send back what the client sent you.",
        shape: &[],
    }],
    ret: "Nothing.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The script is not running for a connection, so there is no client to send to.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The connection broke, or the client stopped reading and the message could not \
                   be written in time. The wait always ends.",
        },
    ],
};

/// `Core\Socket::sendBytes`'s reference card — `rule:core-api/reference-card`.
const SEND_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Sends one binary message to the client, and waits until it is written. `send` is the \
            same method for text.",
    params: &[ParamDoc {
        name: "frame",
        desc: "The bytes to send. They are sent as they are, and nothing checks them.",
        shape: &[],
    }],
    ret: "Nothing.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The script is not running for a connection, so there is no client to send to.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The connection broke, or the client stopped reading and the message could not \
                   be written in time, as for `send`.",
        },
    ],
};

/// `Core\Socket\Message`'s fully-qualified name, as [`CoreTy::Instance`] spells
/// it.
pub(crate) const MESSAGE_NAME: &str = r"Core\Socket\Message";

/// [`MESSAGE`]'s slots, in the order [`message_of_frame`] and
/// [`message_of_delivery`] fill them.
const MESSAGE_TOPIC: usize = 0;
const MESSAGE_TEXT: usize = 1;
const MESSAGE_BYTES: usize = 2;
const MESSAGE_VALUE: usize = 3;

/// `rule:concurrency/a-connection-is-a-loop`'s message: the one shape every `receive` answers in,
/// whether a served connection's peer frame or delivery, or an outbound `Core\Http\Socket`'s
/// peer frame (`crate::http::socket`).
///
/// # One class, not two, and `topic` is what tells them apart
///
/// § 3's loop is written `if ($msg->topic() != null)`, so the discrimination is
/// a reader on the message rather than a type the caller matches on. Two
/// classes would make the loop a type switch and would give `receive` a union
/// return, which is a second control-flow style for the one member § 3 exists
/// to keep straight-line.
///
/// # Four readers, and each answers `null` for what it is not
///
/// A peer frame fills exactly one of `text` and `bytes` — RFC 6455 has two
/// payload kinds and the framing layer has already decided which arrived
/// ([`nvs_runtime::PeerFrame`]) — and a delivery fills `topic` and `value`.
/// `bytes` is not in § 3's example and is here because the seam underneath
/// already carries binary frames: leaving it out would make a binary payload
/// unreachable from Novis while the peer is free to send one, which is the
/// silent wrong answer `rule:errors/ambiguous-input-refused`
/// refuses. A reader answering `null` is the honest report that this message is
/// the other kind, and it costs a caller the `?` it was already writing around
/// `receive`.
///
/// **`text` and `bytes` are `tainted` and `value` is not.** § 3 marks a
/// received frame's payload as untrusted input, which is what those two carry;
/// a delivery's value came from another isolate on this side of the wire and
/// crossed by `rule:classes/graph-copy`'s copy, so it arrives with whatever qualifiers it
/// already had and this row may not add one.
pub(crate) const MESSAGE: CoreClass = CoreClass {
    name: MESSAGE_NAME,
    doc: Some(&MESSAGE_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "topic",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: MESSAGE_TOPIC_SYMBOL,
            doc: Some(&MESSAGE_TOPIC_DOC),
        },
        CoreMethod {
            name: "text",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: MESSAGE_TEXT_SYMBOL,
            doc: Some(&MESSAGE_TEXT_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedBytes),
            symbol: MESSAGE_BYTES_SYMBOL,
            doc: Some(&MESSAGE_BYTES_DOC),
        },
        CoreMethod {
            name: "value",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: MESSAGE_VALUE_SYMBOL,
            doc: Some(&MESSAGE_VALUE_DOC),
        },
    ],
    slots: &["topic", "text", "bytes", "value"],
    constants: &[],
};

/// The symbols [`MESSAGE`]'s four readers are reached through.
const MESSAGE_TOPIC_SYMBOL: &str = "nvs_core_socket_message_topic";
const MESSAGE_TEXT_SYMBOL: &str = "nvs_core_socket_message_text";
const MESSAGE_BYTES_SYMBOL: &str = "nvs_core_socket_message_bytes";
const MESSAGE_VALUE_SYMBOL: &str = "nvs_core_socket_message_value";

/// `Core\Socket\Message`'s class card — `rule:core-api/reference-card`.
const MESSAGE_CARD: ClassDoc = ClassDoc {
    short: "One message on a WebSocket connection, as `Core\\Socket::receive` returns it. A \
            message from the client has `text` or `bytes`. A message from a topic has a `topic` \
            and a `value`.",
};

/// `Core\Socket\Message::topic`'s reference card — `rule:core-api/reference-card`.
const MESSAGE_TOPIC_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the topic the message was published to.",
    params: &[],
    ret: "The topic name for a message from a topic. It is `null` for a message from the \
          client. The name is not `tainted`, because your program chose it when it subscribed.",
    errors: &[],
};

/// `Core\Socket\Message::text`'s reference card — `rule:core-api/reference-card`.
const MESSAGE_TEXT_DOC: MethodDoc = MethodDoc {
    short: "Returns the text of a text message from the client.",
    params: &[],
    ret: "The text, exactly as the client sent it. It is `tainted`, because it comes from the \
          client. It is `null` for a binary message and for a message from a topic.",
    errors: &[],
};

/// `Core\Socket\Message::bytes`'s reference card — `rule:core-api/reference-card`.
const MESSAGE_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns the bytes of a binary message from the client.",
    params: &[],
    ret: "The bytes, exactly as the client sent them. They are `tainted`, because they come \
          from the client. It is `null` for a text message and for a message from a topic.",
    errors: &[],
};

/// `Core\Socket\Message::value`'s reference card — `rule:core-api/reference-card`.
const MESSAGE_VALUE_DOC: MethodDoc = MethodDoc {
    short: "Returns the value that was published to the topic.",
    params: &[],
    ret: "A copy of the published value. Changing the copy does not change the value the \
          publisher has. It is `null` for a message from the client.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        UPGRADE_SYMBOL => (nvs_core_socket_upgrade as *const ()).cast(),
        CURRENT_SYMBOL => (nvs_core_socket_current as *const ()).cast(),
        RECEIVE_SYMBOL => (nvs_core_socket_receive as *const ()).cast(),
        SEND_SYMBOL => (nvs_core_socket_send as *const ()).cast(),
        SEND_BYTES_SYMBOL => (nvs_core_socket_send_bytes as *const ()).cast(),
        MESSAGE_TOPIC_SYMBOL => (nvs_core_socket_message_topic as *const ()).cast(),
        MESSAGE_TEXT_SYMBOL => (nvs_core_socket_message_text as *const ()).cast(),
        MESSAGE_BYTES_SYMBOL => (nvs_core_socket_message_bytes as *const ()).cast(),
        MESSAGE_VALUE_SYMBOL => (nvs_core_socket_message_value as *const ()).cast(),
        _ => return None,
    })
}

/// `rule:concurrency/an-upgrade-is-spawn-shaped`'s entry, resolved into the one thing a connection runs.
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
/// call `rule:security/isolate-shares-nothing` names as where a named-argument mismatch is reported, and it
/// is the last one that still has a `catch` above it. It is *only* judged —
/// what crosses is the caller's own copy, made after this returns.
///
/// # Errors
///
/// Everything [`nvs_runtime::script::resolve`] refuses, in `spawn script`'s own
/// wording with `member` in front of it; `rule:security/isolate-shares-nothing`'s named-argument mismatch for
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
    // `rule:programs/path-literals-resolve-from-their-file`, as at `spawn
    // script`: a relative literal arrives joined to its file's folder, so a
    // relative path here was built while the program ran.
    if let Some(message) =
        nvs_runtime::capability::relative(std::path::Path::new(path), &format!("`{member}`"))
    {
        return Err(Fault::thrown_as(ThrownClass::Runtime, message));
    }
    // `rule:security/capability-check-at-the-door`'s door is inside `resolve` and not here, exactly as it is
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
        // Already a whole sentence naming the capability and the path (`rule:security/denial-is-a-runtime-error`
        // ), so it is thrown as written rather than framed twice.
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

/// `rule:concurrency/an-upgrade-is-spawn-shaped`'s **method entry**, prepared: the callable the request is
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
/// never the parent's slots, so `rule:security/isolate-shares-nothing`'s unshared class statics hold whichever
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
/// A `callable` recording no parameter names at all, and `rule:security/isolate-shares-nothing`'s
/// named-argument mismatch between the target's parameters and `args`.
fn method_program(ctx: &Ctx, entry: Value, args: Value, member: &str) -> Result<Program, Fault> {
    // `None` is a closure with no `fn#names` field, which is an `fn` literal:
    // `nvs_types::expr::isolate` refuses one where an entry is expected, so
    // this is a compiler disagreement rather than a program's — but it is
    // reported as a throw and not a fatal, because a wrong refusal is
    // recoverable and a wrong `FATAL` ends the request.
    let Some(names) = nvs_runtime::closure_param_names(entry)? else {
        return Err(Fault::thrown(format!(
            "`{member}` was handed a `callable` that records no parameter names, so `rule:security/isolate-shares-nothing`'s \
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
        // The whole guard is moved into the closure here. The body reads only
        // `held.0`, a `Copy` field, and a closure that names only a field
        // captures only that field: the guard would then be dropped when
        // `method_program` returns, releasing the reference before the
        // connection ever runs.
        let held = held;
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
        let bound = nvs_runtime::script::bound_arguments(&names, child.isolate_argument());
        match nvs_runtime::call_closure(child, held.0, &bound) {
            Ok(value) => value,
            // The judgement `call_closure` makes on this frame's behalf — an
            // argument whose tag the parameter does not admit, `rule:security/isolate-shares-nothing`'s
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
    /// two forms become one. **The argument** is `rule:classes/graph-copy`'s crossing, and
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

/// The refusal every § 3 member makes on a context that was handed no socket.
///
/// One helper rather than three messages, because the three are the same fact
/// about the same context and a program reading two of them should not have to
/// notice they were worded differently. `LogicError` rather than a
/// `RuntimeError`: nothing about the environment could have made this call
/// work, so it is the call that is wrong — the same reading `Core\Request`'s
/// members outside a request take.
fn no_connection(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "`Core\\Socket::{member}` needs a connection and this program is not one: only a \
             script `Core\\Socket::upgrade` opened has one"
        ),
    )
}

/// One [`MESSAGE`] built out of a frame the peer sent.
///
/// Exactly one of the two payload slots is filled and `topic` is `null`, which
/// is the class doc's "each reader answers `null` for what it is not" written
/// as the one place that decides it.
fn message_of_frame(frame: PeerFrame) -> Value {
    let (text, bytes) = match frame {
        PeerFrame::Text(payload) => (Value::str(NvsStr::new(payload.as_bytes())), Value::null()),
        PeerFrame::Binary(payload) => (Value::null(), Value::bytes(NvsStr::new(&payload))),
    };
    crate::instance::build(&MESSAGE, [Value::null(), text, bytes, Value::null()])
}

/// One [`MESSAGE`] built out of a value the bus delivered.
///
/// The topic is read before the value is taken, because taking it consumes the
/// delivery — [`Delivery`] hands its one owned reference on rather than
/// copying it, and the slot is where that reference comes to rest.
fn message_of_delivery(delivery: Delivery) -> Value {
    let topic = Value::str(NvsStr::new(delivery.topic().as_bytes()));
    let value = delivery.into_value();
    crate::instance::build(&MESSAGE, [topic, Value::null(), Value::null(), value])
}

/// The half both `send` rows share: the receiver check, the peer, and the one
/// throw a failed write is.
///
/// The two differ only in which [`PeerFrame`] they built, which is the whole
/// of what RFC 6455's two payload kinds are — so the wait, the refusal outside
/// a connection and the wording of a failure are written once.
///
/// # Errors
///
/// [`no_connection`] on a context with no peer, and a `RuntimeError` naming
/// what the framing layer reported — which is `rule:concurrency/a-connection-is-a-loop`'s "throws on the
/// send timeout rather than waiting forever", the timeout itself being the
/// implementation's under `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`.
fn send_frame(
    ctx: &mut Ctx,
    receiver: Value,
    frame: PeerFrame,
    member: &str,
) -> Result<Value, Fault> {
    crate::instance::receiver(receiver, &CLASS, member)?;
    let peer = ctx.peer().ok_or_else(|| no_connection(member))?;
    // No case can reach this: a `.nvst` case runs a script that was handed no
    // socket, so it never gets past the refusal above.
    // `send_throws_on_the_send_timeout_rather_than_waiting` is the `#[test]`
    // that asserts it instead.
    peer.send(frame).map_err(|failed| {
        Fault::thrown(format!(
            "`Core\\Socket::{member}` failed on the connection: {}",
            failed.message()
        ))
    })?;
    Ok(Value::null())
}

/// What one of [`MESSAGE`]'s four readers answers: the slot the message was
/// built with, retained for the caller.
///
/// One helper rather than four bodies, for `crate::request`'s `part_slot`
/// reason — none of them spells a slot index itself. The read is
/// [`crate::instance::read_slot`], which `crate::sse`'s two readers reach for
/// the same three steps.
///
/// # Errors
///
/// The [`crate::instance::receiver`] fault a wrongly-tagged receiver is, which
/// compiled code cannot produce.
fn message_slot(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    crate::instance::read_slot(args, &MESSAGE, index, member)
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket::current(): Core\Socket` — `rule:concurrency/a-connection-is-a-loop`'s first line.
    ///
    /// The handle carries nothing, so this allocates an object with no slots
    /// and the *context* is what every member on it reads. [`CLASS`]'s own doc
    /// owns why that is the right shape rather than a slot holding a socket.
    ///
    /// The question it asks is `has_peer`, which is the one thing that
    /// separates a connection isolate from every other kind of context — an
    /// ordinary request, a `spawn script` child and a CLI program all answer
    /// `false`, so there is no list of hosts here to keep in step with
    /// anything.
    fn nvs_core_socket_current(ctx, _args: [0]) {
        if !ctx.has_peer() {
            return Err(no_connection("current"));
        }
        Ok(crate::instance::build(&CLASS, []))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket::receive(): ?Core\Socket\Message` — `rule:concurrency/a-connection-is-a-loop`'s one
    /// wait, over both sources.
    ///
    /// **The bus is drained before the socket is read**, and that order is the
    /// whole of the select this member can make today: a queued delivery is
    /// answered without touching the peer, and only an empty queue parks on
    /// the socket. The drain has two steps for one reason — what another core
    /// published is waiting as bytes rather than as a value, so
    /// `nvs_stdlib::topic`'s `deliver_from_other_cores` reads it back into
    /// this core's queues first and `take_delivery` then answers from one
    /// queue however far the publisher was. What it does not do yet is wake a
    /// park that is already running — `nvs_runtime::Ctx::deliver`'s own known
    /// gap says so, and it is the same gap whichever core published.
    ///
    /// **This is also where a slow subscriber is closed** (§ 4). The publisher
    /// that overflowed this connection's queue could not reach its peer — it is
    /// a field of this context, on this isolate's own stack — so it raised the
    /// overflow on the queue and this wait is what obeys it: the peer is told
    /// [`nvs_runtime::Closing::SlowSubscriber`]'s code and the member answers
    /// § 3's `null`, which is the condition the connection loop already ends
    /// on. It is asked before the drain rather than after, because a connection
    /// being closed for missing one value has no business being handed the
    /// values it did not miss.
    ///
    /// Draining first rather than last is not arbitrary: the peer's read is
    /// the operation that blocks, so checking it first would make a delivery
    /// wait for a frame that may never come, which is the one ordering a
    /// program could observe as a hang.
    fn nvs_core_socket_receive(ctx, args: [1]) {
        crate::instance::receiver(args[0], &CLASS, "receive")?;
        if !ctx.has_peer() {
            return Err(no_connection("receive"));
        }
        crate::topic::deliver_from_other_cores(ctx);
        if ctx.inbox_overflowed() {
            ctx.peer()
                .expect("`has_peer` answered above and nothing since could have taken it")
                .close(Closing::SlowSubscriber);
            return Ok(Value::null());
        }
        if let Some(delivery) = ctx.take_delivery() {
            return Ok(message_of_delivery(delivery));
        }
        let received = ctx
            .peer()
            .expect("`has_peer` answered above and nothing since could have taken it")
            .receive();
        match received {
            // § 3's `null`, and the condition the connection loop ends on. Not
            // an error: an orderly close is how every connection finishes.
            Ok(None) => Ok(Value::null()),
            Ok(Some(frame)) => Ok(message_of_frame(frame)),
            // No case can reach this: a `.nvst` case runs a script that was
            // handed no socket, so it never gets past the refusal above.
            // `receive_reports_a_failed_socket_as_a_throw` is the `#[test]`
            // that asserts it instead.
            Err(failed) => Err(Fault::thrown(format!(
                "`Core\\Socket::receive` failed on the connection: {}",
                failed.message()
            ))),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket::send(string|bytes $frame): void` — `rule:concurrency/a-connection-is-a-loop`'s send,
    /// which throws on the send timeout rather than waiting forever.
    ///
    /// The timeout is the implementation's, under
    /// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s rule that
    /// no outbound wait has an unbounded spelling: this body cannot name a
    /// duration, because the wait happens inside the codec that owns the
    /// descriptor. What it owes is that the failure arrives as a throw at the
    /// call site rather than as a fatal error, so a connection script can log
    /// it and close — which is `Fault::thrown` and nothing more.
    ///
    fn nvs_core_socket_send(ctx, args: [2]) {
        // Unreachable from source: the row declares `string`, so `E0401`
        // refuses every other spelling before this body runs.
        let text = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "`Core\\Socket::send` expected a `string`, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        send_frame(ctx, args[0], PeerFrame::Text(text.to_owned()), "send")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket::sendBytes(bytes $frame): void` — [`nvs_core_socket_send`]'s
    /// twin for RFC 6455's other payload kind, and the same wait.
    fn nvs_core_socket_send_bytes(ctx, args: [2]) {
        // Unreachable from source: the row declares `bytes`, so `E0401`
        // refuses every other spelling before this body runs.
        let octets = args[1].as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "`Core\\Socket::sendBytes` expected a `bytes`, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        send_frame(ctx, args[0], PeerFrame::Binary(octets.to_vec()), "sendBytes")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket\Message::topic(): ?string` — the topic a delivery arrived
    /// on, and `null` for a frame the peer sent.
    fn nvs_core_socket_message_topic(_ctx, args: [1]) {
        message_slot(args, MESSAGE_TOPIC, "topic")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket\Message::text(): ?tainted string` — a text frame's
    /// payload.
    fn nvs_core_socket_message_text(_ctx, args: [1]) {
        message_slot(args, MESSAGE_TEXT, "text")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket\Message::bytes(): ?tainted bytes` — a binary frame's
    /// payload.
    fn nvs_core_socket_message_bytes(_ctx, args: [1]) {
        message_slot(args, MESSAGE_BYTES, "bytes")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Socket\Message::value(): mixed` — what a publisher put on the
    /// topic, already copied across the boundary.
    fn nvs_core_socket_message_value(_ctx, args: [1]) {
        message_slot(args, MESSAGE_VALUE, "value")
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::script::{Installed, Program, Resolver, install};
    use nvs_runtime::{Ctx, Inbound, NvsArray, NvsStr, UpgradeSlot, Value};

    use nvs_runtime::{Closing, Delivery, PeerError, PeerFrame, PeerSocket};

    use super::{
        CLASS, nvs_core_socket_current, nvs_core_socket_message_bytes,
        nvs_core_socket_message_text, nvs_core_socket_message_topic, nvs_core_socket_message_value,
        nvs_core_socket_receive, nvs_core_socket_send, nvs_core_socket_send_bytes,
        nvs_core_socket_upgrade, release_crossed,
    };

    /// A peer whose answers are scripted and whose sends are recorded.
    ///
    /// The framing is `nvs_server::socket`'s and none of it is reachable from
    /// this crate — `nvs-stdlib` names `nvs-runtime` and not the server — so
    /// what these cases assert is the *seam*: which source a wait took its
    /// answer from, and what a program sees when the socket says no. A
    /// `PeerSocket` is exactly two operations wide, which is what makes that a
    /// complete double rather than a partial one.
    #[derive(Debug, Default)]
    struct Script {
        /// What successive `receive` calls answer, in order; an exhausted
        /// queue answers `Ok(None)`, which is the peer having closed.
        incoming: std::collections::VecDeque<Result<Option<PeerFrame>, PeerError>>,
        /// Every frame the program sent, in order.
        sent: Vec<PeerFrame>,
        /// What a `send` fails with, if it fails — `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s finite wait, in
        /// the wording the framing layer would report it with.
        refuse_send: Option<String>,
        /// Why this socket was closed, once something closed it — § 4's code
        /// is a thing a case reads back rather than a thing it takes on trust.
        closed: Option<Closing>,
    }

    /// A handle onto one [`Script`], so the case can read what was sent after
    /// the context has taken the socket.
    #[derive(Clone, Debug, Default)]
    struct Peer(std::rc::Rc<std::cell::RefCell<Script>>);

    impl PeerSocket for Peer {
        fn receive(&mut self) -> Result<Option<PeerFrame>, PeerError> {
            self.0.borrow_mut().incoming.pop_front().unwrap_or(Ok(None))
        }

        fn send(&mut self, frame: PeerFrame) -> Result<(), PeerError> {
            let mut script = self.0.borrow_mut();
            if let Some(failure) = script.refuse_send.clone() {
                return Err(PeerError::new(failure));
            }
            script.sent.push(frame);
            Ok(())
        }

        fn close(&mut self, why: Closing) {
            self.0.borrow_mut().closed = Some(why);
        }
    }

    /// A context that is a connection's: `rule:concurrency/a-connection-is-a-root-isolate`'s isolate with the
    /// socket already moved onto it, which is what `nvs_server::socket` does
    /// for a real one.
    fn connected(peer: &Peer) -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_peer(Box::new(peer.clone()));
        ctx
    }

    /// Gives back one reference a `call` handed this frame.
    #[expect(
        unsafe_code,
        reason = "the case owns every reference `nvs_runtime::call` answered \
                  with, and this is the only place it gives one back"
    )]
    fn dropped(value: Value) {
        // SAFETY: the reference released is the one the call answered with,
        // and nothing else in the case points at it.
        unsafe { value.release() };
    }

    /// One `receive()` on a connection whose peer answers `script`.
    fn receive_on(ctx: &mut Ctx, conn: Value) -> Value {
        nvs_runtime::call(nvs_core_socket_receive, ctx, &[conn]).expect("the wait answered")
    }

    /// `rule:concurrency/a-connection-is-a-loop`'s one wait is one member over two sources, so a connection
    /// holding a queued delivery *and* a frame from the peer answers both from
    /// the same call site — which is the whole reason § 3 refuses a second
    /// `Core\Topic::receive()`.
    ///
    /// The bus is drained first, and the case pins that order: the peer's read
    /// is the operation that blocks, so a delivery behind it would wait for a
    /// frame that may never come.
    // covers: Core\Socket::receive, Core\Socket::current, Core\Socket\Message::topic
    // covers: Core\Socket\Message::text, Core\Socket\Message::bytes, Core\Socket\Message::value
    #[test]
    fn receive_answers_a_peer_frame_and_a_topic_delivery_from_one_wait() {
        let peer = Peer::default();
        peer.0
            .borrow_mut()
            .incoming
            .push_back(Ok(Some(PeerFrame::Text("hello".to_owned()))));
        let mut ctx = connected(&peer);
        assert!(
            ctx.deliver(Delivery::new("room:lobby", Value::int(7)))
                .is_none(),
            "an empty queue takes the first delivery"
        );

        let conn = nvs_runtime::call(nvs_core_socket_current, &mut ctx, &[])
            .expect("a connection isolate answers `current()`");

        let delivery = receive_on(&mut ctx, conn);
        let topic = nvs_runtime::call(nvs_core_socket_message_topic, &mut ctx, &[delivery])
            .expect("a delivery names its topic");
        assert_eq!(topic.as_text(), Some("room:lobby"));
        let carried = nvs_runtime::call(nvs_core_socket_message_value, &mut ctx, &[delivery])
            .expect("a delivery carries its value");
        assert_eq!(carried.as_int(), Some(7));

        let frame = receive_on(&mut ctx, conn);
        let text = nvs_runtime::call(nvs_core_socket_message_text, &mut ctx, &[frame])
            .expect("a text frame carries its payload");
        assert_eq!(text.as_text(), Some("hello"));
        let no_topic = nvs_runtime::call(nvs_core_socket_message_topic, &mut ctx, &[frame])
            .expect("a peer frame answers its topic");
        assert!(
            no_topic.as_text().is_none(),
            "a peer frame was reported as a delivery"
        );
        let no_bytes = nvs_runtime::call(nvs_core_socket_message_bytes, &mut ctx, &[frame])
            .expect("a text frame answers `bytes`");
        assert!(
            no_bytes.as_bytes().is_none(),
            "a text frame answered on the binary reader too"
        );

        for value in [
            topic, carried, text, no_topic, no_bytes, delivery, frame, conn,
        ] {
            dropped(value);
        }
    }

    /// A binary frame fills `bytes` alone, and a delivery leaves both payload
    /// readers `null`: each reader answers `null` for the kind a message is
    /// not, which is what lets a loop test one reader and fall through to the
    /// next. The test above pins the other half, a text frame's `text`.
    // covers: Core\Socket\Message::text, Core\Socket\Message::bytes, Core\Socket\Message::value
    // covers: Core\Socket\Message::topic
    #[test]
    fn each_message_reader_answers_null_for_the_kind_a_message_is_not() {
        let peer = Peer::default();
        peer.0
            .borrow_mut()
            .incoming
            .push_back(Ok(Some(PeerFrame::Binary(vec![0xde, 0xad]))));
        let mut ctx = connected(&peer);
        assert!(
            ctx.deliver(Delivery::new("prices", Value::int(3)))
                .is_none(),
            "an empty queue takes the first delivery"
        );
        let null_tag = Value::null().tag_byte();

        let conn = nvs_runtime::call(nvs_core_socket_current, &mut ctx, &[])
            .expect("a connection isolate answers `current()`");

        let delivery = receive_on(&mut ctx, conn);
        let delivered_text = nvs_runtime::call(nvs_core_socket_message_text, &mut ctx, &[delivery])
            .expect("a delivery answers `text`");
        assert_eq!(
            delivered_text.tag_byte(),
            null_tag,
            "a delivery carried text"
        );
        let delivered_bytes =
            nvs_runtime::call(nvs_core_socket_message_bytes, &mut ctx, &[delivery])
                .expect("a delivery answers `bytes`");
        assert_eq!(
            delivered_bytes.tag_byte(),
            null_tag,
            "a delivery carried bytes"
        );

        let frame = receive_on(&mut ctx, conn);
        let octets = nvs_runtime::call(nvs_core_socket_message_bytes, &mut ctx, &[frame])
            .expect("a binary frame carries its payload");
        assert_eq!(octets.as_bytes(), Some(&[0xde_u8, 0xad][..]));
        let no_text = nvs_runtime::call(nvs_core_socket_message_text, &mut ctx, &[frame])
            .expect("a binary frame answers `text`");
        assert_eq!(
            no_text.tag_byte(),
            null_tag,
            "a binary frame answered as text"
        );
        let no_value = nvs_runtime::call(nvs_core_socket_message_value, &mut ctx, &[frame])
            .expect("a peer frame answers `value`");
        assert_eq!(
            no_value.tag_byte(),
            null_tag,
            "a peer frame carried a value"
        );
        let no_topic = nvs_runtime::call(nvs_core_socket_message_topic, &mut ctx, &[frame])
            .expect("a binary frame answers `topic`");
        assert_eq!(
            no_topic.tag_byte(),
            null_tag,
            "a binary frame was reported as a delivery"
        );

        for value in [
            delivered_text,
            delivered_bytes,
            octets,
            no_text,
            no_value,
            no_topic,
            delivery,
            frame,
            conn,
        ] {
            dropped(value);
        }
    }

    /// § 3's `null`: an orderly close ends the loop rather than throwing, which
    /// is what makes `while (var $msg = $conn->receive())` the whole of a
    /// connection script's control flow.
    // covers: Core\Socket::receive
    #[test]
    fn receive_answers_null_when_the_peer_closes() {
        let peer = Peer::default();
        peer.0.borrow_mut().incoming.push_back(Ok(None));
        let mut ctx = connected(&peer);

        let conn = nvs_runtime::call(nvs_core_socket_current, &mut ctx, &[])
            .expect("a connection isolate answers `current()`");
        let closed = receive_on(&mut ctx, conn);
        assert!(
            closed.as_text().is_none() && closed.obj_ptr().is_none(),
            "a closed peer answered with a message"
        );
        dropped(conn);
    }

    /// § 3's "throws on the send timeout rather than waiting forever", which is
    /// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s rule that no outbound wait has an unbounded spelling. The
    /// wait itself is the framing layer's, so what this crate owes is that the
    /// failure arrives as a *throw* at the call site — catchable, with the
    /// connection still the program's to close — and never as a fatal error.
    // covers: Core\Socket::send, Core\Socket::sendBytes
    #[test]
    fn send_throws_on_the_send_timeout_rather_than_waiting() {
        let peer = Peer::default();
        peer.0.borrow_mut().refuse_send =
            Some("the send wait of 10s expired with the frame unbuffered".to_owned());
        let mut ctx = connected(&peer);

        let conn = nvs_runtime::call(nvs_core_socket_current, &mut ctx, &[])
            .expect("a connection isolate answers `current()`");
        let payload = Value::str(NvsStr::new(b"ack"));
        nvs_runtime::call(nvs_core_socket_send, &mut ctx, &[conn, payload])
            .expect_err("a send that could not buffer is a throw");
        let reported = ctx
            .pending()
            .expect("the throw is on the context")
            .into_owned();
        assert!(
            reported.contains("expired") && reported.contains("Core\\Socket::send"),
            "the throw did not name the wait that expired: {reported}"
        );
        assert!(
            peer.0.borrow().sent.is_empty(),
            "a refused send buffered a frame anyway"
        );

        // The binary twin takes the same wait and reports it the same way,
        // which is what says the two rows are one operation and not two.
        let octets = Value::bytes(NvsStr::new(b"ack"));
        nvs_runtime::call(nvs_core_socket_send_bytes, &mut ctx, &[conn, octets])
            .expect_err("a binary send that could not buffer is a throw");

        dropped(payload);
        dropped(octets);
        dropped(conn);
    }

    /// The two `send` rows are the two payload kinds on the wire: text goes out
    /// as a text frame and bytes as a binary frame, in the order they were
    /// sent, and neither is rewritten on the way.
    // covers: Core\Socket::send, Core\Socket::sendBytes
    #[test]
    fn send_writes_a_text_frame_and_send_bytes_a_binary_frame_in_order() {
        let peer = Peer::default();
        let mut ctx = connected(&peer);

        let conn = nvs_runtime::call(nvs_core_socket_current, &mut ctx, &[])
            .expect("a connection isolate answers `current()`");
        let text = Value::str(NvsStr::new("héllo".as_bytes()));
        nvs_runtime::call(nvs_core_socket_send, &mut ctx, &[conn, text])
            .expect("a peer that reads takes a text frame");
        let octets = Value::bytes(NvsStr::new(&[0x00, 0xff, 0x80]));
        nvs_runtime::call(nvs_core_socket_send_bytes, &mut ctx, &[conn, octets])
            .expect("a peer that reads takes a binary frame");

        assert_eq!(
            peer.0.borrow().sent,
            [
                PeerFrame::Text("héllo".to_owned()),
                PeerFrame::Binary(vec![0x00, 0xff, 0x80]),
            ],
            "the frames did not reach the peer as sent"
        );
        dropped(text);
        dropped(octets);
        dropped(conn);
    }

    /// A socket that failed under a wait is a throw as well, and for the same
    /// reason: § 3 tears a connection down through `rule:errors/escalation-ladder`'s ladder, so the
    /// script gets to see what happened before the isolate ends.
    // covers: Core\Socket::receive
    #[test]
    fn receive_reports_a_failed_socket_as_a_throw() {
        let peer = Peer::default();
        peer.0.borrow_mut().incoming.push_back(Err(PeerError::new(
            "the peer sent a frame RFC 6455 forbids",
        )));
        let mut ctx = connected(&peer);

        let conn = nvs_runtime::call(nvs_core_socket_current, &mut ctx, &[])
            .expect("a connection isolate answers `current()`");
        nvs_runtime::call(nvs_core_socket_receive, &mut ctx, &[conn])
            .expect_err("a failed socket is a throw");
        let reported = ctx
            .pending()
            .expect("the throw is on the context")
            .into_owned();
        assert!(
            reported.contains("RFC 6455"),
            "the throw did not carry what the framing layer said: {reported}"
        );
        dropped(conn);
    }

    /// Every § 3 member refuses a context that was handed no socket, and all
    /// three say so the same way — an ordinary request, a `spawn script` child
    /// and a command each reach this, because none of them is a connection.
    // covers: Core\Socket::current, Core\Socket::receive, Core\Socket::send
    #[test]
    fn the_connection_members_refuse_a_context_with_no_peer() {
        let mut ctx = Ctx::buffered();
        nvs_runtime::call(nvs_core_socket_current, &mut ctx, &[])
            .expect_err("a program that is not a connection has no socket");
        let reported = ctx
            .pending()
            .expect("the throw is on the context")
            .into_owned();
        assert!(
            reported.contains("needs a connection"),
            "the refusal did not name what was missing: {reported}"
        );

        // The other two are reached with a handle built by hand, which is the
        // shape a program holds after a `current()` on the connection it
        // really is — the refusal is about the *context* and not about the
        // receiver.
        let conn = crate::instance::build(&CLASS, []);
        nvs_runtime::call(nvs_core_socket_receive, &mut ctx, &[conn])
            .expect_err("there is no peer to wait on");
        let payload = Value::str(NvsStr::new(b"ack"));
        nvs_runtime::call(nvs_core_socket_send, &mut ctx, &[conn, payload])
            .expect_err("there is no peer to send to");
        dropped(payload);
        dropped(conn);
    }

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

    /// A context granting `script.spawn` for everything, because `rule:security/capability-check-at-the-door`'s
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
    /// the third only a first-class callable has — `rule:security/isolate-shares-nothing`'s parameter names,
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
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_closure(id);
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

    /// `rule:concurrency/a-connection-is-a-root-isolate` gives the connection the slot and § 2's member fills it, so
    /// what this asserts is the hand-over itself: after the call the slot holds
    /// the resolver's program for the path that was *written*, and the argument
    /// beside it is a **copy** rather than the request's own graph.
    ///
    /// The copy is the half that could go wrong silently. `rule:classes/graph-copy`'s
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

        let path = Value::str(NvsStr::new(b"/sockets/chat.nvs"));
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
            Some(17),
            "the slot holds a program for some other path"
        );
        release_crossed(mine);
    }

    /// The refusal that makes this member callable only where `rule:concurrency/a-connection-is-a-root-isolate`'s
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

        let path = Value::str(NvsStr::new(b"/sockets/chat.nvs"));
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
    /// never an upgrade that fails later: `rule:security/isolate-shares-nothing`'s boundary turns only what
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
        let path = Value::str(NvsStr::new(b"/sockets/chat.nvs"));
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
                reported.contains("`Core\\Socket::upgrade('/sockets/chat.nvs')`"),
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

    /// `rule:programs/path-literals-resolve-from-their-file` at the upgrade: a
    /// relative literal is joined to its file's folder while compiling, so a
    /// relative path that arrives here was built while the program ran. It
    /// throws, with a resolver installed that would have answered it and a
    /// grant that covers every path, and the slot stays empty.
    #[test]
    fn a_relative_entry_path_throws_before_the_resolver_is_asked() {
        let resolver = resolving();
        let mut ctx = granting();
        let slot = UpgradeSlot::new();
        upgradable(&mut ctx, &slot);

        let path = Value::str(NvsStr::new(b"sockets/chat.nvs"));
        nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[path, Value::null()])
            .expect_err("a relative entry does not open a connection");
        let reported = ctx
            .pending()
            .expect("the failure is on the context")
            .into_owned();
        assert!(
            reported.contains("`Core\\Socket::upgrade` needs an absolute path")
                && reported.contains("Core\\Path::join"),
            "the report did not say the path is relative: {reported}"
        );
        assert!(
            !slot.is_filled(),
            "a relative path still left a program for the connection"
        );
        drop(resolver);
        release_crossed(path);
    }

    /// `rule:classes/graph-copy`'s refusal is the **parent's**, and this is where the module
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

        let path = Value::str(NvsStr::new(b"/sockets/chat.nvs"));
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

    /// `rule:concurrency/an-upgrade-is-spawn-shaped`'s two written entry forms, prepared side by side and
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
        let path = Value::str(NvsStr::new(b"/sockets/chat.nvs"));
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
            ("a path", &path_slot, path_args, 17),
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

    /// A method entry's program keeps the callable alive after the request has
    /// let go of it. The request ends before the connection starts, so the
    /// request's own reference is gone by the time the program runs.
    ///
    /// The count is asserted, and not only the answer, because reading a freed
    /// callable often still gives the right answer in a test. A program that
    /// held nothing leaves the count at one here: the caller's.
    // covers: Core\Socket::upgrade
    #[test]
    fn a_method_entry_still_runs_after_the_request_releases_its_callable() {
        let _resolver = resolving();
        let mut ctx = granting();
        let slot = UpgradeSlot::new();
        upgradable(&mut ctx, &slot);
        let entry = a_callable("room", echoes_room);
        let mine = a_room();
        nvs_runtime::call(nvs_core_socket_upgrade, &mut ctx, &[entry, mine])
            .expect("a static method entry fills the slot");
        let object = entry.obj_ptr().expect("the entry is an object");
        #[expect(
            unsafe_code,
            reason = "the test still holds its own reference, so the object is live"
        )]
        // SAFETY: the test's reference is released only below this read.
        let count = unsafe { nvs_runtime::NvsObj::refcount_of(object.cast()) };
        assert_eq!(
            count, 2,
            "the prepared program holds no reference of its own to the callable"
        );

        // The request's frame ends, and its reference goes with it.
        release_crossed(entry);
        release_crossed(mine);
        let (program, crossed) = slot.take().expect("the slot was filled").into_parts();
        let mut connection = Ctx::buffered();
        assert_eq!(
            program(&mut connection, crossed).as_int(),
            Some(7),
            "the connection did not run the entry the request named"
        );
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

        let first = Value::str(NvsStr::new(b"/sockets/chat.nvs"));
        let second = Value::str(NvsStr::new(b"/sockets/other-and-longer.nvs"));
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
            Some(17),
            "the second call overwrote the first upgrade"
        );
        release_crossed(first);
        release_crossed(second);
    }
}
