# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running, and its acceptance list can be measured
again.** The driver's `begin` gate walks the goal's `files` list before any check; all eight of goal 4's
new fixtures were missing, so iteration 0007 measured one check and nothing else. They are now on disk.
Every one of them fails its own `exact` check today, which is the intended state: a fixture is the
program its stage will make compile, and the frozen thing is its output, not its source.

**Stage 0's compile-time half landed.** `nvs_stdlib::regex::validate` returns a `Tier` rather than
`Result<(), String>`, the ADR 0057 fold at `crates/nvs-types/src/intrinsics.rs:204` records it, and
`ExprTypeTable::regex_tier` is where `nvs-ir` will read it back — no consumer does yet, which is the
next thing that touches it. Stage 0's one remaining item is § 4's `tainted` operand.

Three fixtures owe configuration their stage must write, and none of it is written:
`examples/http.nvs` names `http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs`
needs an `[[app]]` block naming `examples/logging/handler.nvs` as the error handler; and every fixture
that reaches the world needs its `fs.*` / `process.exec` / `net.connect` grants in `nvs.toml`.

`orient.py` printed two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs` and
`crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**Stage 0's last item, then stage 2's opening** — one file set, because the registry row is what both
halves edit. Files: `crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/regex.rs`,
`crates/nvs-stdlib/src/file.rs`, `crates/nvs-types/src/expr/quals.rs`.

- [ ] **The pattern argument refuses a `tainted` operand.** ADR 0056 § 4 over ADR 0024 § 3, and the last
      of stage 0. `Core\Regex::compile`'s row is `crates/nvs-stdlib/src/regex.rs:107` and its `params`
      carry the `Qual` the checker reads; `crates/nvs-types/src/expr/quals.rs:221`'s
      `admits_tainted_argument` is where a row's qualifier is enforced. The acceptance name is
      `a_regex_pattern_argument_refuses_a_tainted_operand`, `-p nvs-types`, beside this session's
      `a_literal_patterns_tier_is_settled_while_checking` in `crates/nvs-types/tests/intrinsics.rs:190`.
- [ ] **`Core\IO` is the class spec § 14 names, and `Core\File` is its old spelling.** Goal stage 2 item
      2. The `CoreClass` is `crates/nvs-stdlib/src/file.rs:41` and the roster it sits in is
      `crates/nvs-stdlib/src/registry.rs:984`; `examples/capability.nvs` writes `Core\File::read` and
      follows the rename, as do the conformance cases that name it.
- [ ] **The members `examples/files.nvs` calls** — `readText`, `lines`, `size`, `exists`, `remove`,
      `removeDir` and `temporaryDir`, all five edits each per the conventions' *A `Core` member*, in
      `crates/nvs-stdlib/src/file.rs:41`'s class. `Core\Path::join` is already on disk. `within` is a
      slice of its own and is the stage's launderer, so it is deliberately not in this group.

## Backlog

- Stage 2's `within` launderer and `writeStream` — `docs/agent/loop-goal.md` stage 2 items 4 and 5.
- `nvs-ir` has no consumer for `ExprTypeTable::regex_tier` yet — `crates/nvs-types/src/expr_table.rs`.
- `nvs.toml` owes an `[[app]]` block per new fixture — `docs/adr/0104` § 1 and each fixture's header.
- `[context] modules` names two paths that match nothing — `docs/agent/loop-goal.toml`.
- Stage 5 owes `examples/http.nvs` a local origin to talk to — `docs/agent/loop-goal.md` stage 5.
- Stage 8 owes `Core\Ast::parse` a source whose tree is four nodes — `examples/reflect.nvs`.
