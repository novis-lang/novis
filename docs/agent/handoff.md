# Handoff

## State

**Goal 62 — both entry forms reach a worker core, and a serving core offers itself — has just started; nothing of it has landed yet.** Goal `class-scoped-types`'s whole list is this goal's Stage 1 floor.

The design is settled in ADR 0184 and in the goal's *Standing decisions*, and no record is opened: § 2
is what crosses, § 5 is which core and what bounds the set, and § *Diagnostics* is why a placement that
cannot be honoured fails the spawn instead of quietly running the child here. What a session must not
re-decide: the unit table stays `nvs-cli`'s and a `Resolver` gains no operation answering "the unit this
thread is running" — that was refused where the seam is written
(`crates/nvs-runtime/src/script.rs:71-77`). The cache is already cross-core capable;
`nvs_cli::script::Compiler` holds its two maps behind `RwLock`s and a serving fleet shares one
`Arc<Compiler>` (`crates/nvs-cli/src/serve.rs:703`), so what this goal adds is a way to reach it from a
core that has not started yet, not a second table.

## Next group

**Stage 2: the seam a started core can install** — one file set:
`crates/nvs-runtime/src/script.rs`, `crates/nvs-host/src/placed.rs`, `crates/nvs-host/src/worker.rs`,
`crates/nvs-cli/src/main.rs`.

- [ ] **A resolver that can be published rather than borrowed** —
      `crates/nvs-runtime/src/script.rs:218`'s `install` takes a `&'static dyn Resolver` and `:241`'s
      `scoped` a borrow on the installing core's own stack, so neither reaches a thread `nvs-host`
      starts for itself. Add the form a placing core writes and a started core installs on its own
      thread, leaving `resolve`'s answer and `ResolveError` exactly as they are.
      `rule:security/isolate-shares-nothing` is what bounds what may be shared this way.
- [ ] **The started core installs it** — `crates/nvs-host/src/worker.rs:745`'s `destination` is where a
      core is chosen and a scheduler thread is started for the first placement; that start is where the
      published resolver goes in, beside the reactor the inbox poke reaches.
- [ ] **`crosses` stops asking which form the entry is** —
      `crates/nvs-host/src/placed.rs:107` is `entry.is_method() && ctx.class_table().is_some()`; the
      class-table half stays, because it is a fact about the context rather than about the entry.
      Rewrite that module's `# Known gaps` and `destination_for`'s second question
      (`crates/nvs-host/src/placed.rs:94`) as what then runs, and `crates/nvs-host/src/group.rs:89`'s
      restatement with them.

## Backlog

- **Stage 3, the destination set** — `crates/nvs-host/src/worker.rs`, `crates/nvs-cli/src/serve.rs`,
  `crates/nvs-cli/src/worker.rs`: a serving core registers its inbox as it starts, so a placement under
  `nvs serve` reaches a sibling serving core round-robin rather than starting a lazily started core
  beside it. ADR 0184 § 5. Shares `worker.rs` with stage 2 and is cheap right after it.
- **The rule, in the slice that makes it wrong** — `rule:concurrency/on-worker-runs-the-child-on-another-core`'s
  last two paragraphs are written as what runs and both stop being true; amend the fragment in the
  slice that lands the behaviour, not after it.
- **`crates/nvs-runtime/src/script.rs`'s test-resolver comment** — it says a real implementor is
  `!Sync` and goes through `scoped` for that reason. What `scoped` buys is a lifetime; stage 2 makes
  the distinction load-bearing and the comment is rewritten with it.
- When this goal's last check goes green the driver takes goal `gap-zero`.
