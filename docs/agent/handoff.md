# Handoff

## State

**Goal `unowned-closures`. The register is unchanged at `unowned: 15` and goal-owned is down to 52.**
The 15 unowned are the scheduling questions, none of them this goal's own gaps, so
`python tools/owners.py`'s `unowned: 0` still turns on answers only the user can give.
`--deferrals` is green.

**`crates/nvs-types/src/intrinsics.rs` gap 2 is struck, built rather than deferred: the goal's one
ADR is [0189](../decisions/0189.md), the prepared-pattern channel.** The checker files what it
prepared under the **call's** span (`crate::expr_table::Prepared`), the lowering reads it there and
emits it as argument 0 of that call for a closed roster
(`nvs_stdlib::registry::PREPARED_MEMBERS`), and the word rides in the unit's own code — so the
artifact carries it with nothing serialized beside it. `rule:expressions/preparation-preserves-behaviour`
is amended to state that, and `crates/nvs-ir/tests/prepared_patterns.rs` guards it.

**What travels is a fact, never an object, and that bound is why one row has a passenger.** A
compiled pattern is an `Rc` on one core's cache, so `Core\Regex::compile` is handed the **tier** its
literal settled in and spends the compile itself; the plan-carrying grammars (CLDR, `printf`, SQL)
prepare by validating alone and joining the roster is what would change that — ADR 0189 § 6.

**A first-class callable to any `Core` member with an options bag panics in lowering**, which is
pre-existing and not this channel's: `Core\Regex::matches(...)` panics identically, off the roster.
The thunk writes its prepared slot regardless, which is the right ABI; nothing can reach it today.

**Gap numbering in that file is deliberately not contiguous — do not renumber it.** The block now
opens at gap 3; `crates/nvs-ir/src/lib.rs` is the same shape, so this is the tree's practice.

## Next group

**Stage 3: the checker's intrinsic pass, the gap left in it** — one file set:
`crates/nvs-types/src/intrinsics.rs`, `crates/nvs-db/src/sql.rs`, and
`tests/conformance/reject/`.

- [ ] **Refuse an unterminated string literal in a literal query, and amend the doc that declines
      it** — `crates/nvs-types/src/intrinsics.rs:107` (gap 3) is the slot and its `Decided:`
      sentence is "Refuse at compile time and amend `nvs_db::sql`'s doc". The disagreement to
      settle is `crates/nvs-db/src/sql.rs:53`, whose "malformed SQL is the server's diagnosis"
      paragraph declines it in the other direction; `rule:core-classes/db-literal-query-checking`
      is what the refusal has to stay inside, and `check_sql` at
      `crates/nvs-types/src/intrinsics.rs:606` is where it lands.
- [ ] **Pin the refusal with a reject case and declare its code** —
      `crates/nvs-diagnostics/src/lib.rs:3686` is where the types band's next free code `E0824`
      is declared, beside `E0823`; `tests/conformance/reject/` takes a new `.nvst` with no
      registration, and `crates/nvs-types/src/intrinsics.rs:890` (`report_query`) is the refusal
      it has to read like.

## Backlog

- The plan-carrying grammars joining `PREPARED_MEMBERS` — ADR 0189 § 6, no owner and unscheduled.
- A callable reference to a `Core` member with an options bag panics — `crates/nvs-ir/src/lower/mod.rs:3927`.
- `unowned: 0` needs the user's answers to the 15 scheduling questions — `python tools/owners.py`.
