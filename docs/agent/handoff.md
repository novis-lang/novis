# Handoff

## State

**Goal 6, M7 — ADR 0079 § 14 is whole, and §§ 14, 17 and 18 are the runner's landed surface.**
`nvs test --update` splices each failed inline snapshot into the `$expected` literal that asked for
it, and that is the only spelling under which `nvs test` writes to a file. The mechanism is three
crates: `nvs_types::ExprTypeTable::inline_snapshots` is a compile-time row per **written**
`Core\Test::assertMatchesInline` (the literal's file, span and enclosing method — the span exists
nowhere else, a helper being called with a value rather than with the expression that built it),
`nvs_runtime::SnapshotMismatch` is the pair of texts a run contributes, and `nvs-cli`'s
`update_snapshots` joins them by the expected text **within the test that produced it** and refuses
to write an ambiguous join. ADR 0079 § 14's body is the one home of that reading and of why the
spliced literal is single-quoted.

**What is still owed of § 18 is the first mechanism's bag**: `Core\Test::request` carries no
`{headers: …}` and no body, and `crates/nvs-stdlib/src/test.rs`'s module doc (§ *§ 18's in-process
request, and what of it is still owed*) is the one home of why — a body needs a
`nvs_runtime::RequestBody` over held bytes, which nothing in `nvs-stdlib` builds. Headers are
landable on their own.

**§ 17's deadlock message is blocked, not open.** Its paragraph asks for a serialization failure
between two `db:` tests to name the other test; the shipped runner makes and joins isolates one at
a time (`runner.rs`'s module doc, § *What is owed*), so two `db:` transactions never overlap and
there is no contention to report. It becomes writable when § 2's parallelism does, and not before.

## Next group

**§ 18's first mechanism and the members around it, sharing `crates/nvs-stdlib/src/test.rs` and
`crates/nvs-cli/src/runner.rs`** — every item below is a `Core\Test` member whose far side is the
runner's own seam, so the two files are open for any of them.

- [ ] **A synthetic request carries headers** — ADR 0079 § 18's first paragraph, at
      `crates/nvs-stdlib/src/test.rs:1087`, where `nvs_core_test_request` reads its options today
      and no `{headers: …}` map reaches the carrier. `crates/nvs-cli/src/runner.rs:479`'s
      `UnderTest::answer` is the far side that has to read them onto the child's inbound; the
      on-the-wire twin at `crates/nvs-cli/src/runner.rs:1365` already does exactly that from a real
      request and is the shape to copy.
- [ ] **`Core\Test::server(): Core\Http\Target` removes the outbound grant a `server: true` test
      needs** — ADR 0079 § 18's listener half, beside the `serverUrl` row at
      `crates/nvs-stdlib/src/test.rs:332` and its helper at `crates/nvs-stdlib/src/test.rs:1011`.
      Five edits plus three `.nvst` cases (`conventions.md`, *A `Core` member*); the address the
      member answers with is already on the child's context.
- [ ] **A synthetic request carries a body** — the same paragraph and the same helper at
      `crates/nvs-stdlib/src/test.rs:1087`, taken after the headers above. The blocker is named in
      that file's module doc: a `nvs_runtime::RequestBody` over held bytes, which nothing in
      `nvs-stdlib` builds, so this slice is partly in `nvs-runtime`.

## Backlog

- § 17's deadlock message: blocked on § 2's parallelism — ADR 0079 § 17's last paragraph.
- § 2's parallelism: isolates are made and joined one at a time — `nvs-cli/src/runner.rs`'s module
  doc, § *What is owed*.
- `assertMatchesInline` under a `#[TestWith]` row: the join cuts the label back at `#`, so two rows
  of one method share one literal and the second is refused — `update_snapshots`' own doc.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by
  `docs/plan/m7.md`.
