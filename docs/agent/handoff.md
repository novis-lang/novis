# Handoff

## State

**ADR 0024 § 5's two language halves both lower.** `"<b>" as Core\Html\Markup` and
`Markup + Markup` are `InstKind::CoreCall`s on two row-less `nvs-stdlib` symbols,
`crates/nvs-stdlib/src/html.rs`'s `MARKUP_SYMBOL` and `MARKUP_CONCAT_SYMBOL`. The
helper-versus-registry-symbol question the last handoff left open is decided and recorded on the
first of those consts: a `Helper` is a symbol `nvs-runtime` exports and `nvs-runtime` cannot reach
a `Core` class's layout, so both are `CoreCall`s named through `nvs_types` exactly as `spawn
script` is. Neither has a `CoreMethod` row, for the reason `MARKUP` has no members at all.

**`nvs-ir` has no panicking conversion row left**, which its `lower::convert` `_ =>` arm and
`crates/nvs-ir/src/lib.rs` now claim rather than naming the one that was missing.

**§ 5's third piece is the sink's own escape-and-lift** — every non-`Markup` interpolation escaped
and wrapped — and it waits on an HTML response existing, which is goal 6's. That leaves `Core\Mail`
as stage 9's remaining acceptance check.

**The driver's failing check is `mail_sends_against_an_operator_named_endpoint_and_no_other`**, and
it is an item still open rather than a regression: there is no `crates/nvs-stdlib/src/mail.rs`, and
the spec gives the class one table row and no member roster.

**The pack still did not print ADR 0024 § 5**, sliced by hand for the third session running.
`[context] adrs` in `docs/agent/loop-goal.toml` wants `0024 § 5`.

## Next group

**`Core\Mail` — stage 9's one remaining check — over `crates/nvs-stdlib/src/mail.rs` (new),
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/lib.rs` and
`crates/nvs-stdlib/src/http/transport.rs`.**

- [ ] **The class row and its member roster** — `Core\Mail`'s transport is Native by ADR 0051 § 3's
      test 3 and its composition the framework's privileged half (ADR 0082 § 2). The spec has one
      line for it (`docs/spec/01-core-library.md:1105`) and no members, so the roster is this
      slice's decision under ADR 0063's twenty rules. The row goes in
      `crates/nvs-stdlib/src/registry.rs:1079`, its `net.connect` declaration beside it at
      `crates/nvs-stdlib/src/registry.rs:1392`, and the address chain is
      `crates/nvs-stdlib/src/lib.rs:350` — which owes a line even for a row-less symbol, per the
      playbook bullet this session added.
- [ ] **`mail_sends_against_an_operator_named_endpoint_and_no_other`** — ADR 0058 § 5's rule one
      protocol over: the endpoint is the operator's configuration and never the message's, so
      nothing a caller passes chooses a host. The exchange rides `nvs_host::net`'s parking stream,
      whose shape is `crates/nvs-stdlib/src/http/transport.rs:53`.
- [ ] **Three `.nvst` cases** in `tests/conformance/core/`, over whatever roster the first slice
      fixes, and `crates/nvs-stdlib/src/html.rs:250` is the nearest worked example of a class whose
      cases are about a rule rather than about a value.

## Backlog
- § 5's escape-and-lift into an HTML response — waits on goal 6 (`crates/nvs-stdlib/src/html.rs`, `# Known gaps`).
- `Core\Storage` over the `fs` capability — stage 9's fourth check (`docs/agent/loop-goal.toml:2552`).
- CLDR plural categories off the carried data — stage 9's fifth check (`docs/agent/loop-goal.toml:2553`).
- `Core\Html::sanitize` and ADR 0122's WHATWG parser — wait on `Core\Xml`'s tree (`crates/nvs-stdlib/src/html.rs`, `# Known gaps`).
- `[context] adrs` in `docs/agent/loop-goal.toml` still lacks `0024 § 5`.
