# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.**
`Core\Queue\Stats` carries five counters: `deadAttempts` sums the `attempts` the dead-lettered jobs
used, over `nvs_dead_jobs` alone, so every attempt the queue has made is in exactly one of the two
sums. `crates/nvs-stdlib/src/queue.rs`'s gap 2 is deleted, so `python tools/owners.py --closes
decided-closures` names 25 gaps where it named 26. `mime.rs` (session 0005), `zip.rs`, `path.rs`,
`random.rs` and `uuid.rs` were cleared the same way.

**The fifth counter needed no schema change**, which the sheet priced as one: `schema()`'s
dead-letter table already declares `attempts`, and the module only ever *read* `id` and `queue` off
it. The standing decision that code ahead of a record wins is what settled it — the statements and
the record were what was behind, not `nvs queue migrate`.

Stage 4's first acceptance check is still red on its other three tests
(`json_nesting_is_bounded_by_the_heap_stack_not_the_native_one`,
`a_queue_whose_schema_is_behind_is_refused_at_boot`, `an_xml_element_answers_its_namespace_uri`),
all for the first reason — the member does not exist yet. Checks 2 and 3 likewise.

## Next group

**Stage 4: a queue whose schema is behind is refused at boot** — **not one file set**, which is what
this session found: the judgement is `nvs-stdlib`'s (that is where the acceptance check runs) and the
boot that acts on it is `nvs-cli`'s. `rule:core-classes/queue-storage-is-a-table` owns the dedupe
guarantee the check proves is present; the gap's own `Decided:` sentence is the specification.

- [ ] **`crates/nvs-stdlib/src/queue.rs:64` — the judgement, as a pure function over a live
      `nvs_db::schema::Schema`**: what `schema()` asks for that the database lacks, `nvs_jobs_dedupe`
      first, answered as something a caller can print. Pure so the acceptance test
      `a_queue_whose_schema_is_behind_is_refused_at_boot` is a unit test in this file's `mod tests`
      beside `queue_stats_has_a_fifth_counter`. Delete the numbered gap when it lands — it is the
      module's last one.
- [ ] **`crates/nvs-cli/src/serve.rs:1361` — the boot that refuses.** `queue_on_this_core` is where
      a serving core resolves `[queue]`, and `arm_queue_workers` beside it is what opens the
      connections. The live schema is read the way `crates/nvs-stdlib/src/db/schema.rs:431`'s
      `introspected` reads one — `nvs_db::catalog::query(Read::Columns | Read::Indexes, dialect)`
      then `nvs_db::catalog::assemble` — and `nvs_db::plan::diff` grades the difference. Decide, and
      say in the module doc, whether `workers = 0` (§ 2's enqueue-only deployment) is checked too.

## Backlog

- `crates/nvs-stdlib/src/json.rs` gap 2 — the encoder's explicit heap stack; stage 4 check 1's
  `json_nesting_is_bounded_by_the_heap_stack_not_the_native_one`.
- `crates/nvs-stdlib/src/xml.rs` gap 1 — a computed `namespaceUri()`; stage 4 check 1's fourth test.
- `crates/nvs-stdlib/src/regex.rs` gaps 1–3 — stage 4 check 3, and the `[limits]` directive.
- `crates/nvs-stdlib/src/cldr.rs` gap 1 and `time.rs` gap 1 — the prepared-pattern channel, the
  goal's one ADR slot; stage 4 check 3's second test.
- `crates/nvs-stdlib/src/lib.rs` gap 1 — both registry gates widen past § 12.
- `docs/decisions/0084.md` § 1 says four counters and stays frozen; the rule fragment
  `rule:concurrency/queue-four-members` is where the five are now written.
