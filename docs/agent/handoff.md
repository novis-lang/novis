# Handoff

## State

**Goal 6, M7 — ADR 0079 § 18 is whole in both its mechanisms.**
`#[Test(server: true)]` binds `127.0.0.1:0`, serves the program under test on it and hands the test
the address through `Core\Test::serverUrl(): ?string`; the listener is a **sibling** of the test's
isolate (§ 16 reads leftovers off the test's own task) and is retired when that test joins.
`crates/nvs-cli/src/runner.rs`'s `TestServer` is the one home of the bind, the default-policy
`Serving` it runs under, and why its handler holds a `Weak` of the unit rather than an `Rc`. § 18's
body now records the landed surface, including the cost the mechanism carries: a test reaching its
own listener needs ADR 0058's `net.connect` **and** `net.internal` granted, because
`Core\Http\Client` is how a program speaks HTTP. `crate::script::granting_ctx` is where this crate's
tests grant that pair.

**What is still owed of § 18 is the first mechanism's bag**: `Core\Test::request` carries no
`{headers: …}` and no body, and `crates/nvs-stdlib/src/test.rs`'s module doc (§ *§ 18's in-process
request, and what of it is still owed*) is the one home of why — a body needs a
`nvs_runtime::RequestBody` over held bytes, which nothing in `nvs-stdlib` builds.

**ADR 0079 § 14's updater is still blocked** on the fact its helper's doc comment at
`crates/nvs-stdlib/src/test.rs` records: splicing needs the `$expected` literal's span, and no
runtime record holds one.

## Next group

**The last two of ADR 0079's runner, both through `crates/nvs-cli/src/runner.rs`** —
`run_in_isolate` is still where a case's options are read and the child's context is armed, and
`mod tests` is where each check's named test goes. The first is the one the driver's acceptance
check is failing on, so take it first.

- [ ] **`#[Test(db: "test")]` runs inside a transaction the runner rolls back** — ADR 0079 § 17, at
      `crates/nvs-cli/src/runner.rs:832`, where the option is read and the transaction has to be
      opened before the isolate starts and rolled back after it joins. `crates/nvs-cli/src/runner.rs:1041`'s
      `wants_server` is the option-reading shape landed this session and `crates/nvs-cli/src/runner.rs:1074`'s
      `TestServer` is the bind/retire pair to copy — a connection is the same lifetime question as a
      socket. Needs the same reachable PostgreSQL goals 4 and 5 need, and a `[db.test]` block the
      fixture has no `nvs.toml` to carry: decide there whether the runner's test builds the snapshot
      in Rust (as `crate::script::granting_ctx` does for capabilities) or the fixture gains a tree.
- [ ] **A synthetic request carries headers and a body** — ADR 0079 § 18's first paragraph and its
      worked example's `{headers: …}` bag, at `crates/nvs-stdlib/src/test.rs:353` (the `request`
      row, which gains a trailing options bag) and `crates/nvs-cli/src/runner.rs:434`
      (`UnderTest::answer`, which is what pushes them onto the carrier). The body half needs a
      `nvs_runtime::RequestBody` over held bytes and is the larger of the two.
- [ ] **`Core\Test::server(): Core\Http\Target` removes the outbound grant a `server: true` test
      needs** — ADR 0079 § 18 leaves the choice open and names both shapes, at
      `crates/nvs-stdlib/src/test.rs:441` (`SERVER_URL_DOC`, beside the row) and
      `crates/nvs-stdlib/src/http.rs:190` (`TARGET`, whose two slots a constructor would have to
      fill). Only worth it if a second `server: true` case is written and the grant is felt twice.

## Backlog

- ADR 0079 § 14's `nvs test --update` needs the `$expected` literal's span — `crates/nvs-stdlib/src/test.rs`'s helper doc.
- Raw/unparsed body access for an arbitrary content-type — `Core\Request`'s module doc, ADR 0024 *Revisiting*.
- `[[schedule]] fleet` is unarmed — ADR 0073 § 5.
