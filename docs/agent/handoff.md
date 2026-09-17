# Handoff

## State

**Goal `decided-closures`, stage 2 — the runtime and the lowering.** Stage 1's floor is the closed
goal `cache-shared-dial`'s checks, and they pass. Both of stage 2's own checks are green; what keeps
the goal open is its **owner gate**, and two stage 2 items are still gaps naming it.

`crates/nvs-runtime/src/routes.rs` gap 1 is **struck as a stated bound**, its `Decided:` sentence
having asked for a measurement first. The walk is a comparison over every row of the right verb and
stays one: the module now says so as prose, and the figure's home is
`benches/abi-probe/benches/routing.rs` — a new criterion target with hit arms at 8, 64 and 512 rows
plus a miss at the largest, so the slope between two sizes is the per-row cost with the path split
cancelled out. `benches/abi-probe/tests/perf_guards.rs`'s
`a_route_table_walk_costs_a_fraction_of_the_request_it_rides_in` fails a build when a row leaves
that cost class, and `benches/abi-probe/shared/routes.rs` is the one table both of them build, on
the `shared/isolate.rs` pattern beside it.

The denominator is `benches/serve-proxied.json`'s `nvs-serve-direct` arm, **read off disk rather
than re-run**: that bench is a Docker ratio against php-fpm over a one-route `hello`, so no run of
it at any table size could have shown a table's length. Nothing is blocked.

## Next group

**Stage 2: the two gaps the stage's checks do not name** — one file set:
`crates/nvs-runtime/src/`, `crates/nvs-ir/src/`, and `crates/nvs-host/src/` for the first item.

- [ ] **`crates/nvs-ir/src/lib.rs:614` gap 18 — a throw escaping an abandoned generator's `finally`
      reaches the escalation ladder** — `rule:errors/escalation-ladder`, whose tier 3 is
      `crates/nvs-host/src/ladder.rs:90` and therefore **above** `nvs-runtime`: the decided answer
      ("report it through the ladder, without replacing anything") is a hook, not a call. The throw
      is dropped at `crates/nvs-runtime/src/ctx/error.rs:324`, whose one caller is
      `crates/nvs-runtime/src/object.rs:3367`. Tier 4 is already on this side —
      `crates/nvs-runtime/src/floor.rs:110` builds the record from a `Thrown` and
      `crates/nvs-runtime/src/floor.rs:258` writes it — so what the hook adds over calling those two
      directly is tier 3, and `crates/nvs-cli/src/main.rs:2328` is the existing caller whose shape
      an installed hook has to agree with.
- [ ] **`crates/nvs-runtime/src/record.rs:48` gap 1 — a `secret` into an array element or a shape
      field is refused where it is written** — `rule:security/secret-qualifier`. Its `Decided:`
      sentence is a **compile-time** refusal, so the build is `crates/nvs-types/src/` and not this
      crate: it is the same work as stage 3's failing acceptance check
      `a_secret_stored_into_an_array_element_is_refused_at_compile_time`, and the record walk's item
      is struck once the checker refuses. `crates/nvs-runtime/src/record.rs:48` is the gap and
      `nvs_types::expr::quals::reject_secret_debug_argument` is the refusal it is modelled on.

## Backlog

- The `[context] modules` manifest reaches no `nvs-host` path, so the ladder above did not print —
  `docs/agent/loop-goal.toml`, and the next item needs it.
- Stage 3's three `[[check]]` blocks are the goal's earliest red ones — `docs/agent/loop-goal.toml:11568`.
- `benches/serve-proxied.json` has no large-table arm; the walk's figure is a microbench by
  necessity — `benches/abi-probe/benches/routing.rs` owns why.
- 39 gaps still name this goal outside stage 2 — `python tools/owners.py --closes decided-closures`.
