# Handoff

## State

**Goal 6, M7 — ADR 0079 § 17 is on disk, and §§ 14, 17 and 18 are the runner's landed surface.**
`#[Test(db: "test")]` runs inside a transaction the runner opens before the method and rolls back
after it, so a test writes no cleanup code and the next test sees a pristine database. The one
decision the mechanism makes is **where** the transaction is opened: on the *test's own* context,
from inside its isolate, because a connection is memoized on the context it was opened on — so the
test's own `Core\Db::connect` reaches that connection and its own `transaction()` is a `SAVEPOINT`
with no special case anywhere. ADR 0079 § 17's body records that and what is still owed of it (the
deadlock message naming the other test). `nvs_stdlib::db::begin_test_transaction` and
`roll_back_test_transaction` are the one home of the reading; `crates/nvs-cli/src/runner.rs`'s
`run_in_isolate` arms them around § 20's whole retry allowance rather than around one attempt.

**What is still owed of § 18 is the first mechanism's bag**: `Core\Test::request` carries no
`{headers: …}` and no body, and `crates/nvs-stdlib/src/test.rs`'s module doc (§ *§ 18's in-process
request, and what of it is still owed*) is the one home of why — a body needs a
`nvs_runtime::RequestBody` over held bytes, which nothing in `nvs-stdlib` builds.

**ADR 0079 § 14's updater is still blocked** on the fact its helper's doc comment at
`crates/nvs-stdlib/src/test.rs` records: splicing needs the `$expected` literal's span, and no
runtime record holds one.

## Next group

**The last of ADR 0079's runner and the first mechanism's bag, sharing
`crates/nvs-stdlib/src/test.rs` and `crates/nvs-cli/src/runner.rs`** — both are `Core\Test` members
whose far side is the runner's own seam, so the two files are open for either.

- [ ] **A synthetic request carries headers and a body** — ADR 0079 § 18's first paragraph, at
      `crates/nvs-stdlib/src/test.rs:1084`, where `nvs_core_test_request` reads its options today
      and neither a `{headers: …}` map nor a body reaches the carrier. The blocker the module doc at
      `crates/nvs-stdlib/src/test.rs:144` names is the body alone — a `nvs_runtime::RequestBody`
      over held bytes — so **headers are landable on their own** and are the slice to take first;
      `crates/nvs-cli/src/runner.rs:1290`'s `UnderTest::answer` is the far side that has to read
      them onto the child's inbound.
- [ ] **`Core\Test::server(): Core\Http\Target` removes the outbound grant a `server: true` test
      needs** — ADR 0079 § 18, at `crates/nvs-stdlib/src/test.rs:1008`, beside `serverUrl` which is
      the string form it would replace. `crates/nvs-cli/src/script.rs:500` is the `net.connect` /
      `net.internal` pair this crate's tests grant today and which a `Target` would make
      unnecessary; `crates/nvs-cli/src/runner.rs:1143`'s `TestServer` holds the address either way.
- [ ] **§ 17's deadlock message names the other test** — ADR 0079 § 17's last paragraph, at
      `crates/nvs-cli/src/runner.rs:1109`, where `wants_db` is read. It needs a table of which test
      holds which transaction, and the shipped runner runs tests one after another, so this is
      reachable only once § 2's parallelism is. Take it last, or move it to `carried-gaps.md`.

## Backlog

- The deadlock message § 17 still owes — `docs/adr/0079-testing-is-a-language-feature.md` § 17.
- `Core\Test::request` has no body — `crates/nvs-stdlib/src/test.rs`'s module doc.
- § 14's updater needs the `$expected` literal's span — `crates/nvs-stdlib/src/test.rs`.
- An `open` pool's bounds are the defaults — `crates/nvs-stdlib/src/db/mod.rs` § *Known gaps*.
