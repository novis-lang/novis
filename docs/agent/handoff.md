# Handoff

## State

**Goal 6, M7. ADR 0102 § 2's *table half* is whole, on both sides of the crossing.**
`nvs_runtime::routes::Routes::methods_for` walks the table with the verb filter dropped and answers
the verbs a path claims, once each and in load order; `Core\Router::methodsFor(tainted string
$path): array<Core\Http\Method>` is its program-visible reader over `Ctx::routes`. Empty is § 2's
`404` and non-empty is the `405`'s `Allow:`. § 1's match is unchanged.

**The door does not send either answer, and the check that says it does is wrong.** ADR 0102 § 1 —
"the program may still serve the request however it likes, because nothing here dispatches" — and a
door refusing a miss would refuse every request of a program that declares no route at all. So
stage 5's `no_methods_for_a_path_is_404_and_some_is_405_with_allow`, filed `-p nvs-server`, cannot
assert a status anything sends; the playbook bullet under *Tooling* has the whole finding. The
previous handoff's item 3 ("the door answers 404/405") is what that bullet retires.

**The other failing name is unchanged and still triaged**: neither half of
`the_csrf_check_and_the_route_label_read_the_match_rather_than_matching_again` exists — `nvs-server`
depends on no `nvs-stdlib`, where `Core\Csrf::verify` would live, and no `route` metric label is
emitted anywhere.

**`orient.py`'s gap, carried forward unclosed**: `[context] adrs` in `docs/agent/loop-goal.toml`
should gain ADR 0102 §§ 4 and 8 and ADR 0096 § 4 — §§ 4 and 8 are what the two open items are
specified against, and nothing printed them this session either.

## Next group

**Where § 2's two answers are *sent* from, and the two names stage 5's check still reports.** The
file set is `crates/nvs-server/src/route.rs`, `crates/nvs-runtime/src/routes.rs` and
`docs/agent/loop-goal.toml`.

- [ ] **File `no_methods_for_a_path_is_404_and_some_is_405_with_allow` where it can be honest**
      (ADR 0102 §§ 1-2) — the walk it would assert is `crates/nvs-runtime/src/routes.rs:479`'s
      `methods_for`, already covered there by three tests and by three `.nvst` cases; what is open is
      the check block at `docs/agent/loop-goal.toml:3447`, whose `args = ["test", "-p",
      "nvs-server"]` puts the test in the one crate that may not send the status. Decide between
      moving the check to `-p nvs-runtime` and giving `crates/nvs-server/src/route.rs:57` a
      *shape* — the reply a dispatcher would send — with no sender behind it; the first is what
      the ADR reads like, and the comment above the block is the specification either way.
- [ ] **The CSRF check and the `route` label read the match rather than matching again**
      (ADR 0102 §§ 1 and 8, ADR 0096 § 4) — `crates/nvs-server/src/route.rs:57` is where the match
      lands and the reader would sit; `crates/nvs-server/Cargo.toml:1` is the manifest that names no
      `nvs-stdlib`, which is why the CSRF half is a dependency decision before it is a test.
- [ ] **`nvs_runtime::routes` gap 2: a `decimal` and a `Core\Uuid` capture still match as text**
      (ADR 0102 § 5) — `crates/nvs-runtime/src/routes.rs:100`'s `CaptureConv::Unconverted` and the
      arm missing from `crates/nvs-runtime/src/routes.rs:321`'s `convert`, which is `commands`' gap 1
      with the same fix waiting.

## Backlog

- `Core\Router::match` stays out of scope — `docs/agent/loop-goal.md` § *Standing decisions*.
- The mount prefix in front of a link — `crates/nvs-stdlib/src/router.rs`'s gap 2.
- A capture's text is still percent-encoded — `crates/nvs-runtime/src/routes.rs`'s gap 4.
- A `501` at the door for a verb outside the roster — `crates/nvs-stdlib/src/router.rs`'s gap 4.
