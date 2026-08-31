# Handoff

## State

**Stage 5's compile-time half is closed, ADR 0074 § 7 included.** A `post` writing `retryAttempts`
without `retryIdempotencyKey` is `E0796`: `nvs_types::expr::args`' `reject_keyless_retry`, reached
from the static-call path in `expr/calls.rs` and from nowhere else, because every member carrying
the obligation is a static one. Its own doc comment is the home for why R2 is what makes the
question answerable while compiling.

**The rule holds no copy of a spelling, and one absence in it is a decision.**
`nvs_stdlib::registry::idempotent_retry_rule` answers which member owes a key and under which two
option names; `crate::http`'s `RETRY_ATTEMPTS_OPTION`/`RETRY_KEY_OPTION` are what both the rows and
the rule read. An omitted `retryAttempts` obliges nothing — `IdempotentRetry::asks` is that
judgement's home — because no `[http.client]` figure turns retries on behind a call today; if a
config default ever lands, that is what has to move with it. `patch` is already named though no
such row exists.

**There is still no transport**, so every row ends at
`crates/nvs-stdlib/src/http.rs:629`'s refusal and `examples/http.nvs` still fails. That is what the
driver's acceptance check reports and it is this stage's ordinary state, not a regression: closing
it needs all three of the transport, `Core\Http\Response`'s slots and readers, and a spelling for
the example's environment read.

**`Core\Env` exists nowhere but in that example.** No ADR 0051 § 3 roster line, no
`docs/spec/01-core-library.md` row, no registry class — so item 3 below is a *placement* decision
(roster line, spec row, and whether reading the environment is one of ADR 0118's doors), not a
five-edit member. `Core\Config::get` reads `nvs.toml` rather than the environment, so it is not the
existing spelling either.

## Next group

**The transport, and the two things `examples/http.nvs` needs beside it.** The file set is
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-host/src/net.rs`, `crates/nvs-stdlib/src/registry.rs`
and `tests/conformance/core/`.

- [ ] **`Core\Http\Response`, its slots and its readers** — ADR 0074 §§ 5-6, ADR 0058 § 4. The empty
      class is `crates/nvs-stdlib/src/http.rs:427` and every new body owes an arm at
      `crates/nvs-stdlib/src/http.rs:166`. `examples/http.nvs:33` writes `$response->status` as a
      *property*, and a `Core` instance has no property a program can reach
      (`nvs_stdlib::registry::CoreClass::slots`' own doc), so either the example takes `->status()`
      or that rule moves — decide it and record it where the deciding doc is.
- [ ] **The transport behind the five rows** — ADR 0074 § 6, ADR 0051 § 3's "over the runtime's own
      reactor rather than a second event loop". `crates/nvs-stdlib/src/http.rs:606` is `request`,
      which already does everything but send; `crates/nvs-stdlib/src/http.rs:629` is the refusal it
      ends with, and deleting it also rewrites
      `tests/conformance/core/every-client-member-judges-a-request-before-it-leaves-the-process.nvst:73`.
      `crates/nvs-host/src/net.rs:223` is the parking stream to build on.
- [ ] **A spelling for `examples/http.nvs:40`'s environment read** — placement first, per ## State.
      `crates/nvs-stdlib/src/registry.rs:1054` is `CLASSES`, and a new class owes its spec row.

## Backlog

- `Core\Http\Client::send(Core\Http\Request)` — § 7's dynamic-verb half, which throws before the
  first attempt; `docs/spec/01-core-library.md` § 16 owns the row.
- A request body for `post`/`put`, which the same `send` row's taint decision governs —
  `crates/nvs-stdlib/src/http.rs`'s module doc says why it is not guessed.
- `website/src/content/docs/docs/adr/0074.md:335` still says `idempotencyKey` where
  `docs/adr/0074-http-defaults-safe-and-finite.md:360` says `retryIdempotencyKey`; whatever
  regenerates the site copy has not run since the flattening.
