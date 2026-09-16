# Handoff

## State

**Goal `unowned-closures`, stage 2.** `crates/nvs-ir/src/lib.rs` gap 11 is closed and struck: a `secret`
compared against a `mixed` now takes `ir::Helper::SecretEq` instead of the short-circuiting
`Helper::Identical`. `nvs_secret_eq` reads the tag it is handed — a `string`/`bytes` payload of the other
side's own tag compares with `subtle`, every other tag answers `false`, which is
`nvs_runtime::value_identical`'s own answer for that pair.
`rule:security/secret-comparison-is-constant-time` states it and gains a second guard,
`tests/conformance/lang/a-secret-compared-against-a-mixed-is-constant-time.nvst`.

Stage 1's floor is goal `m8-stdlib-depth`'s whole list, carried and untouched. Nothing is blocked, and
stage 2's other two items are untaken.

## Next group

**Stage 2: the lowering and the runtime, security first** — one file set: `crates/nvs-runtime/src/`,
with `crates/nvs-stdlib/src/` read-only for the `.expect` sweep the first item needs.

- [ ] **One allocation past the budget, to its decision** — `crates/nvs-runtime/src/budget.rs:115`'s gap 1,
      whose `Decided:` sentence is "route input-sized allocations in helpers through `affords`". The seam
      already exists and is already threaded through every one of them: `crates/nvs-runtime/src/abi.rs:255`
      `affordable` is what `crates/nvs-stdlib/src/bytes.rs:1011`, `:1033`, `crates/nvs-stdlib/src/str.rs:3237`,
      `crates/nvs-stdlib/src/arr.rs:2803` and some fifty more already call — and it refuses **only** a size
      past `isize::MAX`, never asking `crates/nvs-runtime/src/budget.rs:665`'s `affords`. So the build is
      one function, not an audit of the call sites. Two things it has to settle rather than assume: a
      `false` from `affords` has already recorded the breach, so the refusal is `rule:errors/on-limit`'s
      FATAL and not the catchable `Fault::Thrown` `affordable` returns today; and roughly fifteen callers
      spell it `.expect("nothing is unaffordable here")`
      (`crates/nvs-stdlib/src/cache.rs:4175`, `crates/nvs-stdlib/src/session.rs:1903`,
      `crates/nvs-stdlib/src/signed_cookie.rs:350`, `crates/nvs-stdlib/src/csrf.rs:401`),
      each of which becomes a contained panic the day the seam can refuse for a second reason.
      `crates/nvs-runtime/src/array.rs:1032`'s comment states the present behaviour and is rewritten with it.
- [ ] **A hooked property is reached through an erased key** — `crates/nvs-runtime/src/object.rs:3403`,
      the descriptor's property row gaining a hook marker that `nvs_object_key_get` and the erased write
      both read (goal `unowned-closures` § *Stage 2*, third bullet).

## Backlog

- Stage 2's remaining **Decided** list — `crates/nvs-runtime/src/lib.rs` gaps 1, 2, 6, 7;
  `array.rs`, `commands.rs`, `decimal.rs`, `graph.rs`, `routes.rs` — `docs/agent/loop-goal.md` § *Stage 2*.
- Stage 2's **no choice left** builds: `crates/nvs-ir/src/lib.rs` gaps 1 and 2, unless goal `m4-refusals`
  already closed them as refusal sites — check first, per the same stage.
- `crates/nvs-ir/src/lib.rs` gap 14 is stage 6's, retagged `M10` rather than built — `docs/plan/m10.md`.
