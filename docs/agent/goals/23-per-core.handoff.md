# Handoff

## State

**Goal 23 — `nvs serve` takes every core — has just started; nothing of it has landed yet.** Goal 22's
whole list is this goal's Stage 1 floor.

This is M7's own stated scope, shipped around: goal 6 built the single-core server and closed, and
until this entry existed no `[[goal]]` owned "per-core accept and dispatch". It is **the largest
measured performance item in the repository** — `benches/serve-proxied.json`'s deployed arm has php-fpm
scaling 2.44x from one core to four while `nvs serve` stays flat, turning a 2.92x lead into 1.19x.

**The primitives are built and unreached.** `nvs_host::NvsListener::from_std`
(`crates/nvs-host/src/net.rs:336`) exists for this exact fan-out; `nvs_host::Worker::spawn(cpu, …)`
(`crates/nvs-host/src/lib.rs:273`) pins a scheduler per core for a cost paid per process start. Both
have only `#[cfg(test)]` callers, and nothing in `nvs-cli` or `nvs-server` reads a CPU count at all.

**The design question is already decided** and is not reopened: the compiled unit is **shared behind an
`Arc` with one publisher**, not per core. An immutable unit is sound to share, and sharing is what
keeps M7's "10k cold requests compile it exactly once" acceptance meaning the same thing at four cores
as at one.

## Next group

**Stage 2: the shared unit** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-config/src/cache.rs`.

- [ ] **The cache stops being a `RefCell`** — `crates/nvs-cli/src/script.rs:58`'s module doc states the
      current shape and the argument that a second core breaks ("nothing can observe this cache while a
      compile is running"). Rewrite it, don't overlay it.
- [ ] **One publisher for `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s revalidate-and-swap** — a revalidation that wins publishes a new
      `Arc`; readers never block on a compile; § 3a's `validate` pick and the "a fresher revalidation
      has not won" ordering are restated for N readers rather than one.
- [ ] **The compile counter counts compiles, not cores** — `script.rs:193`. This is what stage 5
      asserts against, and multiplying it is the failure the shared cache exists to prevent.
- [ ] `routes: Arc<Routes>` (`script.rs:113`) already shares correctly and needs nothing.

## Backlog

- **Stage 3 (the fan-out)** is its own file set — `crates/nvs-cli/src/serve.rs`,
  `crates/nvs-config/src/server.rs`, `crates/nvs-server/src/serve.rs` — and rewrites `serve.rs:42`'s
  § *Decision: one socket, and the flag is the last word*, which names its own successor. A `[server]
  workers` key defaults to `available_parallelism`; the Unix-domain refusal stays one refusal.
- **Stage 4 (nothing leaks across a core)** parameterises the existing state-bleed suite by core rather
  than adding a second one — the shared `Isolate` is what makes that a parameterisation.
- **Stage 5 (the number)** re-records `benches/serve-proxied.json` on one and four cores. The threshold
  is written from what the bench prints, not by hand.
