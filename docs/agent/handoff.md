# Handoff

## State

**Goal 14, stage 12 is closed: the workspace-wide async-runtime guard exists under the name the
acceptance check uses.** `the_workspace_has_no_async_runtime` at
`crates/nvs-runtime/tests/manifest_policy.rs:295` asserts the family rather than the one crate — no
manifest of ours asks for a scheduler crate in any of the four spellings (`smol = "2"`,
`smol.workspace = true`, `[dependencies.smol]`, a rename's `package = "smol"`), no crate of ours
resolves to one in the lock file, and the lock file's only scheduler crate is `hyper`'s `tokio`. The
family list is `ASYNC_RUNTIMES` at `crates/nvs-runtime/tests/manifest_policy.rs:252` and it is the
*scheduler* half of that ecosystem only: `futures-core`, `futures-util` and `async-trait` schedule
nothing and are already in the graph under `wasmtime`. `rule:concurrency/one-scheduler` is what it
pins.

**The landed `tokio_appears_in_neither_the_manifest_nor_the_lockfile` is untouched**, so the stage-1
check that names it stays green; the two tests overlap on `tokio` on purpose, the older one in depth
and this one in breadth.

**M4B no longer asserts something false.** Both the lead paragraph and *Verify* now say what those two
tests check instead of "`tokio` appears in neither `Cargo.toml` nor `Cargo.lock`", so the playbook
bullet whose `[until: gone docs/plan/m4b.md:appears in neither]` waited on that is dropped by this
wrap.

**Every goal-14 stage the ledger has named is green.** Whether the acceptance list is whole is the
driver's own check; this session ran `verify.py`, not that list.

## Next group

**The tool chapters that owe every proof, starting with the CLI's** — one file set:
`docs/reference/tools/10-cli.md`, `crates/nvs-cli/tests/`, `docs/examples/tools/cli/` and
`tests/hostile/tools/cli/`. `python tools/dossier.py --group tools:cli` is 13 features at zero, and
`rule:testing/one-slice-is-one-feature` is why a feature's three proofs land together rather than a
row at a time.

- [ ] **`tools:cli/nvs-ast` carries a test, an example and an attack** — the `covers:` marker goes on
      a case that already exercises the subcommand in `crates/nvs-cli/tests/ast.rs:1`, per
      `rule:testing/proof-attribution`; the example goes under `docs/examples/tools/cli/` and the
      attack under `tests/hostile/tools/cli/`. `crates/nvs-lsp/tests/handshake.rs:97` is the marker's
      shape as stage 11 landed it, and `rule:testing/four-proofs` is what a feature owes before
      `POLICY["tool"]` narrows it to three.
- [ ] **`tools:cli/nvs-meta-json` carries the same three** — `crates/nvs-cli/tests/meta.rs:1` is the
      binary that already covers it, so this is one marker plus the two files; the `--json` output is
      the registry, which makes the attack a malformed or oversized registry rather than a bad flag.
- [ ] **`tools:cli/nvs-check` carries the same three** — no test file is named for it, so this one
      also decides where the case lives: `crates/nvs-cli/tests/strict_docs.rs:1` is the closest
      existing home and `crates/nvs-cli/tests/fixtures` is the fixture tree it would use.

## Backlog

- The other two tool chapters owe every proof too — `docs/reference/tools/20-config.md` and
  `30-php-differences.md`, and no goal gates on them yet.
- The latency ceilings come from one machine; a second platform's figure is what a tightening pass
  would need — `crates/nvs-lsp/tests/latency.rs`'s module doc.
- `Foo::class` colours neither half; the `class` keyword is the TextMate layer's —
  `crates/nvs-lsp/src/semantic.rs`'s module doc.
- The matrix is a ratchet now: a case at a construct nobody reached obliges six more rows —
  `crates/nvs-lsp/src/coverage.rs:46`.
