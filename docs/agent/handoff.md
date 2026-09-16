# Handoff

## State

**Goal `unowned-closures`, stage 2.** Two gaps closed and struck; a third left for its own session
because it needs a different file set.

`crates/nvs-runtime/src/commands.rs` gap 1: an argument declared at a subset of an enum's cases —
§ 3's `Level::Warn|Level::Error` — converts by the case's own name, exactly as the whole enum does.
`nvs_types::commands::conversion_of` filters `cases_of`'s pairs through
`nvs_types::routes::admitted_cases`, now `pub(crate)` and the one home of *which cases does this
type admit* for a route capture and a command word alike; the two still disagree about spelling and
that stays in their own rules. A union spanning **two** enums leaves no enum for a word to be a
case of and is refused where it is written, so `ArgConv::Unconverted` now means a positional
parameter at a type § 6 does not admit — see the new playbook bullet for why that arm survives.

`crates/nvs-runtime/src/array.rs` gap 1: the element-type descriptor is written as the bound it is.
Nothing reads one back — a write through `mixed` is checked against the target array's element type,
and `as array<U>` restamps at O(n) against `U` — so `ArrayHeader` carries none, and
`rule:types/arrays` is amended to promise the pointer *where something reads one*. The first real
reader is `crate::object`'s tag list, where a `Tag::Array` answers for every `array<T>`.

Stage 1's floor is goal `m8-stdlib-depth`'s whole list, carried and untouched. Nothing is blocked.

## Next group

**Stage 2: the runtime's own `Decided` list** — one file set: `crates/nvs-runtime/src/graph.rs`
with the two files its gaps reach into.

- [ ] **A closure bit on the class descriptor** — `crates/nvs-runtime/src/graph.rs:61`'s gap 1,
      whose `Decided:` sentence is "a closure bit on the class descriptor". A closure is recognized
      today by its class declaring an `invoke`, so a user class declaring one of its own is copied
      as a closure and refused; `rule:classes/graph-copy` is the walk's rule and `rule:types/declaration`
      is what makes the confusion a compile error nearly everywhere else. The bit goes on
      `crates/nvs-runtime/src/object.rs:290`'s `ClassDesc`, and both readers adopt it —
      the copy walk and `crates/nvs-runtime/src/closure.rs:186`'s `call_closure`.
- [ ] **`decode` rebuilds a `Core` instance rather than refusing it** —
      `crates/nvs-runtime/src/graph.rs:69`'s gap 2, whose `Decided:` sentence is "install a
      Core-class resolver on Ctx at boot". `rule:classes/serialize-is-a-closed-format` is the format;
      the resolver is installed the way the route and command tables are, and
      `nvs_stdlib::instance`'s table is what it asks.

## Backlog

- The route walk's linear scan, `crates/nvs-runtime/src/routes.rs:88` gap 1 — its `Decided:` is
  *measure on `benches/serve-proxied.json` first*, so it needs a large-table bench arm and a release
  run: its own session, and a file set of `benches/` and `docs/perf/`.
- `crates/nvs-runtime/src/record.rs:36` gap 1 — refusing a `secret` into an array element or shape
  field is a compile-time check in `nvs-types`, not a change to the dump.
- `crates/nvs-runtime/src/record.rs:55` gap 2 — the `Decided:` keeps the rule, so it is a prose
  strike stating the bound, like `array.rs`'s above.
- `crates/nvs-runtime/src/lib.rs:205`–`:252` — five owned gaps in one module doc.
- `crates/nvs-runtime/src/metrics.rs:110` and `crates/nvs-runtime/src/decimal.rs:54` — one each.
- `crates/nvs-cli/src/service.rs:784`'s `Notify::install` is a process-global `OnceLock`, which made
  the `sd_notify` case fail once under load here and pass on the re-run; the playbook bullet is the
  trap, and scoping the sink per case is unowned work.
