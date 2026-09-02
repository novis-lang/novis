# Handoff

## State

**ADR 0084 § 1's roster is complete.** `Core\Queue::push`, `::status`, `::cancel` and `::stats`
are all live in `crates/nvs-stdlib/src/queue.rs`, with `Core\Queue\Id`, `Core\Queue\Stats` and
`Core\Queue\State` in the registry. That module's known gaps say what the surface still owes
around them; nothing on § 1's list is missing any more.

**`Core\Queue\Stats` is a class whose four counters are members**, not a record returned by value:
a `Core`-owned instance has no property a program can reach, so `$stats->pending()` is the
spelling and `$stats->pending` resolves to nothing. `Core\Db\Write` is the same shape for the same
reason. ADR 0084 § 1's body now carries the annotation — `::stats(string $queue): Queue\Stats` —
so the decision has one home rather than living only in code.

**`stats` reads all four out of one aggregate**, `COUNTS` in that module, so the counters describe
one instant rather than four. `attempts` sums `nvs_jobs` alone and the depth is a scalar subquery
over `nvs_dead_jobs`, because `DEAD_TABLE`'s doc deliberately decides only `id` and `queue` on that
table. The `0` and `1` literals no `const` can reach are held to the enum by
`queue_statements_agree_with_the_state_enum`, and the slot/member/index agreement by
`every_stats_counter_reads_the_slot_its_member_is_named_for`.

**Stage 8's acceptance line moved but did not close.** `examples/queue.nvs` compiles now and runs;
it fails at line 54 instead, where `push`'s `{args: {order: 7}}` is an ADR 0036 shape and the JSON
encoder refuses it — "an instance of `$shape{order}` has no JSON encoding". Past that the fixture
still needs a worker for `claimed 1`, `ran` and `retried`, and § 4's claim statement is unwritten.
The `nvs queue migrate` check has no subcommand behind it at all: `nvs-cli` has no `queue` arm.

**`orient.py`'s pack was short in the same places the last two sessions named**, all still unfixed:
`[context] modules` names no `nvs-config/src/*` and owes `nvs-stdlib/src/queue.rs`; `[context]
adrs` printed ADR 0067 §§ 1, 9 and 13 but this session needed ADR 0084 §§ 1 and 6, which it had to
slice by hand.

## Next group

**The two things standing between stage 8's checks and a worker, over
`crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/queue.rs` and `crates/nvs-cli/src/main.rs`.**

- [ ] **A shape encodes as a JSON object, so `push`'s `args` can carry one** — ADR 0084 § 1, ADR
      0036. `Encodable::document` is at `crates/nvs-stdlib/src/json.rs:444` and the struct at
      `crates/nvs-stdlib/src/json.rs:426`; the caller that trips over it is `payload_of` at
      `crates/nvs-stdlib/src/queue.rs:838`. A shape has field names and no `#[Json\Derive]` to
      carry, so the object spelling is the only answer that does not make `mixed` a lie — decide
      it and record it in ADR 0084 § 1 beside the `Stats` note if it changes that surface.
- [ ] **`nvs queue migrate` creates both of § 2's tables** — ADR 0084 § 2. The subcommand enum is
      `crates/nvs-cli/src/main.rs:139`; the schema is read off `crates/nvs-stdlib/src/queue.rs:95`
      and `crates/nvs-stdlib/src/queue.rs:104`, whose docs say those constants are the one home for
      what the columns are. `docs/agent/loop-goal.toml:2971` is the check and it wants `jobs` and
      `dead_letter` in a `--dry-run`'s output. `state`'s column takes `Core\Queue\State`'s ordinal
      and `attempts` must be summable as `bigint` — `COUNTS` casts, so the width is free.

## Backlog

- § 4's worker: the claim statement, the visibility timeout, the retry ladder — ADR 0084 §§ 4 and 6.
- `limits` and `grants` on `push` wait on a shape parameter, gap 1 in `queue.rs` — same blocker as
  `Core\Db::open`.
- `$args` does not refuse a `secret`, gap 2 in `queue.rs` — `CoreTy::Mixed` carries no qualifier.
- `key`'s dedupe is racy at `read committed` until `nvs queue migrate` adds the partial unique
  index, gap 3 in `queue.rs`.
- Four of ADR 0067's five drivers have no statement path, gap 5 in `queue.rs` and gap 2 in `db.rs`.
- `[context]` in `docs/agent/loop-goal.toml` owes `nvs-stdlib/src/queue.rs` under `modules` and
  ADR 0084 §§ 1 and 6 under `adrs`.
