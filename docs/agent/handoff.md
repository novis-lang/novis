# Handoff

## State

**Goal `one-type-test`: three of stage 7's four checks are green.** The gate's grep returns nothing
outside its excluded files, the retired rule id is cited nowhere, and `python tools/chain.py --check`
walks. The one check still red is `docs/rules/php-migration.json:199`, which reads
`"status": "designed"` where the gate wants `shipped`.

**That flip is gated on a hole in code, not on prose.** `$m is Core\Str` type-checks and dies at
codegen: `crates/nvs-codegen/src/emit.rs:2676` returns `CodegenError::Unsupported` when this unit
declares no descriptor for the tested class and `nvs_stdlib::class_has_instances` is false, which is
every static-only `Core` class. `rule:types/type-test` says an answer the checker can settle folds to
a constant rather than becoming a failure, and a class no value can hold an instance of is settled:
the fold belongs in the checker, leaving emit.rs's guard the workspace-bug assertion its own comment
says it is.

**The word survives on purpose only where the gate excludes it** — the token, the parser and its
test, the diagnostics table, the reject case, and the four `php-migration` rule files. Everything
else names the test `is` or cites `rule:php-migration/one-type-test`. Two reader-facing costs, both
taken deliberately and small enough not to hold the run: `30-php-differences.md`'s keyword list no
longer carries the PHP word, so a docs search for it lands on the rule chapter instead, and that
chapter's row for the operator's refusal is worded by description rather than by the literal
spelling.

## Next group

**Stage 7: the `Core`-class test that dies at codegen, then the status flip** — one file set:
`crates/nvs-types/src/expr/members.rs`, `crates/nvs-codegen/src/emit.rs`, one conformance case and
the rule's metadata.

- [ ] **Fold a test against a `Core` class that can hold no instance to `false` in the checker**,
      where `rule:types/type-test` puts a settled answer — `crates/nvs-types/src/expr/members.rs:284`
      is `testable_core_class`, the predicate that already admits exactly the names the two descriptor
      tables answer to. `$m is Core\Str` is the case that dies today.
- [ ] **The comment names a function that does not exist** — `crates/nvs-codegen/src/emit.rs:2673`
      says `expr::members::testable_class_name`; it is `testable_core_class`
      (`crates/nvs-types/src/expr/members.rs:284`). Rewrite the comment whole, per `AGENTS.md` rule 7.
- [ ] **A conformance case for the folded answer**, beside `rule:types/type-test`'s own guard
      `tests/conformance/class/an-is-test-through-an-erased-subject-answers-every-tag.nvst:1`.
- [ ] **Flip `docs/rules/php-migration.json:199` to `"status": "shipped"`** once the three above are
      green, then `python tools/rules.py --render` so the chapter carries it. This is stage 7's last
      red check.

## Backlog

- `docs/decisions/0192.md` and `docs/agent/goals/*one-type-test*` spell the refused word and sit
  outside the gate's paths on purpose: a record is frozen rationale (`AGENTS.md` § *Where to look*).
- If a migrant searching the reference for the PHP word matters more than the gate reaching
  `docs/reference`, the alternative is that one path in the exclusion list at
  `docs/agent/loop-goal.toml:11626` — a user call, not a session's.
- `docs/agent/loop-goal.toml:9618` is a carried floor check whose `want` still lists `E0497`, a code
  now retired from `crates/`; it passes today, so nothing is owed until it does not.
- `crates/nvs-cli/src/serve.rs:3913` fails under `cargo test`'s side-by-side binaries and passes
  alone: the sequence it reads opens with a stray `STOPPING=1`, which is the shared-state class
  `tools/verify.py` § *Why `test` runs its binaries side by side* describes. Nothing in this
  session's docs-only diff reaches it; it needs its own socket path rather than a fixed one.
