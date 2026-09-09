# Handoff

## State

**Goal 23 — `nvs serve` takes every core. Stage 2 has started; its first slice is on disk.** Goal
22's whole list is this goal's Stage 1 floor and is green.

**What landed:** the compiled-unit cache's interior mutability is the shape a fleet needs.
`Compiler`'s two maps are `RwLock`s and its compile counter is an `AtomicU64`
(`crates/nvs-cli/src/script.rs:188`), neither guard is ever held across a compile, and the module
doc at `crates/nvs-cli/src/script.rs:42` states the two known gaps that are left instead of the
old "a single core collapses the concurrent half" argument. Nothing else in stage 2 can land
before those gaps close.

**What blocks the rest of stage 2, and it is not a design question:** `nvs_codegen::Unit` is
neither `Send` nor `Sync`, so `Compiled` cannot hold `Arc`s and the cache cannot be reached from a
second core at all. That is what `a_compiled_unit_is_read_by_every_core_through_one_arc` needs
first. The standing decision ("shared behind an `Arc`, one publisher") is unchanged — this is its
mechanism, and it lives in `nvs-codegen` and `nvs-runtime` rather than here. The playbook bullet
above lists every field that makes it so.

**Single-flighting is a later slice on purpose.** With one accepting core, `CompileState::Compiling`
and step 4's "a fresher revalidation has not won" compare are both unreachable, so writing them
before the fan-out would be untestable code. They land with the `Arc`, not before it.

## Next group

**Stage 2: a compiled unit crosses a core boundary** — one file set:
`crates/nvs-codegen/src/lib.rs`, `crates/nvs-runtime/src/object.rs`,
`crates/nvs-runtime/src/ctx/isolate.rs`, `crates/nvs-runtime/src/ctx/error.rs`.

- [ ] **`Unit`'s two `Rc`s become `Arc`s** — `crates/nvs-codegen/src/lib.rs:320` (`classes`) and
      `crates/nvs-codegen/src/lib.rs:345` (`statics`), with the two signatures that take them:
      `crates/nvs-runtime/src/ctx/isolate.rs:163`'s `install_statics` and
      `crates/nvs-runtime/src/ctx/error.rs:69`'s `ErrorClass::table`. What an isolate may share is
      `rule:security/isolate-shares-nothing`'s "immutable compiled code"; the cost is one atomic
      increment per isolate install, per `rule:programs/memory-priority`.
- [ ] **The raw pointers get their `unsafe impl Send + Sync`, or a reason they cannot** —
      `crates/nvs-runtime/src/object.rs:1046` (`ClassTable`, whose `ClassDesc` at `:259` holds
      `Vec<*const ClassDesc>` into boxes the table itself owns) and
      `crates/nvs-codegen/src/lib.rs:308` (`Unit`, whose `entries` are addresses in its own
      mapping). The argument is that neither is mutated after
      `crates/nvs-codegen/src/lib.rs:905`'s `into_unit`; `Code::Placed(Box<dyn Placed>)` at
      `crates/nvs-codegen/src/lib.rs:371` needs the same on the trait. This is the goal's one
      permitted ADR number if it wants a record.
- [ ] **Then `Compiled` holds `Arc`s and the cache publishes** — `crates/nvs-cli/src/script.rs:124`,
      which is what `a_compiled_unit_is_read_by_every_core_through_one_arc` and
      `a_reader_holding_the_old_unit_keeps_answering_until_it_drops_it` assert. `CompileState`'s
      third state and step 4's compare (`crates/nvs-cli/src/script.rs:340`) become reachable and
      land with it, for the other three stage-2 checks.

## Backlog

- **Stage 3 (the fan-out)** is its own file set — `crates/nvs-cli/src/serve.rs`,
  `crates/nvs-config/src/server.rs`, `crates/nvs-server/src/serve.rs` — and rewrites `serve.rs:42`'s
  § *Decision: one socket, and the flag is the last word*, which names its own successor. A `[server]
  workers` key defaults to `available_parallelism`; the Unix-domain refusal stays one refusal.
- **Stage 4 (nothing leaks across a core)** parameterises the existing state-bleed suite by core
  rather than adding a second one.
- **Stage 5 (the number)** re-records `benches/serve-proxied.json` on one and four cores.
- The goal's `[context] modules` now names `nvs-codegen`'s and `nvs-runtime`'s three files, so the
  next pack prints them; nothing else was missing from the pack this session.
