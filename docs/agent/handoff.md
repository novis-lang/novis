# Handoff

## State

**Stage 5's compile-time half is closed.** `Core\Http\Client` is five rows — `get`, `post`, `put`,
`delete`, `head` — each over ADR 0058 § 1's sink (`string | Core\Http\Target`, both unqualified) and
one trailing `Core\Http\Options` bag, in `crates/nvs-stdlib/src/http.rs`. Its module doc is the one
home for why `retry` is flat, why there is no body parameter yet, and why `Core\Http\Response` is a
name with nothing behind it.

**ADR 0074 § 5's nested `retry` shape is now three flat options** — `retryAttempts`, `retryBackoff`,
`retryIdempotencyKey`. A bag flattens to one ABI argument per option, so a bag nested inside one has
nothing to flatten into and the registry refuses it. § 5's own type block, §§ 6-7's prose and the
spec's § 16 row were amended to the flat spelling; the ADR body is the rule.

**There is no transport, and every row says so at run time.** A request member resolves and pins
(`string` argument), judges § 5's bounds and § 6's attempt count, and then throws
`… the request is approved and there is no transport behind it yet`. That sentence is pinned by
`every-client-member-judges-a-request-before-it-leaves-the-process.nvst` and is the one expectation
in the suite the transport slice deletes.

**Three of the item's four `nvs-types` tests are green.**
`no_client_member_accepts_an_unbounded_timeout` (`crates/nvs-types/src/core_lib.rs:896`) asks the
lowered bag, not the row: every bound describes as exactly `Core\Time\Duration`, none is nullable,
and each defaults to *not given*. `a_post_retried_without_an_idempotency_key_is_a_compile_error` is
still open — it needs a diagnostic, and it is the next group's first slice.

**`examples/http.nvs` fails on three things now, none of them the client's rows.**
`$response->status` is a property read, and a `Core` instance has no property a program can reach
(`registry::CoreClass::slots`' own doc) — so either the example takes a member call or that rule
moves; `$response->text()` needs `Core\Http\Response`'s readers; and `Core\Env::get` has no row in
`nvs_stdlib::registry` at all.

**The manifest gap the last two handoffs carried is closed, by being wrong.** `[context] modules`
already globs `crates/nvs-stdlib/src/*.rs`, and names `nvs-host/src/net.rs` and `blocking.rs` rather
than the dead `pool.rs`/`stream.rs` pair. ADR 0074 §§ 5-7 are ~3k of pack and belong to three stage-5
sessions out of the goal's remainder, so they stay sliced per item rather than added to
`[context] adrs`. Nothing is blocked.

## Next group

**Stage 5's transport, and the one refusal that is still a promise.** The file set is
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-types/src/expr/args.rs`,
`crates/nvs-types/src/core_lib.rs` and `tests/conformance/core/`.

- [ ] **`post` with a retry and no idempotency key is a compile error** — ADR 0074 § 7. The options
      bag is a written shape literal and the verb is the member's own name, so both halves are known
      at `crates/nvs-types/src/expr/args.rs:631`, where `E0454` already refuses an unknown option
      key. A new code beside `crates/nvs-diagnostics/src/lib.rs:2608` (`E0796` is free), and the
      test goes beside `crates/nvs-types/src/core_lib.rs:896`.
- [ ] **`Core\Http\Response`, its readers and the transport** — ADR 0074 §§ 5-6, ADR 0058 § 4. The
      class is `crates/nvs-stdlib/src/http.rs:410` with no slots and no members on purpose; the
      request body that throws instead of sending is `crates/nvs-stdlib/src/http.rs:589`, over
      goal 2's parking stream (`crates/nvs-host/src/net.rs`). A redirect hop re-pins through
      `crates/nvs-stdlib/src/http.rs:207`'s `pin`, and retries reuse the target rather than
      re-resolving.
- [ ] **`Core\Env::get`, which `examples/http.nvs:40` needs** — the class has no row in
      `crates/nvs-stdlib/src/registry.rs:1225`'s list. Its answer is `?tainted string`: the
      environment is outside the process, so ADR 0024 § 2 makes it tainted and `allowUrl` is what a
      program does with it.

## Backlog

- A request body for `post`/`put`, decided with `send(Core\Http\Request)` — `crates/nvs-stdlib/src/http.rs`'s module doc owns why it is not guessed here.
- `examples/http.nvs:33`'s `$response->status`: a property on a `Core` instance, which `registry::CoreClass::slots` says does not exist.
- Stage 5's local origin harness for `examples/http.nvs` — `docs/agent/loop-goal.toml`'s stage 5 `exact` check.
- `an_outbound_request_carries_traceparent` and the reactor test — ADR 0076 § 2, with the transport.
- Header values are `Qual::Neutral` and refuse `tainted` by assignability alone; ADR 0088's classification is not read at calls yet (`crates/nvs-types/src/core_lib.rs:366`).
