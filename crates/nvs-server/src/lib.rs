//! The built-in HTTP server: a socket to a root isolate and back.
//!
//! [ADR 0097](../../../docs/adr/0097-development-server-and-proxied-origin.md)
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
//! ([ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
//! § 3). There is one isolation path in this tree and that is it; a second one
//! would leave M7's state-bleed suite proving nothing.
//!
//! **A connection is bounded by a clock**: ADR 0097 § 5's four waits arrive as
//! one `nvs_config::server::Waits`, and [`io`]'s docs § *The clock* are where
//! they are enforced — idle waits refreshed by the bytes that move, never a
//! total, and no state a connection can be in that is not one of the four.
//!
//! [`mount`] is which isolate a request selects: § 4's five steps over the table
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
//! the number the file wrote: ADR 0106 § 13 takes the smaller of `max_in_flight`
//! and what the memory budget affords against the per-request cap, because a
//! concurrency ceiling and a memory cap with no stated relationship leave the
//! out-of-memory killer as the real admission control. The valve is asked before
//! the handler is — a `503` with `Retry-After: 1` and no isolate allocated for
//! it — and that module's docs own why the order *is* the guarantee.
//!
//! What is **not** here yet is the rest of `[server]`. [`serve`]'s own docs
//! § *What this module does not decide yet* is the list.
//!
//! # Why `hyper` and not our own h1
//!
//! Framing is where request smuggling lives, and it is not a parser to own for
//! one implementation's worth of traffic. The workspace `Cargo.toml`'s comment
//! above the dependency is the argument in full — including the part that
//! matters most, which is that ADR 0051 § 4's first question answers **yes**
//! here and the answer is still to take the dependency.
//!
//! **There is no second scheduler.** `hyper` with `http1` and `server` alone
//! needs no `Executor` and `serve_connection` spawns nothing, so what drives a
//! connection is one `Future` polled on the accepting coroutine's own stack
//! ([ADR 0138](../../../docs/adr/0138-a-connection-future-is-driven-by-the-coroutine-that-owns-it.md)).
//! [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)'s
//! refusal of tokio's task primitives is untouched by that.
//!
//! `tokio` itself is nevertheless in the lock file, because `hyper` 1.11 depends
//! on it unconditionally for one `oneshot` in its upgrade path. What is compiled
//! is `features = ["sync"]` and nothing else — no `rt`, no `net`, no `time`, no
//! executor and no `spawn` — so it is a channel library here and not a runtime.
//! The workspace `Cargo.toml`'s comment above the dependency is the home of that
//! reading and of what pinning `hyper` backwards would have cost instead.

pub mod admit;
pub mod io;
pub mod mount;
pub mod secure;
pub mod serve;
pub mod statics;

pub use admit::{Admission, Ceiling, InFlight};
pub use io::{ConnectionIo, Phase};
pub use mount::{Dispatch, Existing, OnDisk, Resolved, Selection, Table, What};
pub use secure::{Scheme, Secure};
pub use serve::{Answer, Draining, Reply, Serving, serve_connection, serve_on_this_core};
pub use statics::{Source, Stat};
