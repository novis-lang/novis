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
//! [`io`] and [`serve`]. The first is the seam between a library that is
//! `async` and a runtime that is not: `hyper`'s two IO traits over
//! [`nvs_host::NvsTcp`], driven by [`nvs_host::block_on()`] on the coroutine that
//! owns the connection. The second is the socket on either end of it — accept,
//! one child task per connection, one connection future per task — and it
//! answers a request with whatever its caller's handler returns.
//!
//! What is **not** here yet is the mount table, the response policy and the
//! request itself: a handler cannot park, so the request that runs Novis code is
//! ADR 0006's isolate and is the slice after these. [`serve`]'s own docs § *What
//! this module does not decide yet* is the list, and it includes the one that
//! matters most — a connection carries no deadline, so nothing user-reachable
//! starts this loop until it does.
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

pub mod io;
pub mod serve;

pub use io::ConnectionIo;
pub use serve::{Answer, serve_connection, serve_on_this_core};
