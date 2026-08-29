# Handoff

## State

**Stage 6 has its heap boundary.** [ADR 0116](../adr/0116-an-isolates-arena-is-an-ownership-root.md) is
the goal's second pre-authorized slot and is spent: an isolate's arena is an **ownership root**, not an
address range, so entering one maps nothing, "released wholesale" is one drain of `crate::release`'s
worklist (which runs the native teardown a region free would skip), and a crossing at refcount 1 is a
pointer handoff rather than a copy. The ADR's body is the rule; the plan's *Open now* summarises it.

**`crates/nvs-host/src/isolate.rs` is that ADR in code** — design.md's one `Isolate` type. A `Program` is
a boxed closure over an already-prepared unit rather than a path, because reaching the compiler from here
would link a JIT into `nvs check`; the module doc owns that and the argument/answer refusal asymmetry.
`Ctx::isolate` (`crates/nvs-runtime/src/ctx.rs:886`) is the runtime half: an isolate's statics base is its
own, where `Ctx::child`'s aliases, and that one word is the whole of "a child cannot read or write a
parent static". It is safe where `Ctx::child` is `unsafe`.

**Four of Stage 6's eight `cargo-named` names are green**, all in `isolate.rs`'s own test module:
`a_child_cannot_read_or_write_a_parent_variable_or_static`, `a_child_cannot_see_the_parents_output_buffer`,
`a_closure_a_reference_or_a_resource_is_refused_at_the_boundary` and
`an_uncaught_throw_in_a_child_leaves_the_parent_running`. 116 tests in the crate, against 110.

**The driver's acceptance check still fails, and it is still item 20's:** `examples/isolate.nvs` exits on
`E0703 — 'spawn script' is not compiled yet`. Nothing above closes it, because nothing above reaches the
language surface — the next group is where that happens.

**One anchor in the previous handoff was wrong and is corrected here.** `E0703` is *reported* by
`crates/nvs-types/src/expr/mod.rs:749`, not by `crates/nvs-ir/src/lower/expr.rs:444` — that line is the
lowering dispatch's roster **comment**, which explains the refusal but does not raise it. A group that
edits only the `nvs-ir` line will find the checker still refusing.

**Orientation gaps.** `[context] adrs` printed ADR 0023 § 2 only; ADR 0006's `## Decision`,
*What is and is not shared*, *Failure is a value* and *Output is captured by default* were all read by
hand and are what a Stage 6 slice is written against — that ADR is the goal's subject and none of it is in
the manifest. `[context] modules` still has no pattern for `nvs-types/src/expr/`, and now none for
`nvs-runtime/src/{graph,alloc,release}.rs` either.

## Next group

**`spawn script` reaches the `Isolate` that now exists.** File set: `crates/nvs-types/src/expr/mod.rs`,
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-cli/`, against `crates/nvs-host/src/isolate.rs` and
`crates/nvs-runtime/src/ctx.rs:886` (both landed, read-only for this group).

- [ ] **Build a `Program` from a path** — the half `nvs-host` may not have. `nvs-cli`'s own run path
      already compiles a file and installs a unit's statics; the slice is a resolver returning
      `nvs_host::Program` for an `examples/isolate/*.nvs`, published through a seam `nvs-ir`'s lowering
      can reach. ADR 0006 § *Executing code is its own capability* bounds it; ADR 0116 § 5 says what
      crosses. Anchors: `crates/nvs-host/src/isolate.rs:@Program`, `crates/nvs-runtime/src/host.rs`
      (the shape of an existing seam).
- [ ] **Retire `E0703`** at `crates/nvs-types/src/expr/mod.rs:749` — the *reporting* site — and lower the
      construct at `crates/nvs-ir/src/lower/expr.rs:444`'s arm, whose roster comment names it. The
      `with(…)` options are ADR 0006's `args`/`output` only for this slice; `limits` and `grants` are
      goal 3's and each site says so.
- [ ] **`examples/isolate.nvs` prints its five frozen lines**, which is the driver's failing check
      (`docs/agent/loop-goal.toml`, stage `6 isolates`). The three children under `examples/isolate/`
      already exist.

## Backlog

- Stage 6's other four `cargo-named` names: an unresolvable class at the boundary, a contained panic, a
  cancelled parent leaving no orphan, a child cancelled at its next safepoint (`docs/agent/loop-goal.toml`).
- `Core\Script::args()` — how a child reads what crossed in; ADR 0012 says it is a method, not a variable.
- `Core\Secret::reveal()`: item 18's escape hatch, open at both ends — no parameter spelling accepts a
  qualifier and `Qual::Launder` has no consumer (ADR 0033).
- `on: 'worker'` — ADR 0116 § 6 decided the crossing may not adopt across cores; the placement is unbuilt.
- M4's residue: the 1000-case conformance corpus count (`docs/implementation-plan.md`).
