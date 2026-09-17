---
milestone: post-parity
---
# Loop goal 64 — a `bytes` parameter binds on every driver

A `bytes` value bound to a statement reaches SQL Server as a `varbinary` parameter, the way it already
reaches the other four backends, so [ADR 0067 § 9](../decisions/0067.md)'s `BINARY`/`BLOB`/`BYTEA`
row is whole in both directions on every driver and one program binds one value list wherever it is
pointed. Afterwards `crates/nvs-db/src/tds/mod.rs` owns no gap.

## Why here

Directly after goal `core-class-tests` and in front of goal `gap-zero`, because that goal's gate is
that no register item names a goal and `crates/nvs-db/src/tds/mod.rs` gap 1 names this one. Goal
`m8-db-queue` took the row side of the same ADR and its stage gate names every tag it closed, so this
one was never its.

It is here rather than earlier because what the gap called an open question is already answered on
disk. The read half is whole — `decode_column` (`crates/nvs-db/src/tds/value.rs`) answers `varbinary`,
`binary` and `image` as `bytes` like every other driver. And the write half's hard part, a `@params`
declaration that is a function of the *values* a call binds rather than of the statement alone, is
built: `TdsPlan::declared` (`crates/nvs-db/src/tds/plan.rs:38`) carries the declaration a plan was
compiled against and compares it on every lookup, treating a mismatch as a miss that unprepares the
plan it did not fit. That exists because `declarations` (`crates/nvs-db/src/tds/rpc.rs:315`) already
widens a marker to `nvarchar(max)` past `NVARCHAR_CHARS`, which is the same problem in a narrower
shape. So § 1's cache key does not have to move to carry a `bytes`, and this goal is a driver's
encoder rather than a rule change for five drivers.

## Stage 0 — the catch-up

The sentences on disk that go wrong the day this goal is green, each with the file that holds them:

- `crates/nvs-db/src/tds/mod.rs:88` — gap 1 itself, and the sentence above the `# Known gaps` block
  that calls the refusal *this driver's only departure* from § 9's table.
- `crates/nvs-db/src/tds/rpc.rs:123-128` — *a parameter that is not UTF-8 is refused rather than
  reinterpreted*. The reason stays whole — `nvarchar` → `varbinary` on SQL Server is a reinterpretation
  and never a conversion — and what changes is that a `bytes` is no longer a value with no form to
  send.
- `crates/nvs-db/src/tds/rpc.rs:143-152` and `:448-451` — `bind`'s doc and `encode`'s `# Errors`, both
  of which list a `bytes` among the values with no form on this protocol.
- `crates/nvs-db/src/tds/rpc.rs:269-283` — `text_of`'s refusal message, which names the `bytes` it
  refuses and says an encoding of its own is what it would need.
- `docs/agent/carried-gaps.md` § *Owned* — the row naming this goal, struck when the gap closes.

## Stage 1 — the floor

Goal `core-class-tests`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: a bound value carries its form

`bind` (`crates/nvs-db/src/tds/rpc.rs:153`) answers one list of already-UCS-2 text, which is why every
parameter can go out under one `nvarchar` declaration and why a `bytes` has nowhere to be. The keystone
is that list's element saying which form it is — text or binary — produced where `encode`
(`crates/nvs-db/src/tds/rpc.rs:452`) reads the value's tag, so the decision is made once, before
anything is written, exactly where the refusal is made today.

Everything after this is mechanical: the declaration and the wire write both read that form, and
neither has a second question to ask.

## Stage 3 — the declaration, and the bytes on the wire

`declarations` (`crates/nvs-db/src/tds/rpc.rs:315`) gives a binary marker its own `varbinary` entry
beside the `nvarchar` ones, and `text_param` (`crates/nvs-db/src/tds/rpc.rs:372`) writes a binary value
as itself rather than as UCS-2. Two calls binding the same statement with a `bytes` at different
markers are told apart by the comparison `TdsPlan::declared` already makes, so a plan compiled for one
declaration is never handed a call the other shape.

`rule:core-classes/db-column-types` is what both halves answer to, and the refusal `encode` and
`text_param` each state where it is stops being true in the same slice.

## Stage 4 — the round trip is pinned, on a scripted server and a real one

A `bytes` bound and read back is equal to what went out, over `crates/nvs-db/src/tds/testing.rs`'s
scripted server for the wire shape and over `python tools/db-matrix.py --all` for the claim that a real
SQL Server accepts it. `rule:core-classes/db-one-api`'s promise is that one program binds one value
list whatever it is pointed at, so the case is the same case the other drivers already pass.

## Standing decisions

- **§ 1's cache key does not move.** SQL text plus expansion arity stays the key on every driver. A
  `bytes` at a different marker is told apart by `TdsPlan::declared`'s existing comparison, which is
  one driver's reason to reject a hit rather than another way to spell the key. A session that finds
  itself adding bound types to the key has changed five drivers to fix one.
- **No interpolation, in any form.** § 1's *emulated prepares do not exist in any form* binds here too:
  a `bytes` goes out as a bound `varbinary` parameter and is never rendered into the SQL text as a
  `0x…` literal, however much simpler that would be to write.
- **The refusal is closed in both places or in neither.** `encode` and `text_param` each state it where
  they are, and a value that gets past one and not the other is a value the driver sends wrong.
- **The read side does not move.** `decode_column` already answers § 9's row; this goal adds no column
  type, no `Tag` and no member.
- **What it spends**, per `rule:programs/memory-priority` and written into the module that takes it: a
  bound `bytes` travels as its own bytes instead of as the UCS-2 text it has no form for, and the
  per-connection plan cache holds the declaration string it already holds, one per plan shape.
- **Not this goal**: the other four drivers, whose `bytes` already binds; `Core\Db::stream` and the
  schema half, which `gap-zero`'s register names against their own owners; and anything about how a
  `bytes` is spelled in the language.
- **ADR slots**: none. §§ 1 and 9 state both halves already, and the declaration comparison is
  `TdsPlan::declared`'s own doc. A rule fragment this goal makes wrong is amended in the slice that
  makes it wrong.
