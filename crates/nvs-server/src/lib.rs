//! The built-in HTTP server: a socket to a root isolate and back.
//!
//! `rule:http-server/two-deployments-and-nothing-a-proxy-owns`
//! is this crate's specification, and its § 2 is the rule everything else here
//! is built to keep: **a filesystem path is never derived from a URL at request
//! time.** A request *selects* an entry point from a table expanded against
//! disk at boot; it never constructs one. Nothing in this crate resolves a path
//! from request bytes, and the test that says so is the goal's Stage 9 set
//! equality rather than a suite of attempted escapes.
//!
//! # What is here so far
//!
//! [`io`], [`serve`] and [`mount`]. The first is the seam between a library that is
//! `async` and a runtime that is not: `hyper`'s two IO traits over
//! [`nvs_host::NvsTcp`], driven by [`nvs_host::block_on()`] on the coroutine that
//! owns the connection. The second is the socket on either end of it — accept,
//! one child task per connection, one connection future per task — and it
//! answers a request by **running** it: its caller's handler names the isolate
//! a request is, [`nvs_host::Isolate`] runs it as a child of the connection's
//! own task, and what that isolate echoed is the response body
//! (`rule:tooling/echo-always-has-a-sink`
//! ). There is one isolation path in this tree and that is it; a second one
//! would leave M7's state-bleed suite proving nothing.
//!
//! **A connection is bounded by a clock**: `rule:http-server/the-server-block-is-boot-class`'s waits arrive as
//! one `nvs_config::server::Waits`, and [`io`]'s docs § *The clock* are where
//! they are enforced — idle waits refreshed by the bytes that move, never a
//! total, and no state a connection can be in that is not one of them.
//!
//! [`mount`] is which isolate a request selects: § 4's steps over the table
//! `nvs_config::mount::expand` walked against the disk at boot, answering either
//! the file to run or the file to send. It is the only module here that touches a
//! filesystem at all, and its own docs § *What a remainder may be* are why doing
//! so keeps § 2 rather than spending it.
//!
//! [`statics`] is the other half of that answer: the file § 4 step 3 chose,
//! turned into a response under that section's own paragraph — the exact file
//! and never a listing, `no-cache` with a strong `ETag` over `(size,
//! mtime_nanos)`, one `Range` and a refused multi-range, a fixed extension
//! table. **One policy in both deployments**, because a second one is a second
//! security model: nothing in that module takes a mode or a switch, so the
//! `[server]` switches decide only whether step 3 runs.
//!
//! [`admit`] is § 5's in-flight ceiling, and it is an **arithmetic** rather than
//! the number the file wrote: `rule:http-server/admission-is-arithmetic-not-a-number` takes the smaller of `max_in_flight`
//! and what the memory budget affords against the per-request cap, because a
//! concurrency ceiling and a memory cap with no stated relationship leave the
//! out-of-memory killer as the real admission control. The valve is asked before
//! the handler is — a `503` with `Retry-After: 1` and no isolate allocated for
//! it — and that module's docs own why the order *is* the guarantee.
//!
//! [`route`] is `rule:routing/matched-once-before-the-handler`'s match: the selected unit's own route table
//! against the mount-stripped path, taken **once** and written onto the request
//! rather than left for the program to ask a second time. It dispatches
//! nothing — a name and typed parameters, and then it stops — which is why the
//! CSRF check, the `route` metric label and § 8's access decision can each read
//! one answer instead of each making its own.
//!
//! [`socket`] is where this crate stops speaking HTTP:
//! `rule:concurrency/a-connection-is-a-root-isolate`'s
//! upgrade, framed. [`serve`] answers the opening handshake `101` and takes the
//! connection back off `hyper`; this module puts RFC 6455 over the same
//! descriptor and hands the result to the connection's root isolate as
//! [`nvs_runtime::PeerSocket`], which is the one type either side names. The
//! codec is synchronous because [`nvs_host::NvsTcp`] parks rather than blocks,
//! so a connection isolate's loop is straight-line code and not a second
//! `async` seam.
//!
//! [`nvs_runtime::sse`] is the other door's framing, and it is the opposite
//! shape: a function over bytes, holding no descriptor and no clock, because
//! an event is framed by whoever has the payload and written by whoever has
//! the body. It lives one crate down for that reason — the half with the
//! payload is a `Core\Sse` member in `nvs_stdlib`, which cannot reach this
//! crate, and a second copy of the only place in the workspace that writes a
//! `data:` line would be the wrong kind of duplication. Its own docs are why a
//! payload is normalized before it is split, which is what stops a program
//! escaping its own event.
//!
//! [`schedule`] is the other thing this core runs, and it is a **second task on
//! the same scheduler** rather than a second scheduler: `rule:config/a-scheduled-run-is-a-root-isolate`'s ticker,
//! sleeping until the soonest `[[schedule]]` fire and spawning each one as a
//! root isolate of its own. It takes the *how to fire* as a parameter exactly as
//! [`serve`] takes the handler, because turning a `script` path into runnable
//! code is the compiler's and this crate has none. A `fleet` entry is not armed
//! at all; that module's docs § *What is not armed* is the whole of why, and it
//! is a refusal rather than a gap.
//!
//! What is **not** here yet is the rest of `[server]`. [`serve`]'s own docs
//! § *What this loop does not decide, and who does* is the list, and each entry
//! names the module the answer belongs to rather than owing one here.
//!
//! # The `exporter` feature
//!
//! `prometheus` and `otlp` are the two modules a build of this crate can be
//! without.
//! `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` puts
//! both encoders behind the `exporter` feature, on by default, so a binary that
//! only ever runs a program from the command line carries neither the exposition
//! writer nor either pusher.
//! [`metrics`] is beside them in **every** build, because a `Core\Metrics` call
//! that compiled in one build and not another would make the `Core` namespace
//! conditional; a build without the feature still accumulates every series and
//! has no reader for them, which is that rule's own reading of the split.
//! [`trace`] sits on the same side as [`metrics`] and for the same reason: every
//! build gives a request an id and decides whether it is recorded, and what a
//! featureless one lacks is the pusher that would ship what it derived. The
//! prose here names the modules rather than linking them, so these docs are
//! whole whichever way they were built.
//!
//! # Why `hyper` and not our own h1
//!
//! Framing is where request smuggling lives, and it is not a parser to own for
//! one implementation's worth of traffic. The workspace `Cargo.toml`'s comment
//! above the dependency is the argument in full — including the part that
//! matters most, which is that `rule:packaging/a-c-dependency-answers-two-questions`'s first question answers **yes**
//! here and the answer is still to take the dependency.
//!
//! **There is no second scheduler.** `hyper` with `http1` and `server` alone
//! needs no `Executor` and `serve_connection` spawns nothing, so what drives a
//! connection is one `Future` polled on the accepting coroutine's own stack
//! (`rule:concurrency/one-future-per-connection`).
//! `rule:concurrency/one-scheduler`'s
//! refusal of tokio's task primitives is untouched by that.
//!
//! `tokio` itself is nevertheless in the lock file, because `hyper` 1.11 depends
//! on it unconditionally for one `oneshot` in its upgrade path. What is compiled
//! is `features = ["sync"]` and nothing else — no `rt`, no `net`, no `time`, no
//! executor and no `spawn` — so it is a channel library here and not a runtime.
//! The workspace `Cargo.toml`'s comment above the dependency is the home of that
//! reading and of what pinning `hyper` backwards would have cost instead.

pub mod admit;
pub mod body;
pub mod bounds;
pub mod control;
pub mod cors;
pub mod forwarded;
pub mod io;
pub mod metrics;
pub mod mount;
// The crate doc's § *The `exporter` feature* is why these two modules are
// conditional and `metrics` above them is not.
#[cfg(feature = "exporter")]
pub mod otlp;
#[cfg(feature = "exporter")]
pub mod prometheus;
pub mod route;
pub mod schedule;
pub mod secure;
pub mod serve;
pub mod slotted;
pub mod socket;
pub mod statics;
pub mod trace;

// The request a handler is handed, so that one can be *spelled* where it is
// written. `hyper` is this crate's dependency and deliberately not its callers'
// — `rule:packaging/a-c-dependency-answers-two-questions`'s answer is one crate owning h1 — but the parameter type of a
// `Fn(Request<Incoming>, Origin) -> Reply` has to be nameable outside it — the
// second parameter is `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s walk, which ran on the connection before
// the handler because its answer decides policy on responses no handler sees —
// and a handler
// that cannot annotate its own parameter is one whose first statement decides
// what it is.
pub use hyper::Request;
pub use hyper::body::Incoming;

pub use admit::{Admission, Ceiling, InFlight};
pub use body::{Arrived, Pull, Supply};
pub use control::{Controlled, Denied, Operation, answer};
pub use cors::Cors;
pub use forwarded::{Arrival, Origin, Trusted, Unusable};
pub use io::{ConnectionIo, Phase};
pub use metrics::{Family, Histogram, Kind, Refused, Registry, Series, Value};
pub use mount::{Dispatch, Existing, OnDisk, Resolved, Selection, Table, What};
#[cfg(feature = "exporter")]
pub use otlp::{
    Endpoint, Pending, Recorded, Signal, push_queued_on_this_core, push_registry_on_this_core,
    queue,
};
#[cfg(feature = "exporter")]
pub use prometheus::{scrape, scrape_every_core, serve_scrapes_on_this_core};
pub use schedule::{Armed, Fires, Leases, Rearm, Roster, arm, tick_on_this_core};
pub use secure::{Scheme, Secure};
pub use serve::{
    Answer, Draining, Generations, Listening, Reply, Serving, serve_connection, serve_on_this_core,
};
pub use socket::{Framed, accept_key};
pub use statics::{Source, Stat};
