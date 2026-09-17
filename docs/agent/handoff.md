# Handoff

## State

**Goal 64 — a `bytes` parameter binds on every driver — has just started; nothing of it has landed yet.** Goal `core-class-tests`'s whole list is this goal's Stage 1 floor.

What is settled before the first session: the read half is built — `decode_column`
(`crates/nvs-db/src/tds/value.rs`) answers `varbinary`, `binary` and `image` as `bytes` — and so is the
mechanism a declaration that varies with the bound *values* needs. `TdsPlan::declared`
(`crates/nvs-db/src/tds/plan.rs:38`) carries the `@params` text a plan was compiled against and compares
it on every lookup, unpreparing a plan a call does not fit, which is what `declarations`
(`crates/nvs-db/src/tds/rpc.rs:315`) already needed to widen a marker to `nvarchar(max)`. A session must
not re-decide two things because of that: ADR 0067 § 1's cache key — SQL text plus expansion arity —
stays as it is for every driver, and a `bytes` is never rendered into the SQL text as a `0x…` literal
(the goal's *Standing decisions*).

## Next group

**Stage 2: the bound form** — one file set: `crates/nvs-db/src/tds/rpc.rs`, `crates/nvs-db/src/pg.rs`.

- [ ] **A bound value says which form it is** — `crates/nvs-db/src/tds/rpc.rs:153`'s `bind` answers
      `Vec<Option<Vec<u8>>>` of UCS-2 text, which is why every marker can share one `nvarchar`
      declaration and why a `bytes` has nowhere to be. The element gains its form — text or binary —
      and nothing else about the list moves. `rule:core-classes/db-column-types` is what the two forms
      answer to.
- [ ] **`encode` stops refusing a `bytes`** — `crates/nvs-db/src/tds/rpc.rs:452` reads the value's tag
      and returns `InvalidInput` for a `bytes`; that arm produces the binary form instead, which is
      where the decision is made once and before anything is written. `crates/nvs-db/src/pg.rs:1817` is
      the driver that already does it, and the shape to agree with rather than invent.
- [ ] **Both halves of the refusal go together** — `crates/nvs-db/src/tds/rpc.rs:372`'s `text_param`
      and `:269`'s `text_of` state the same refusal one layer down, and a value that gets past one and
      not the other is a value this driver sends wrong (the goal's *Standing decisions*).

## Backlog

- **Stage 3, the declaration and the wire** — `crates/nvs-db/src/tds/rpc.rs:315`'s `declarations` gives
  a binary marker its own `varbinary` entry and `text_param` writes it as itself; the plans are told
  apart by the comparison `crates/nvs-db/src/tds/plan.rs:38` already makes. Same file set as stage 2.
- **Stage 4, the round trip** — `crates/nvs-db/src/tds/testing.rs`'s scripted server for the wire
  shape, then `python tools/db-matrix.py --all` for the claim that a real SQL Server accepts it.
- **Stage 0's five sentences** — each is rewritten in the slice that makes it wrong, not after it; the
  goal's *Stage 0* lists them with their files, and `crates/nvs-db/src/tds/mod.rs:88` gap 1 is the last
  of them.
- When this goal's last check goes green the driver takes goal `gap-zero`.
