# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stages 1–4 are on disk, and nothing of stages 5–14 is.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

**Stage 4 is closed.** `Core\Http\Client` carries `patch` and `request(Core\Http\Method $method, …)`
beside the five named rows (`crates/nvs-stdlib/src/http.rs:800`); `request` reads the verb out of slot 0
and hands `request()` a slice of its own arguments, so every option sits one slot on and nothing reads
the bag twice. What a named row's call site is refused while compiling — a body on a bodyless verb, a
keyless retried `POST` or `PATCH` — `judge_verb` throws before the first attempt where the verb is an
argument (`crates/nvs-stdlib/src/http.rs:1106`). The `headers` option is `array<secret string>` now:
`rule:security/secret-sinks-refuse` names an outbound header as one of the three positions a credential
has to reach, and a `tainted` value is still refused there by assignability.

Nothing is blocked and no design question is open.

## Next group

**Stage 5: reading a reply** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-types/src/expr/args.rs`.

- [ ] **The reply carries its headers** — `header(string $name): ?tainted string` and
      `headers(string $name): array<tainted string>` join `Core\Http\Response`'s two instance rows at
      `crates/nvs-stdlib/src/http.rs:875`, over a third slot beside `status` and `body` at
      `crates/nvs-stdlib/src/http.rs:898`, which the transport fills from what it already parsed.
      Names match case-insensitively; `header` joins repeats with `, ` and is a `LogicError` naming
      `headers` for `set-cookie`. Goal prose stage 5, `rule:security/tainted-qualifier`.
- [ ] **`bytes(): tainted bytes` reads the body as octets** — the same slot `text` reads, handed back
      without the UTF-8 demand, beside `text` at `crates/nvs-stdlib/src/http.rs:875`. Every reply is
      `tainted`, which is this goal's standing decision, not a new one.
- [ ] **`jsonAs<T>()` joins the decode-site roster** — the check is
      `crates/nvs-types/src/expr/args.rs:1517-1528`, where `Core\Request::jsonAs` already sits: `T` is a
      `tainted` shape, or a class whose text fields declare `tainted`
      (`rule:security/derived-codec-qualifiers`).
- [ ] **`Retry-After` is read in both forms** — `retry_after` at
      `crates/nvs-stdlib/src/http/transport.rs:467`, today delay-seconds only and read after all four
      closed statuses at `crates/nvs-stdlib/src/http/transport.rs:211`. An HTTP-date past the remaining
      deadline throws, one already past keeps the jittered backoff, and the header is read after a `429`
      or a `503` alone (`rule:http-server/retry-is-opt-in-jittered-and-closed`).

## Backlog

- Stage 6 onwards is untouched: streaming, the per-core pool, compression, redirect credentials, TLS.
- `docs/agent/loop-goal.md` § *Not this goal* names what a session must write here instead of taking.
