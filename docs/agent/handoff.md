# Handoff

## State

**Goal 6, M7, stage 6b — ADR 0006's entry rule is complete.** `spawn script Class::method(...)`
lowers to `CORE_SCRIPT_SPAWN_METHOD` with **four** arguments — the label, `args:`, `output:`, and
the entry's parameter names as a `ConstStr`, comma-separated in declaration order
(`crates/nvs-ir/src/lower/expr.rs:2985`'s `spawn_method_entry` owns the encoding) — and
`nvs_stdlib::script` binds the map's entries to those parameters by name. `E0804` is retired;
`nvs_types::expr::isolate`'s `check_entry` now refuses nothing about a method entry.

**The names come from the typed-expression table**, not from the runtime:
`nvs_types::expr_table::ResolvedCall::param_names` is new beside `param_tys`, filled in
`crates/nvs-types/src/expr/calls.rs:998`. `nvs_runtime::MethodRow` has arity and parameter tags
and never names, which is why this route exists at all.

**The judgement is split, and the split is the ADR's.** `entry_names_agree` compares the map's
keys against the entry's parameters *at the spawn*, in the parent's frame, and throws a
`LogicError` there — ADR 0006 § *Decision*'s "reported … at the spawn" for a map the compiler did
not compare. `nvs_runtime::call_static_bound` judges arity and each parameter's required tag in
the *child*, where the copied values reach a compiled callee's slots, through
`nvs_runtime::closure`'s `check_param_tags`.

**Known gap, recorded at `check_entry`:** the ADR also reports that mismatch at *compile time*
when `args:` is a literal, and nothing asks that yet. What it costs is when the error arrives,
never whether it does.

**The stage's first acceptance check is green** — five tests in
`crates/nvs-cli/tests/spawn_entry.rs`, its four plus the new spawn-time refusal.

## Next group

**ADR 0083 §§ 1-2 — the connection isolate and its entry**, over
`crates/nvs-stdlib/src/registry.rs`, a new `crates/nvs-stdlib/src/socket.rs`,
`crates/nvs-stdlib/src/lib.rs` and `crates/nvs-server/src/serve.rs`. **None of that surface
exists**: there is no `Core\Socket`, `Core\Sse` or `Core\Topic` row, and no `tungstenite`
dependency in any manifest — so the next check's eight `-p nvs-server` tests are all cause 2, and
item 1 is where the goal's standing decision on `tungstenite` gets spent.

- [ ] **`Core\Socket::upgrade` as a registry class** (ADR 0083 §§ 1-2) — the five edits of
      *A `Core` member*, in a new `socket.rs` registered at
      `crates/nvs-stdlib/src/registry.rs:1224` and reached at `crates/nvs-stdlib/src/lib.rs:348`.
      The operand is `spawn script`'s, so the row's first parameter is the entry, not a callable.
- [ ] **The entry rule at its second site** (ADR 0083 § 2) — `check_entry` is the rule's one home
      (`crates/nvs-types/src/expr/isolate.rs:206`) and an `upgrade` operand has to reach it rather
      than grow a second copy; `crates/nvs-types/src/expr/isolate.rs:122` is where `spawn script`'s
      own operand reaches it.
- [ ] **Where the upgrade hands the connection to a root isolate** (ADR 0083 § 1) — decide and
      record it against `crates/nvs-server/src/serve.rs:540`, whose doc at
      `crates/nvs-server/src/serve.rs:470` already states the request-ends-here rule the ADR needs
      inverted for a connection that outlives it.

## Backlog

- ADR 0006's compile-time half for a *literal* `args:` — `nvs_types::expr::isolate`'s known gap.
- `[context] adrs` printed **no ADR 0083 section** this session and the next group is all 0083;
  add §§ 1-3, 4 and 7 — `docs/agent/loop-goal.toml`.
- ADR 0083 § 4's `Core\Topic` and § 7's bounds are two further checks in the same stage —
  `docs/agent/loop-goal.toml` stage 6b.
