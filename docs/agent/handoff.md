# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `crates/nvs-types/`.** `python
tools/owners.py --untagged` names nothing in that crate now; the tree-wide roster is 44 tags, every
one of which resolves to a live goal, a milestone that is not `done`, or an `unowned` with its reason
written, and 112 items still naming nobody in the other crates.

**Four of the nine items read this session were not gaps.** `crates/nvs-types/src/layout.rs`'s two
bullets (an enum has no entry, only the classes in the files walked are present) and
`crates/nvs-types/src/commands.rs`'s one item (a row carries the *conversion* a parameter needs, not
its type) each state a settled design with a rule or an argument behind it, so they are body sections
above the heading now rather than entries in a list of what is owed — the goal's § *Standing
decisions* resolves that ambiguity toward *decision*. `crates/nvs-types/src/error_lib.rs`'s two
closing paragraphs moved for the same reason, and moving them is what made its tag register.

**This crate carries the first milestone tags.** `links.rs` gap 2 is `M7`: what segment text reaches
an enum case is `Core\Router::match`'s decision and that member lands there (`docs/plan/m7.md:82`).
`derive.rs` gaps 1 and 2 are `M8`, which carries both `Core\Db` — and so `Core\Db\Row`, which
`fromRow` needs — and `Core\Json`, whose missing decoders are why a `decimal` or an `Instant` field
still erases to `CodecTy::Opaque` (`docs/plan/m8.md:79-85`).

**[carried-gaps.md](carried-gaps.md) § *Unowned* is twelve entries**, two of them new: whether
`rule:core-classes/derive-attribute`'s run-time third moves to compile time, and how much of a
`mixed` value's tag dispatch is written at once. `python tools/chain.py --check` is green — it was
red only on the previous handoff quoting the plan's chain numbers, and the paragraph under the plan's
milestone table names slugs now instead. Stage 4 is untouched: `verify.py` still does not run the
owners gate, so the acceptance check is its only reader.

**`python tools/verify.py` is green through fmt, lints, build, 3829 tests, the two `.nvst` trees and
the reference, and red at clippy for a reason no session wrote:** an untracked
`benches/serve-probe/Cargo.toml` with no `src/` is a workspace member cargo cannot load, and it
appeared between the build step and the clippy step of that run. Nothing in this session is Rust.
Either finish that crate or drop the directory before reading a clippy failure as a regression.

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-cli/src/`'s three
blocks and then the two single-item blocks in the crates beneath it. The kinds are the goal's
§ *Standing decisions*, the three owner kinds are `python tools/owners.py --help`, and `--untagged`
is the worklist.

- [ ] **Tag `crates/nvs-cli/src/bundle.rs:47`'s two remaining items.** Gap 1 of that block already
      names an owner, so the file shows the shape to match; both are
      `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s, and gap 3 (a
      bundled process resolves `./nvs.toml` from its working directory) may be a decision rather
      than a gap. Check each against the code first.
- [ ] **Tag `crates/nvs-cli/src/cache.rs:154` and `crates/nvs-cli/src/worker.rs:74`**, one item
      each — a wired producer and a dead-lettered row keeping only the last attempt's error. The
      queue half is `rule:config/scheduled-work-is-a-config-block`'s milestone, so check whether
      `M7`'s plan or a live goal claims it before reaching for `unowned`.
- [ ] **Tag `crates/nvs-hir/src/lib.rs:48` and `crates/nvs-ir/src/ty.rs:16`.** Both are scope
      statements — "covers what M2's paragraph names", "scoped to exactly what's lowered so far" —
      and both read as decisions rather than gaps; if they are, they move above the heading and the
      blocks go away.

## Backlog

- `crates/nvs-ir/src/ty.rs`'s `Ty::Tagged` variant carries its own `# Known gap`, which `owners.py`
  never sees because it reads module blocks only. It is the same gap the new tag-dispatch entry in
  `docs/agent/carried-gaps.md` names.
- `crates/nvs-types/src/expr/quals.rs:25` says § 5's auto-escape and `Markup + Markup` wait on
  `Core\Html` existing; both exist (`crates/nvs-types/src/expr_table.rs:734`). Stale prose, not a
  tagged gap.
- `crates/nvs-types/src/lib.rs:1` opens as a changelog — "M2's last open thread … this one adds" —
  which `AGENTS.md` rule 6 forbids. Left alone; it is not in the gap block.
- `crates/nvs-types/src/lib.rs`'s `# Layout` list names eleven of the crate's modules and skips
  `intrinsics`, `links`, `reasons`, `commands`, `core_lib` and `error_lib`.
- Stage 4 is unstarted: `verify.py` still does not run `owners.py`
  (`docs/agent/loop-goal.toml`).
- This clone has no hooks: `git config core.hooksPath tools/git-hooks`, which is what rejects an
  attribution trailer that arrives past `session.py` (`docs/agent/conventions.md`).
