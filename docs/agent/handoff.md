# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `nvs-db`**, after `nvs-cli`,
`nvs-syntax`, `nvs-hir`, `nvs-test` and `nvs-runtime`. 77 items still name nobody, and every one of
them is in `nvs-stdlib`. `python tools/owners.py --check --reasons` stays green.

**`nvs-db`'s fourteen went seven `unowned` and seven out of the gap block.** The seven that left were
each one engine's rule or one stated policy rather than an unwritten shape, which the goal's
§ *Standing decisions* resolves toward *decision*: `ddl.rs` grew § *An engine's own limit is not a
gap here* for SQL Server's unindexable `MAX` columns, SQLite's rowid identity, SQLite's after-the-fact
unique index and SQLite's declared-type affinity; `catalog.rs` moved its unreportable index and its
one-schema-deep read into the § that already holds the consequences that look like omissions; and
`schema.rs` § 11's exclusions are a heading of their own, since "the vocabulary grows only when a
construct exists on every backend" is a policy, not a hole.

**The seven that stayed are one scheduling question and two smaller ones.** § 5's normalization is
recorded as owed by both directions — the emitter's map into a dialect is lossy, so a schema applied
and read straight back describes as a different vocabulary case — and goals `database` and `schema`
are retired with M8's database half carried, so nothing claims it. `carried-gaps.md` § *Unowned* is
**twenty-two entries**; the two added are that one and the db matrix's unpublished `AF_UNIX` leg.

**`python tools/chain.py --check` — this goal's own floor check — was red on one sentence in this
file**, which named the live goals as a *range of numbers*. A range is the form the playbook's
number-citation bullet did not name; it names it now.

**`python tools/verify.py`: 9 of 9 green.**

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-stdlib`'s module
docs, which hold every item that still names nobody. The owner kinds are the goal's § *Standing
decisions* and `python tools/owners.py --help`; `--untagged` is the worklist and `--check
--untagged-is-an-error --reasons` is the gate. Take `db/mod.rs` first: it is the `Core\Db` surface
over the crate just tagged, and where its gaps point at the schema plan the reason is already written
under `carried-gaps.md` § *Unowned*.

- [ ] **Tag `crates/nvs-stdlib/src/db/mod.rs:118`'s eight items** — at
      `crates/nvs-stdlib/src/db/mod.rs:118`, `:141`, `:197`, `:205`, `:225`, `:251`, `:268` and
      `:272`. Item 6 says outright it is "a refusal rather than an unwritten shape", so it is the
      one to weigh for *decision* first; item 8's three run-time refusals that should be compile-time
      are `rule:core-classes/derive-attribute`'s question, which `carried-gaps.md` § *Unowned*
      already carries for `crates/nvs-types/src/derive.rs`.
- [ ] **Tag `crates/nvs-stdlib/src/json.rs:100`'s eight items** — at
      `crates/nvs-stdlib/src/json.rs:100`, `:108`, `:129`, `:143`, `:148`, `:154`, `:158` and
      `:167`. Several are `rule:core-classes/derive-field-types`' roster and read as one decision
      taken once, not eight.
- [ ] **Tag `crates/nvs-stdlib/src/time.rs:62`'s four items** — at
      `crates/nvs-stdlib/src/time.rs:62`, `:71`, `:75` and `:86`.

## Backlog

- The rest of `nvs-stdlib`'s 77 — `ast`, `cldr`, `cli`, `command`, `compress`, `csv`, `debug`,
  `decimal`, `html`, `lib`, `math`, `mime`, `out`, `path`, `regex`, `registry`, `response`,
  `storage`, `test`, `uuid`, `xml`, `zip` (goal `gap-owners` stage 3).
- `--untagged-is-an-error` is the half of the gate still red; it goes green when `nvs-stdlib` is
  done, and `verify.py` runs the full form.
- § 5's schema normalization has no owner and is a real scheduling question for the user
  (`carried-gaps.md` § *Unowned*).
- The db matrix's `AF_UNIX` leg needs `tools/db-matrix.py` to bind-mount a container socket
  directory (`carried-gaps.md` § *Unowned*, `crates/nvs-db/src/matrix.rs` gap 1).
