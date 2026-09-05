# Handoff

## State

**Goal 21, stage 10 is closed.** `Core\Uri`'s two decoders answer `bytes` (landed earlier), a route
capture is percent-decoded exactly once where it crosses into a program, and
`examples/octets.nvs` prints the stage's three frozen lines. Conformance 1556, differential 275.

**The decode's home is `crates/nvs-stdlib/src/router.rs`'s `capture_value`**, and that doc owns the
rule for both readers: `Core\Request::route()` and `Core\Router::match` share `match_value`, so one
decode serves a served request and a program's own question alike. A capture whose octets are not
UTF-8 **throws** — its binding is a `tainted string` and ADR 0009 § 1 leaves it no `string` to be.
`crates/nvs-runtime/src/routes.rs` states the other half as prose rather than as a gap: the
conversion runs there and runs first, so a decode below the crossing would let `%34` reach a
`{n: uint}` route as `4`.

**`Core\Router::match` landed**, spec § 15's and ADR 0102 § 1's row, and it was not optional: no CLI
program can read a capture back without it, so the frozen `capture: hello world` line had no honest
program. It walks `Routes::match_request` — the door's own walk — and dispatches nothing, so ADR
0077 § 4's refusal is untouched. Both of `router.rs`'s stale gaps are gone.

**ADR 0102 § 1 was deliberately not amended.** The goal's § *Standing decisions* names ADR 0067 §§ 3
and 13 and spec § 12 as the only folds it may make. § 1 is *silent* about a capture's encoding
rather than wrong about it, so nothing there contradicts the tree; the first item below is where
that sentence belongs once a goal may open it.

**The stage's `-p nvs-server` check could never have run**: the decode is at the crossing, in
`nvs-stdlib`, and `crates/nvs-server/Cargo.toml` names neither `nvs-stdlib` nor `nvs-types`. Both
TOML copies now file it `-p nvs-stdlib`. Nothing is blocked on a decision.

## Next group

**The paperwork the two landed members leave behind.** One file set:
`docs/spec/01-core-library.md`, `docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md`,
`crates/nvs-stdlib/src/router.rs`, `docs/agent/goals/27-gap-owners.md`.

- [ ] **Spec § 15's `Core\Router` row names `match`'s two parameters.** ADR 0063 R2 has every
      parameter callable by the spec's `$name`, and that row writes
      `match(Http\Method, tainted string)` with neither named while `url(string $name, …)` beside it
      names its own. The registry row already calls them `method` and `path` —
      `docs/spec/01-core-library.md:982`, `crates/nvs-stdlib/src/router.rs:241`.
- [ ] **ADR 0102 § 1 gains the sentence saying where a capture is decoded**, once a goal's standing
      decisions admit that ADR. Fold, do not overlay: the body states the rule and the module doc
      keeps the reasoning — `docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md:96`,
      `crates/nvs-stdlib/src/router.rs:1007`.
- [ ] **Goal 27's stage 3 item 4 is answered before that goal starts.** It asks whether
      `router.rs` gap 3 ("`Core\Router::match` is absent") or the plan is wrong; the member has
      landed, so the item is a settled question rather than a judgement —
      `docs/agent/goals/27-gap-owners.md:70`, `docs/agent/goals/27-gap-owners.handoff.md:45`.

## Backlog

- `Core\Http\Method::Head` matches only `Head` rows, exactly as the door's own walk does
  (`crates/nvs-server/src/route.rs:90`); nothing maps it to `Get` for routing —
  `crates/nvs-stdlib/src/request.rs`'s module doc owns the one place that mapping exists.
- A mount capture (`Core\Request::mount()`) still crosses undecoded, and ADR 0097 § 4 has not said
  whether it should — `crates/nvs-stdlib/src/request.rs:1651`.
- The remaining goal-21 items are stage 11's suites only; every earlier stage was green when this
  session started — `docs/agent/loop-goal.toml`.
