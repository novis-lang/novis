# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stages 0, 2 and 3 have
landed, and stage 4 is whole but for two of the five unit tests its check names.** Stage 1 is goal
15's whole list; stage 5 is untouched.

**`Core\Request::json()` answers out of a hold now.** `crates/nvs-runtime/src/ctx/inbound.rs:464`'s
`HeldValue` is the carrier's one reference on a program's behalf — a `Value` with the `maxDepth` it
was decoded at, and a `Drop` that gives the reference back. `Inbound::decoded` holds it, the member
fills it on the first reading only, and a call naming another cap decodes again and leaves it alone,
which is what keeps `{maxDepth: 2}` throwing over a document the default depth already read.

**The release is taken in `Ctx::drop`, not left to the field.** A carrier is a field of the context,
so its own drop runs after `crate::object::sweep`, which would then reach an allocation about to be
freed a second time; the `pending` line one above it is the precedent and now has a twin.

**The rule fragment is `status: shipped`**, guarded by
`tests/conformance/core/a-request-json-answers-the-same-document-twice.nvst`. Acceptance is still red
on `examples/json-body.nvs`, stage 5's fixture and unwritten — an open item, not a regression.

## Next group

**Stage 4: the codec-class fixture, and the two unit tests that need one** — one file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-runtime/src/object.rs`.

- [ ] **A test fixture building a class with a derived JSON codec**, beside the tests that need it at
      `crates/nvs-stdlib/src/request.rs:5349`. `ClassTable::define` plus
      `crates/nvs-runtime/src/object.rs:1194`'s `set_codec` is half of one: hydration runs the class's
      real constructor through `crates/nvs-runtime/src/object.rs:2798`, which faults on a class with
      no `CONSTRUCTOR` row, so the fixture owes a native `unsafe extern "C" fn` writing its arguments
      into slots. `crates/nvs-stdlib/tests/allocation_policy.rs:288` is the nearest shape.
      `rule:core-classes/derive-field-list`.
- [ ] **`json_as_hydrates_a_declared_type_and_never_shares_an_object`** over that fixture, at
      `crates/nvs-stdlib/src/request.rs:5349`'s neighbourhood: two `jsonAs` calls answer two objects
      whose `obj_ptr()` differ, which is why that member holds nothing where `json()` holds its value.
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
- [ ] **`json_and_json_as_share_one_reading_of_one_body`**, same file and fixture: both members claim
      `BodyNeed::Octets`, so the second answers out of the hold rather than a drained wire.
      `crates/nvs-stdlib/src/request.rs:2206` is the member, and its `check_codec` runs *before* the
      claim — so a codec-less class cannot stand in for the fixture, and this test cannot be written
      without the item above. `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.

## Backlog
- Stage 5: `examples/json-body.nvs` and `nvs run --request <file>` — the red acceptance check.
  `docs/agent/loop-goal.md` § *Standing decisions* owns the flag's shape.
- `rule:security/derived-codec-qualifiers`' other half has no diagnostic: a `T` whose text fields are
  unqualified must be refused at a `jsonAs<T>()` call site.
  `tests/conformance/reject/a-json-body-hydrated-into-an-unqualified-type-is-refused.nvst` names it.
- Valgrind reached this refcount edge through the `nvs-stdlib` unit test rather than a fixture: no
  `.nvs` file answers a request until `nvs run --request` lands, so `tools/leak-check.sh` has nothing
  to point at. Green — the only definite-loss record is `nvs_stdlib::instance::shape_descriptor`'s
  deliberately leaked `ClassTable`, which a pre-existing test raises on its own.
