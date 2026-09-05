# Handoff

## State

**Goal 6, M7 — stage 8's acceptance check was misfiled and is now four checks.** All four tests
were filed `-p nvs-test`, and `crates/nvs-test/Cargo.toml` declares **no dependencies at all** on
purpose, so not one of them could ever have run there: that crate is the `.nvst` format and a
runner for it, with nothing that could build a request, a listener or a database. What implements
ADR 0079 §§ 14, 17 and 18 is `nvs_cli::runner` — its own module doc calls it the one place holding
a checked program and a runtime context in one scope — so the four now sit at `-p nvs-cli`, one
check each, and the ledger names which feature is open rather than only the first.

**ADR 0079 § 14's assertion half landed; its updater did not.**
`Core\Test::assertMatchesInline(mixed $actual, string $expected)` compares
`crates/nvs-stdlib/src/debug.rs:301`'s rendering — `Core\Debug::render`'s own text, borrowed rather
than grown — against the literal in the test body, so ADR 0092 § 5's redaction reaches a snapshot
for free and a `secret` property cannot be committed into one. Three `.nvst` cases:
the 6×6 grid, both sides of a mismatch quoted, and the redaction. **`nvs test --update` is
blocked on a fact worth not rediscovering**: splicing needs the `$expected` literal's *span*, and
no runtime record holds one — a helper is called with a value, not with the expression that built
it. The design (a compile-time table beside `nvs_types::ExprTypeTable::tests`, joined to a run's
mismatches by the expected text) is in the helper's doc comment at
`crates/nvs-stdlib/src/test.rs:1097`. Searching the source for the literal instead was refused
there: § 14's workflow starts from an empty `""`, which occurs everywhere.

The rustdoc gate is green again — `crates/nvs-cli/src/service.rs:46` links `[`unit()`]`, which
disambiguates the function from the primitive type.

## Next group

**The runner's remaining ADR 0079 sections, all three in `crates/nvs-cli/src/runner.rs`** —
`run_in_isolate` is where a case's options are read and the child's context is armed, and
`mod tests` at `crates/nvs-cli/src/runner.rs:1311` is where each check's named test goes. Take
them in this order; the first two share the same insertion point.

- [ ] **`#[Test(server: true)]` gets an ephemeral listener** — ADR 0079 § 18's second mechanism,
      at `crates/nvs-cli/src/runner.rs:721`, where the option is read and the listener has to be
      bound before the isolate starts. `crates/nvs-cli/src/serve.rs:274` is this binary's one
      existing `NvsListener::bind`, and `crates/nvs-cli/src/serve.rs:497`'s
      `serve_on_this_core` is the loop beside it. Decide there how the test learns the address —
      a `Core\Test` row is the cheap answer and `Core\Server` is the other one.
- [ ] **`#[Test(db: "test")]` runs inside a transaction the runner rolls back** — ADR 0079 § 17,
      the same option read at `crates/nvs-cli/src/runner.rs:721` and the same per-class scope at
      `crates/nvs-cli/src/runner.rs:414`. A transaction opened *inside* the test must become a
      savepoint, which is ADR 0067's closure form doing what it already does. Needs the reachable
      PostgreSQL goals 4 and 5 use.
- [ ] **`nvs test --update` splices an inline snapshot** — ADR 0079 § 14's other half. The span
      table goes beside `crates/nvs-types/src/testing.rs:245`'s `TestCase`, the flag beside
      `crates/nvs-cli/src/main.rs:234`'s `Test` subcommand, and the splice in
      `crates/nvs-cli/src/runner.rs:414`, whose `ctx` outlives every test and is where a run's
      mismatches can accumulate.

## Backlog

- `Core\Test::request` — § 18's first half, a registry row in `crates/nvs-stdlib/src/test.rs` over
  a dispatch seam `crates/nvs-server/src/route.rs` already computes. Its check is stage 8's first.
- The `[context] adrs` manifest has no ADR 0079 sections, though stage 8's checks cite §§ 14, 17
  and 18; this session sliced them by hand. Add `0079` §§ 14, 17, 18.
- A `"""` block string literal does not exist, so § 14's own example does not compile and an
  object's snapshot is written with `\$`. `docs/adr/0079` § 14 owns the spelling.
- `nvs-cli`'s `Cargo.toml` should be re-read before the db slice: the manifest excerpt this
  session read stopped before `nvs-db`/`nvs-stdlib`.
