# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and **stages 3 to 7 are complete**. Stage 8 is open on two counts: its `nvs-suite` check wants three
`.nvst` cases and two are on disk, and its `cargo-named` check — a served request's mount origin, and
the boot refusal for a mount that needs one and has none — is untouched. Nothing is blocked.

**`rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name` is `shipped`.** One
compile-time answer serves every position: the match converts on it, `Core\Router::url` is refused
against it and renders through it, and the generated document lists it. The `#[Query]` half left in
its closing paragraph is an *arrival* and not this rule's — no query value of any declared type is
bound to its parameter yet (`rule:routing/a-bad-query-value-is-a-400`).

**Where the spelling crosses to run time.** `nvs_types::links`' `spellings` collects a name-spelled
subset's value-to-segment rows, `UrlPiece::prepared` writes them as `link::SPELLING` pieces in front
of every path piece, and `nvs_stdlib::router`'s `spelled` looks the arriving text up in them. A
value-spelled subset carries no rows at all: a case is its backing integer by the time it is a value
(`rule:enums/no-class-machinery`), and that decimal is already the segment. The rows are a
*conversion* and never a check, which is what lets them cross where `substitute`'s own doc refuses to
carry a closed set for a refusal.

**`RouteParam::admits` is the one home of "every segment this parameter admits"** — the literal
union's members and an enum subset's spellings as one list. `links`' refusal and `nvs_cli::openapi`'s
`enum:` row both read it, because a route, its links and its document are spelled the same way or one
of the three is wrong.

## Next group

**Stage 8: the routes, the mount prefix** — one file set: `crates/nvs-stdlib/src/router.rs`,
`crates/nvs-stdlib/src/test.rs` and `crates/nvs-runtime/src/ctx/inbound.rs`.

- [ ] **`url` prepends the mount prefix the door stripped** — `crates/nvs-stdlib/src/router.rs:65`'s
      gap 2, owned by this goal: the door puts the prefix on the request
      (`crates/nvs-server/src/mount.rs:518`'s `carry` into
      `crates/nvs-runtime/src/ctx/inbound.rs:653`'s `set_mount`) and no member here asks for it, so
      `crates/nvs-stdlib/src/router.rs:973`'s `nvs_core_router_link` answers from the mount root out.
      That body is where the prefix joins, in front of `substitute`'s answer, and a command-line run
      has none to read. `rule:routing/link-carries-the-mount-prefix`. **The settled half of the
      decision:** a synthetic request carries no mount at all —
      `crates/nvs-stdlib/src/test.rs:1911`'s `described` builds a
      `crates/nvs-runtime/src/ctx/inbound.rs:1055` `InboundSpec` with a path, a query, headers and a
      body and nothing else — so the `.nvst` the check wants needs the test bag to carry one key
      more, beside `headers` and `body`, and a spec field the isolate applies through `set_mount`.
      Case: `tests/conformance/core/router-url-prepends-the-mount-prefix-the-door-stripped.nvst`.
- [ ] **A served request receives its mount's resolved origin, and a mount that needs one and has
      none refuses the boot** — stage 8's `cargo-named` check, and
      `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s own unbuilt half: the reader parses
      and substitutes a mount's `origin` and only a command-line run installs one
      (`crates/nvs-cli/src/main.rs:1296`). `crates/nvs-config/src/mount.rs:206`'s `expand` is where
      the per-resolved-mount check belongs, `crates/nvs-server/src/mount.rs:518`'s `carry` is where
      the value would reach the request beside the prefix above, and
      `crates/nvs-runtime/src/ctx/wiring.rs:338`'s `Ctx::origin` is what
      `crates/nvs-stdlib/src/router.rs:992`'s `urlAbsolute` already reads.

## Backlog

- A `#[Query]` value is not bound to its parameter for any declared type — owner
  `rule:routing/a-bad-query-value-is-a-400` § *The query half is not shipped*.
- `urlSigned` over a name-spelled enum capture: `signed_payload` writes whatever `$params` held, so a
  program passing the segment spelling rather than the case signs bytes the verifier would not derive
  — not checked against `nvs_core_router_signed_route`, and `rule:core-classes/router-signed-url` is
  where it lands if it is real.
- `crates/nvs-types/src/links.rs`'s gap 1, a named argument in a link, stays with goal
  `unowned-closures`.
