# Handoff

## State

**Stage 5's compile half is closed.** ADR 0088 § 2 now says a mark that admits `tainted` admits a
**union** carrying it, arm by arm, and `examples/http.nvs` type-checks. `nvs_types::expr::quals`
owns both halves: `untainted` reaches through a union so each arm is narrowed and then compared,
and the new `carries_tainted` reads the contagion back out. The two reaches are **deliberately
asymmetric** — `carries_tainted` reaches through `array<…>` as well, because over-tainting a result
is safe while over-admitting an argument leaks — and `untainted`'s own doc comment is the home of
why, including that `core_lib::qual_of` gives an array parameter no classification at all, so no
array can reach the admission side today.

**`examples/http.nvs` has one blocker left**: nothing serves `127.0.0.1:8099`, so line 32 throws
`connecting to 127.0.0.1:8099 failed`. That is the whole of what stands between the tree and the
stage 5 `exact` check at `docs/agent/loop-goal.toml:2357`, which wants `status=200` and `body=ok`.

`unsecret` did **not** follow into a union: ADR 0033 § 3's escape hatch has no contagion to carry, so
the reach would buy nothing and cost the axis its over-strict posture. Recorded at `unsecret`'s site.

## Next group

**The origin the acceptance check needs, and the case that covers it.** They share the driver's
stage-5 setup and `examples/http.nvs`; nothing here touches `nvs-types` again.

- [ ] **An origin on `127.0.0.1:8099` for the acceptance check.** The stage 5 `exact` check at
      `docs/agent/loop-goal.toml:2357` wants `status=200` and `body=ok` from a real server, and
      `examples/http.nvs:32` is the call that asks for it. Decide *where* it is started — the driver
      bringing one up around the stage, or the example spawning its own — and record the choice
      beside the check. ADR 0097 § 1 is the development server's own section and the first place to
      look for something already able to serve one route.
- [ ] **A `.nvst` case for the transport's success path**, once an origin exists to talk to:
      today every client case pins a refusal, so `crates/nvs-stdlib/src/http/transport.rs:1` has no
      case asserting a 200 with a body is read back. `tests/conformance/core/every-client-member-refuses-a-tainted-url.nvst:1`
      is the sibling to sit beside.

## Backlog

- `--INI--` is the last section on `crates/nvs-test`'s `case.rs` `NOT_YET` list.
- `Core\Env::mode()` and the `EOL`/`OS`/`VERSION` constants are in the spec's § 15 row, unwritten —
  `docs/spec/01-core-library.md` § 15 owns the roster.
- Stage 6's two stores are next after stage 5 closes — `docs/agent/loop-goal.toml`, stage `6 stores`.
- A `Qual::Launder` parameter declared `array<text>` is unreachable by construction; if a row ever
  wants one, `crates/nvs-types/src/core_lib.rs:335`'s `qual_of` is the limit to lift first.
