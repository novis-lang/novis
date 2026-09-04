# Handoff

## State

**Goal 6, M7. ADR 0102 § 1 is whole: a request is matched once at the door, and the match now
reaches the program.** `Core\Request::route()` answers `?Core\Router\Match`, built by
`nvs_stdlib::router`'s `match_value` out of the `nvs_runtime::routes::Match` the door left on the
`Inbound`. The class answers `name()`, `params()` and `param()` and carries nothing else — its own
doc owns why the matched `Route` does not cross, and `nvs_runtime::routes` gap 3 says the same from
the other side.

**A capture is a union, not a string.** `params()` answers `array<uint|int|tainted string>`;
`router.rs`'s `CAPTURE` owns the argument, and it is that § 1's "typed parameters" forbids
re-parsing a `{id: uint}`, while `mixed` was refused because a `mixed` binding accepts a `tainted
string` and would launder the request path. `name()` is plain `?string`: it is the unit's own
`#[Route(name: …)]` literal rather than anything the peer wrote.

**The driver's `nvs-server (match once, and the two answers)` check still fails on its other three
names**, which is the ordinary open state. Triage of the one it reports, so the next session does
not redo it: **neither half of `the_csrf_check_and_the_route_label_read_the_match_rather_than_
matching_again` exists in any crate yet** — `crates/nvs-server` names no `csrf` at all and its
`Cargo.toml` does not depend on `nvs-stdlib`, where `Core\Csrf::verify` lives; and no metric label
named `route` is emitted anywhere. That item is open work with a design call in front of it, not a
misfiling.

**`orient.py` did not print what that triage needed**: `[context] adrs` in
`docs/agent/loop-goal.toml` should gain ADR 0102 §§ 4 and 8 and ADR 0096 § 4, which are what the
group's remaining two items are specified against.

## Next group

**ADR 0102 § 2's two answers — the question asked only once `match` has returned `null`.** The file
set is `crates/nvs-runtime/src/routes.rs`, `crates/nvs-stdlib/src/router.rs` and
`crates/nvs-server/src/route.rs`.

- [ ] **`Routes::methods_for` answers the verbs a path claims, in load order** (ADR 0102 § 2) — the
      same walk `crates/nvs-runtime/src/routes.rs:447`'s `match_request` already performs, collecting
      every row's verb instead of stopping at the best rank, and reporting both forms where § 4's
      `{name?}` makes a node terminal.
- [ ] **`Core\Router::methodsFor` is its program-visible reader** (ADR 0102 § 2) — the five edits at
      `crates/nvs-stdlib/src/router.rs:209`'s `CLASS` and `crates/nvs-stdlib/src/router.rs:458`'s
      `address`, answering `array<Core\Http\Method>` over the table `Ctx` was installed with;
      `crates/nvs-stdlib/src/router.rs:339`'s `MATCH` is the sibling that landed this session and
      the shape to copy.
- [ ] **The door answers `404` where a path claims no verb and `405` with `Allow` where it does**
      (ADR 0102 § 2) — `crates/nvs-server/src/route.rs:57`'s `take`, which today records a match and
      returns; the `Allow:` list is the member above, and § 2 says it is asked only after `match`
      answered `null`.

## Backlog

- The CSRF check and the `route` label read the match — ADR 0102 § 8, ADR 0096 § 4; blocked on where
  the check runs, per the triage in `## State`.
- `Core\Router::match` stays out of scope — `docs/agent/loop-goal.md` § *Standing decisions*.
- ADR 0102 § 7's mount captures reaching a handler as tainted values — same stage 5 check.
- A capture's value is still percent-encoded — `nvs_runtime::routes` gap 4.
- `decimal` and `Core\Uuid` captures are unconverted — `nvs_runtime::routes` gap 2.
- The match walk is a linear scan rather than ADR 0077 § 2's trie — `nvs_runtime::routes` gap 1.
