# Handoff

## State

**Goal 6, M7. ADR 0088 § 3's first row is in the tree: an HTTP request attaches the HTML sink.**
`nvs_runtime::OutputSink::Body` is that sink — the same buffer as `Buffer`, read back the same way,
and the only variant `Ctx::carrier` answers `Core\Html\Markup` for
(`crates/nvs-runtime/src/ctx.rs:4170`). It is selected in **one** place,
`crates/nvs-host/src/isolate.rs:270`: an isolate handed a request takes it, and one spawned inside a
request takes it too, because § 3's third row gives a child the *parent's* carrier. `Ctx::child`
propagates it for the same reason — an ADR 0072 task is part of the request, not a context of its
own. Everything else — a CLI program, a scheduled script, a job worker, a `#[Test]` method — keeps
the terminal sink by never having attached anything, which is § 3's fail-closed direction as a fact
about the sink rather than as a rule a call site states.

**The driver's failed acceptance check was a filing bug spanning four crates.** `nvs-server (the
binding table and the defaults)` named seven tests and *none* of the seven existed; the name was a
conjunction over ADR 0088 § 3's four-row table and ADR 0074 §§ 1-4, filed under the one crate that
could host barely half of it. It is now five checks: the request row over a socket (`nvs-server`),
the isolate row at the line that decides it (`nvs-host`), § 4's cookie where the member lives
(`nvs-stdlib`), the directive class where the registry lives (`nvs-config`), and § 1's shipped set
under the three names the tree already gave it. The scheduled-script row was **dropped with its
reason in the toml**: ADR 0073's runner does not exist, and until it does that run *is* an isolate
answering no request.

Three of the check's names are still genuinely open work, and they are the group below.

## Next group

**ADR 0074 §§ 2-4's defaults with nothing configured.** The first two share one file set:
`crates/nvs-server/src/secure.rs`, `crates/nvs-server/src/serve.rs` and the `HttpCors` block
`nvs_config::tree` already deserializes (`crates/nvs-config/src/tree.rs:408`).

- [ ] **CORS is closed with nothing configured** (ADR 0074 § 2). The header set is applied at
      `crates/nvs-server/src/secure.rs:211` and `nothing_configured_is_section_ones_shipped_set`
      (`crates/nvs-server/src/secure.rs:300`) is the shape to write beside. `origins = []` is the
      shipped default (`crates/nvs-config/src/tree.rs:408`), so the assertion is that **no**
      `Access-Control-Allow-Origin` reaches a response the peer sent an `Origin` on. The acceptance
      name is `cors_is_closed_with_nothing_configured`.
- [ ] **A preflight nobody configured is refused** (ADR 0074 § 2-3). Same two files; the `OPTIONS`
      arrives before a mount is chosen, so it is answered at the door beside § 1's set rather than
      by a program — `crates/nvs-server/src/serve.rs:729` is where a response is assembled.
- [ ] **`every_http_response_directive_is_runtime_class`** (ADR 0074 § 5, ADR 0005). One case over
      every `http.*` key a response reads, asserting the *class* rather than listing the keys —
      `crates/nvs-config/src/directive.rs:1` is the registry that answers it. Different file set:
      take it only if the two above left you well short of the ceiling.

## Backlog

- `Core\Request::clientIp()` and `scheme()` — the five edits each, `crates/nvs-stdlib/src/request.rs:196`.
- ADR 0097 § 6's `Origin::ignored_forwarded` is computed and never reported as a `Warn`.
- ADR 0074 § 4's cookie defaults — `crates/nvs-stdlib/src/response.rs:1292`, now its own check.
- § 3's scheduled-script row, when ADR 0073's runner lands — `docs/agent/loop-goal.toml` says so.
- `Core\Request::host()` waits on whether a forwarded host may be believed — nobody has decided.
- Whether `Core\Out::capture` inside a request should hand back `Markup` — `nvs_stdlib::out`.
