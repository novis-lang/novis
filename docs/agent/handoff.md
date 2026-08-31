# Handoff

## State

**Stage 5's door is open: `Core\Http::allowUrl` is a row, a card, a body and three `.nvst` cases**
in `crates/nvs-stdlib/src/http.rs`, beside `jwt.rs` and the rest of stage 4's roster. Its module doc
is the one home for why the answer is a value, why `Core\Http\Target` has no members, and which half
of the policy it does not hold.

**ADR 0058 § 5's address policy is in the capability, not in this client.** The denied-range table is
`nvs_config::capability::denied_by_default` (`crates/nvs-config/src/capability.rs:86`) and the door
that applies it is `nvs_runtime::capability::pin_host`
(`crates/nvs-runtime/src/capability.rs:117`), which asks the grant about the *name*, resolves, and
then asks the table about the *address* — in that order, so an ungranted program cannot use the
member as a resolver. `Core\Net` and `Core\Db::open` reach the same door rather than a second copy.
The operator-exception half of § 3 (a grant that widens the table for one internal API) is not
written; today the table is deny-only.

**Two of the item's four `nvs-types` tests landed; two could not.**
`an_outbound_url_parameter_refuses_a_tainted_operand`
(`crates/nvs-types/src/core_lib.rs:790`) and `allow_url_pins_what_it_launders` (`:842`) are green.
`no_client_member_accepts_an_unbounded_timeout` and
`a_post_retried_without_an_idempotency_key_is_a_compile_error` are properties of
`Core\Http\Client`'s rows, which do not exist — a test over an absent class passes vacuously, which
is worse than an open item. They are the next group's first slice.

**Why the client's rows are their own slice and not an oversight.**
`every_core_class_has_a_conformance_floor_of_three` needs three cases per member, and
`every_error_path_is_asserted_or_declared_unreachable` needs each `Fault::` site in `nvs-stdlib`
caught by one. A `get`/`post` row therefore owes cases before it owes a socket — and they can be
`--EXPECTF-ERROR--` cases, which never run, so the compile-time half can land whole before any
transport exists.

**The driver's acceptance failure is still stage 5's.** `examples/http.nvs` now type-annotates
`Core\Http\Target` where it said `string` (ADR 0058 § 2's signature); it fails on
`Core\Http\Client::get`, which no slice has landed, and its `127.0.0.1:8099` origin needs both the
transport and the operator exception above.

**Manifest gaps.** `[context] adrs` still needs ADR 0060 §§ 1, 4, 5, and now ADR 0058 §§ 1-5 and
ADR 0074 §§ 5-7 — this session paid for all of 0058 and 0074, and § 3 in particular is the table it
implemented. Two dead `[context] modules` selectors (`crates/nvs-host/src/pool.rs`,
`crates/nvs-host/src/stream.rs`); it wants `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-runtime/src/capability.rs`, `crates/nvs-config/src/capability.rs`,
`crates/nvs-types/src/core_lib.rs`, `crates/nvs-stdlib/src/registry.rs` and
`crates/nvs-test/src/case.rs` added. `[context] shapes` wants the `.nvst` format's
`--FILE <path>--` section, which cost this session real time. Nothing is blocked.

## Next group

**Stage 5's compile-time half, finished.** The file set is this session's:
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/core_lib.rs` and `tests/conformance/core/`.

- [ ] **`Core\Http\Client`'s rows and its options shape** — ADR 0074 § 5. `get`, `post`, `put`,
      `delete`, `head` over `string | Core\Http\Target`, both unqualified so a `tainted` operand is
      a diagnostic (ADR 0058 § 1), with `Core\Http\Options` as the one trailing shape: `deadline`
      and `connectTimeout` are `Duration` with no null and no zero, `retry` is
      `{attempts, backoff?, idempotencyKey?}`. Rows and cards beside
      `crates/nvs-stdlib/src/http.rs:61`, the class list at
      `crates/nvs-stdlib/src/registry.rs:1215`. `crates/nvs-stdlib/src/jwt.rs:165` is the shape for
      a `Duration` in a row.
- [ ] **The two deferred tests, which the rows above make non-vacuous** — ADR 0074 §§ 5, 7.
      `no_client_member_accepts_an_unbounded_timeout` and
      `a_post_retried_without_an_idempotency_key_is_a_compile_error`, beside this session's two at
      `crates/nvs-types/src/core_lib.rs:790` and `:842`. The second is a *diagnostic*, not a row
      property: R2 makes the options bag a compile-time-constant shape literal and the method is the
      member's own name, so the check is at the call site and the next free code is `E0796`.
- [ ] **The cases those rows owe** — three per member, under `tests/conformance/core/`, and
      `--EXPECTF-ERROR--` for every one that pins a refusal, so none of them needs a transport. The
      two gates are `crates/nvs-stdlib/tests/conformance_coverage.rs:286` and `:638`, and
      `tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst:1` is this session's
      shape for a case that writes its own `nvs.toml`.

## Backlog

- The transport itself: `Core\Http\Client` over goal 2's parking stream — `docs/agent/loop-goal.toml` stage 5's `nvs-stdlib` check.
- The operator exception to the address policy, and the `127.0.0.1:8099` origin harness — ADR 0058 § 3.
- `a_redirect_is_re_checked_against_the_same_policy` and the retry jitter — ADR 0058 § 4, ADR 0074 § 6.
- `an_outbound_request_carries_traceparent` — ADR 0076 § 2.
- Stage 6's two stores, `Core\Cache` and `Core\RateLimit` — ADR 0059, ADR 0075.
