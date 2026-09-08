# Handoff

## State

**Goal 16 — stage 2 is two checks of three green.** `nvs-test` now pins all six tests the first check
names, and `nvs-cli` both of the second's. What is left of the stage is the third check: the three
`.nvst` cases under `tests/conformance/core/` that answer a request end to end. Nothing is blocked.

**The whole crossing already works at the built binary**, so the remaining cases are authoring and not
implementation: `./target/debug/nvs.exe run --request <file> <program>` answers `Core\Request::path`,
`query`, `header`, `cookie`, `body` and `method` off the file, and refuses with `LogicError` where no
file was named (`crates/nvs-cli/tests/fixtures/request/reads-the-request.nvs:1`, run both ways this
session). The runner writes `request.nvsr` into the case's workdir at `crates/nvs-test/src/run.rs:193`
and passes `--request` at `crates/nvs-test/src/run.rs:334`, so a case that writes a request section
needs no runner change at all.

## Next group

**Stage 2: a `.nvst` case answers a request** — one file set: `tests/conformance/core/`, over the
runner path already on disk. Each case is `--RUN--` `run` by default, which is the only subcommand that
takes a request (`crates/nvs-test/src/case.rs:577`), and **never `--ORACLE--`** — conformance runs
where there is no PHP.

- [ ] **`tests/conformance/core/a-request-reads-the-body-a-post-raw-section-sent.nvst`** — a
      `--POST_RAW--` case echoing `Core\Request::body()`. The body is the rest of the request file
      verbatim, its trailing newline included (`crates/nvs-test/src/request.rs:117`), so `--EXPECT--`
      carries that newline; `crates/nvs-test/src/request.rs:276` is the test that pins it.
      `rule:testing/nvst-is-separate` is the rule.
- [ ] **`tests/conformance/core/a-request-reads-a-field-a-post-section-sent.nvst`** — a `--POST--`
      case reading one pair back. The `post` row is `crates/nvs-stdlib/src/request.rs:329`; **not
      checked** whether it answers at runtime, unlike `query`, which this session ran. If it does not,
      the case is still the right one to write and the gap is stdlib's, not the format's.
- [ ] **`tests/conformance/core/a-request-reads-a-header-and-a-cookie-a-section-sent.nvst`** — a
      `--HEADERS--` plus `--COOKIE--` case. Both members answer today, and the spellings that compile
      are in `crates/nvs-cli/tests/fixtures/request/reads-the-request.nvs:12`; `--COOKIE--`'s pairs
      arrive as the one joined `cookie` field (`crates/nvs-test/src/request.rs:92`).

## Backlog

- The plan's conformance count rises by three when those cases land — `docs/implementation-plan.md`.
- `Core\Request::method()` compares with `==` and never `===`, which `E0232` refuses — a case echoing
  the verb needs the `if` form, not a cast. `docs/rules/` owns the operator.
- Stage 3 and later of goal 16 are untouched — `docs/agent/loop-goal.toml`.
