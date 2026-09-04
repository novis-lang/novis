# Handoff

## State

**Goal 6, M7. ADR 0074 § 2 is whole.** `nvs_server::cors` resolves the entire `[http.cors]` block
at boot and answers both things § 2 configures: an ordinary answer's `Access-Control-Allow-Origin`,
`Vary`, `Allow-Credentials` and `Expose-Headers`, and a preflight's `204`-or-`403` carrying
`Allow-Methods`, `Allow-Headers` and `Max-Age`. **One match decides both** — `Cors::answer`'s
`Crossing`, which `Preflight` carries rather than matching again — and nothing but the `Vary` is
written onto an answer that has no origin to allow. That module's doc owns the whole argument,
including why `expose` goes on an ordinary answer and not on the preflight in front of it.

**A preflight is now `OPTIONS` + `Origin` + `Access-Control-Request-Method`**, which is ADR 0097
§ 4's own definition. The landed code read only the last of the three, so an `OPTIONS` that named
no origin took the policy's answer; it now reaches the program, as it does under a closed policy.

**`nvs_config::http`'s `validate` refuses what `cors` would otherwise have to repair while
answering**: `E0625` for a `methods`/`headers`/`expose` entry a header line cannot carry, `E0601`
for a `max_age` that is not a duration. That is what lets `Cors::of` stay infallible and resolve
the block into header lines with no arm for a value it cannot use — `Cookies::of`'s arrangement,
for `Cookies::of`'s reason.

**The driver's failing check is an open item, not a regression**, and last session's triage still
holds: `nvs-server (match once, and the two answers)` names four ADR 0102 tests, no route table
reaches the server, and two of the four cannot be hosted by `-p nvs-server` at all. The group below
opens it. `[context] adrs` now names `0102 §1` and `0102 §2`; the `0074:2` gap the last handoff
reported is moot, § 2 being closed.

## Next group

**ADR 0102's route table reaching a request** — every one of the four tests the failing check names
rests on it, and the table already exists on the compiler's side. The file set is
`crates/nvs-server/src/serve.rs`, `crates/nvs-server/src/mount.rs` and
`crates/nvs-stdlib/src/request.rs`, against `nvs_types::routes`.

- [ ] **The compiled unit's route table reaches the door, and the match is taken once** (ADR 0102
      § 1). `crates/nvs-types/src/routes.rs:517` is `RouteTable` as the compiler builds it;
      `crates/nvs-server/src/serve.rs:630` is where the door decides everything it decides before
      the handler takes the request by value — the `Crossing` is computed on that line for exactly
      that reason — and `crates/nvs-server/src/mount.rs:332` is the `Selection` a request already
      resolves to before any of it.
- [ ] **`Core\Request::route()` answers that match rather than matching again** (ADR 0102 § 1).
      `crates/nvs-stdlib/src/request.rs:192` is the class's row block;
      `crates/nvs-stdlib/src/router.rs:38` is the known-gaps block saying `Core\Router::match` is
      unwritten, and it is the doc that goes stale the moment this lands.
- [ ] **§ 2's two answers: `404` where a path has no methods, `405` with `Allow` where it has
      some** (ADR 0102 § 2). `crates/nvs-server/src/mount.rs:308` is `Resolved`, whose `404` today
      is the mount table's and not a route table's.

## Backlog

- ADR 0102 § 5's capture narrowing and § 6's per-mount absolute origin — `crates/nvs-stdlib/src/router.rs`'s gap 2 names the missing half.
- `Core\Session`: ADR 0012 § 4 fixes the shape, ADR 0059 § 4 forbids the local tier — stage 5's `nvs-stdlib (session, and the tier it may not use)`.
- ADR 0105's still-open upload names, in the check that holds only what the tree does not have yet.
- `[context] adrs` can drop `0074 §5` and `0105 §§ 2-4` once stage 5 has moved past uploads.
