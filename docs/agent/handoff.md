# Handoff

## State

**ADR 0042 is on disk whole except its bench**, in `crates/nvs-cli/src/cache.rs`: §§ 1-4's writer
and reader, § 5's trust check and § 6's eviction. All six of the goal's `cargo-named` tests for
*nvs-cli (the cache)* pass, on Windows included. Stage 5's one open check is
`tools/bench.py --warm-start --max-ms 10`, which is the next group's first item.

**§ 5 sits in `Cache::new`, which is fallible now.** A `Cache` value is itself the evidence that
`nvs_config::trust::check` passed on its directory — the discipline `Verified` holds one level up
on the read path — so no caller can be the one who forgets. A directory the first `store` has yet
to create is checked at the nearest ancestor that exists, which is ADR 0103 § 6's own answer for an
absent `optional` include; the configured spelling is kept rather than the canonical path the check
returns. Both reasons are in the module doc's *§ 5's directory check*.

**§ 6 is `store` = `publish` then `maybe_gc`.** `Cache::gc` is the same walk without the roll, ready
for the `nvs cache gc` the ADR offers as the deterministic escape hatch; nothing on the read path
reaches either, which is what the new test's middle assertion measures rather than asserts by
inspection. § 7 leaves the three defaults to the implementation and `Eviction::default` decides
them — 256 MiB, one store in a hundred, an 80% hysteresis floor — each with its reason.

**Nothing calls any of it yet**, and that has not moved: there is no payload until `nvs-codegen`
gains a second `Module`, per that module's *Known gaps*, which is why the file still opens with
`#![allow(dead_code)]`.

`orient.py`'s `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed.
`[context] adrs` should now drop **0042 § 3** (landed) and carry **§§ 5-6** only while the bench is
open; this session sliced §§ 5-6 by hand.

## Next group

**Stage 5's last check, then stage 6's boot refusals.** The first shares no files with the other
two: `tools/bench.py` alone, then `crates/nvs-config/`'s tree and its resolver tests.

- [ ] **The warm-start bench** — stage 5's remaining acceptance check:
      `python tools/bench.py --warm-start --max-ms 10`, wanting `warm start` on stdout. `bench.py`
      is today a PHP/Node/Bun comparison harness with no such flag. Decide what *warm* means while
      the pipeline stores nothing — the honest answer is a second `nvs run` of one script with the
      OS caches hot, reported as the CLI's floor, and said plainly to not exercise `Cache::load`
      until the codegen redesign lands. Anchors: `tools/bench.py:325`, `tools/bench.py:240`.
- [ ] **ADR 0073's four boot refusals** — `a_schedule_entry_with_no_scope_refuses_the_boot`,
      `a_malformed_cron_refuses_the_boot`, `a_scheduled_script_outside_the_spawn_roots_refuses_the_boot`,
      `a_fleet_scope_with_no_shared_store_refuses_the_boot`. None of the four exists: `[[schedule]]`
      deserializes and is never validated. Anchors: `crates/nvs-config/src/tree.rs:474`,
      `crates/nvs-config/tests/resolve.rs:589`.
- [ ] **ADR 0074's two, and the clause that needs one implementation rather than two** —
      `cors_star_origins_with_credentials_true_is_refused`, `same_site_none_without_secure_is_refused`,
      `the_same_two_refusals_come_from_config_set_as_from_the_boot`. Anchors:
      `crates/nvs-config/src/tree.rs:342`, `crates/nvs-config/tests/resolve.rs:589`.

## Backlog

- A second `Module` in `nvs-codegen` and a named symbol for every baked address — the cache's
  payload (`crates/nvs-cli/src/cache.rs` *Known gaps*).
- Reading `[cache] dir` and `[opcache]`'s three eviction keys into a real `Cache`, once the compile
  pipeline has a call site for one (ADR 0042 § 7).
- `nvs cache gc` / `nvs cache clear` — ADR 0042 § 6's escape hatch; the walk exists, the commands
  do not.
- Item 18's `Core\Secret::reveal()` is not in the registry (`docs/implementation-plan.md`).
- `Live::admit`'s same-class check is asked of the answer, not the argument
  (`crates/nvs-runtime/src/graph.rs` *Known gaps*).
