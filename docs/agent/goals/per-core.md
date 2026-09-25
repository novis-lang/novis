---
milestone: M7
---
# Loop goal 23 — `nvs serve` takes every core, and the compiled unit is shared

`nvs serve` binds one socket on one core today, and no path in the process starts a second. This goal
gives the command a core count: every entry of `[server] listen` is bound, each core takes its own
handle on the descriptor, and one compiled unit is shared across all of them behind an `Arc` with a
single publisher. When it is green, `benches/serve-proxied.json`'s deployed arm scales with cores
instead of staying flat.

**It sits here because M7's own scope says so and no goal claimed it.**
[m7.md](../../plan/m7.md) puts "per-core accept and dispatch" inside the milestone; goal `server` shipped the
single-core server and closed, so this is a milestone's stated scope that nothing owned — the reason
`carried-gaps.md` carried it as `unowned` until this entry existed. It is first of
the entries added after goal `unix-sockets` because it is **the largest measured performance item in the
repository**: the proxied bench has php-fpm scaling 2.44x from one core to four while `nvs serve`
stays flat, turning a 2.92x lead into 1.19x — and inverting it on a box with more cores.

**The primitives are built and unreached.** [`nvs_host::NvsListener::from_std`](../../../crates/nvs-host/src/net.rs)
(`net.rs:336`) exists for precisely this — each core takes its own handle on the descriptor —
and [`nvs_host::Worker::spawn(cpu, …)`](../../../crates/nvs-host/src/lib.rs) (`lib.rs:273`) pins a
scheduler per core for a cost paid per *process start*, with a handshake that makes `Worker::pinned`
answerable rather than racing. Both are called only from `#[cfg(test)]`. Nothing in `nvs-cli` or
`nvs-server` reads a CPU count at all; the one `std::thread::available_parallelism` call in the tree is
`crates/nvs-cli/src/info.rs:145`, and it prints a string.

**Nothing about the design is in the way.** [design.md](../../plan/design.md)
§ *Thread-per-core, shared-nothing runtime* is this shape;
`rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s in-flight ceiling is already a
relaxed atomic "so one hot core cannot refuse while its neighbours idle", its watchdog is already per
worker, and its h2c refusal argues *from* connections being balanced across cores.

## Why here

The primitives are built and unreached: `NvsListener::from_std` and `Worker::spawn(cpu, ...)` have
only `#[cfg(test)]` callers.

## Stage 0 — the catch-up

1. **`serve.rs`'s § *Decision: one socket, and the flag is the last word*** (`crates/nvs-cli/src/serve.rs:42`)
   is the module doc stating the rule this goal replaces. It names its own successor — "binding all of
   them is `NvsListener::from_std`'s fan-out, which is the slice that gives this command a core count" —
   so it is rewritten here, not amended by an overlay. The stderr line saying what the loop *left*
   goes with it.
2. **Any fixture or bench asserting single-core behaviour.** `benches/serve-proxied.json`'s recorded
   arm is a measurement, not a fixture, and is re-recorded at stage 5 rather than edited.

## Stage 1 — the floor

Goal `warm-start`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: the compiled unit is shared, and the swap has one publisher

The cache goes first because a fan-out over a per-core `RefCell` would compile the same file once per
core, and M7's acceptance is that 10k concurrent cold requests compile it **exactly once**.

1. **`crates/nvs-cli/src/script.rs`'s cache stops being a `RefCell`.** Its module doc (`script.rs:58`)
   states the current shape — "a `RefCell` reached from" one core, sound because "nothing can observe
   this cache while a compile is running". That argument is what a second core breaks. The unit behind
   it is immutable, so it is sound to share; what needs a publisher is the *swap*.
2. **One publisher for `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s revalidate-and-swap.**
   A revalidation that wins publishes a new `Arc`; every core reads through the old one until it does,
   and a reader never blocks on a compile. `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s `validate` pick, its one-window-is-one-check
   rule and its "a fresher revalidation has not won" ordering are unchanged in meaning and re-stated
   for N readers rather than one.
3. **The compile counter counts compiles, not cores** — `script.rs:193`'s counter is what M7's
   acceptance asserts against, and its whole value here is that fanning out must not multiply it.
4. **`routes: Arc<Routes>` needs nothing** (`script.rs:113`): the route table is already handed out
   behind an `Arc` and is already immutable.

## Stage 3 — the fan-out

1. **`[server] listen` binds every entry.** `crates/nvs-cli/src/serve.rs` stops taking the first and
   dropping the rest; `--listen`/`--port` still override the file and still conflict with each other,
   on `rule:http-server/the-server-block-is-boot-class`'s own sentence.
2. **A core count, read once.** `std::thread::available_parallelism`, bounded by a `[server] workers`
   key with the count as its default, and one `Worker::spawn(cpu, …)` per core. The handshake is per
   process start, which is what `Worker::spawn`'s doc already prices.
3. **Each core takes its own handle**, via `NvsListener::from_std` on a duplicated descriptor — the
   constructor's stated purpose. The accept loop, its backoff (`rule:http-server/the-body-is-read-on-demand-under-two-caps`
   ) and the drain probe ([ADR 0017](../../decisions/0017.md) § 5) run per core.
4. **Draining is fleet-wide, not per core.** `isDraining()` answers the same on every core, and the
   process exits when the last core's in-flight count reaches zero — a request whose isolates are
   still running when the client disconnects leaves none behind, which is M7's acceptance and is now a
   claim about N cores.
5. **The Unix-domain refusal moves, and the classification does not** — `serve.rs` refuses
   `Listen::Unix` because `NvsListener` accepts on TCP alone. That is unchanged by this goal and stays
   one refusal rather than becoming N.

## Stage 4 — nothing leaks across a core

1. **The state-bleed suite runs across a core boundary** the way it already runs across an isolate
   boundary — a parameterisation of the existing suite, not a second suite, which is what the shared
   `Isolate` bought.
2. **`rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s in-flight ceiling is fleet-wide and already atomic.** The assertion is that a hot core
   cannot refuse while its neighbours idle, which is the sentence that atomic was written for.
3. **The per-worker watchdog fires per worker**, unchanged, and a stalled core does not stall the
   fleet.

## Stage 5 — the number

1. **`benches/serve-proxied.json` is re-recorded on one and four cores.** The check is that
   `nvs serve` scales, stated as a floor the bench prints rather than a figure written by hand — a
   fan-out that does not scale is a fan-out to delete, and this is what would say so.
2. **10k concurrent cold requests for the same file compile it exactly once**, asserted through
   stage 2's counter with every core accepting.

## Standing decisions

- **The compiled unit is shared behind an `Arc`, with one publisher — decided, not open.** An
  immutable unit is sound to share, and sharing is what makes M7's "compile exactly once" acceptance
  hold as written rather than becoming a per-core assertion. Per-core caches were the alternative and
  are rejected: they cost N compiles of the same file and would have made the milestone's own
  acceptance figure mean something different at four cores than at one.
- **This does not weaken shared-nothing.** [design.md](../../plan/design.md)'s rule is about *request*
  state; a compiled unit is immutable program text, which is the same exception `rule:security/isolate-shares-nothing` already
  makes when it says an isolate "shares immutable compiled code".
- **This goal may open one ADR number** for the per-core accept and the shared unit cache, and no
  second. Every other question it meets is an edit to `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s or `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s fragment, with a record whose `changes:` block names it.
- **A core count is bounded by configuration, never by a heuristic.** `[server] workers` defaults to
  the available parallelism and is the last word when written; there is no "auto" spelling that means
  something other than the default.
- **Ambiguity about where the publisher lives resolves toward `script.rs`** — it owns the cache and
  the revalidation policy already, and its module doc is where the decision is recorded.
- **What this spends**, per `rule:programs/memory-priority`: one scheduler, one
  accept loop and one listener handle per core, all per process start and O(cores) rather than
  O(requests). The unit cache holds strictly *less* than N per-core caches would.
