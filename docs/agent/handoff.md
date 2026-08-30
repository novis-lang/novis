# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
Items 31, 32 and 33 are closed. **Item 34 is fifteen of seventeen and its `[[check]]`'s case list is
now complete** — all fourteen cases are written, M1 last — so that check should go green and the
driver's next failure will be a different one. What item 34 still owes has no case in that list:
**D7** (`decodeAs<T>` is a `FATAL` for an array or enum field) and **D10** (`#[Api]`'s `tags`,
`security`, `errors`, `example` absent from `nvs build --openapi`).

**`Core\Task::afterResponse` exists.** `nvs_stdlib::task`'s third row registers the closure;
`crates/nvs-runtime/src/deferred.rs` is the one home for everything that follows — when it runs on a
host with no response, why a request that ended by a throw, an `exit` or a `FATAL` runs none of it,
and why each registration is a group of one. `crates/nvs-cli/src/main.rs:782`'s task body drains it.
**Only the request's own task may register**: `Ctx::child` and `Ctx::isolate` are born sealed,
because a queue on a child would be drained by nobody, and the refusal is a `RuntimeError` at the
call site. ADR 0072 § 6's body states that and the no-response trigger now. Two known gaps are in
that module's doc, both waiting on a host that holds more than one tree: § 7's `max_concurrent` is
**not** counted (one tree per process cannot reach a cap of 256, and no `.nvst` can set the
directive), and an isolate has no drain of its own. `deferred.deadline` joined
`nvs_config::DIRECTIVES` as the `Runtime` half of the block. Both refcount edges are valgrind-clean
— the drain and the un-run `Ctx::drop` path — via
`MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1075,
differential 206.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** This session added `0071 §§ 1, 5` to `adrs` and
`crates/nvs-stdlib/src/json.rs` + `crates/nvs-types/src/derive.rs` to `modules`, for the group
below. Still missing, each proven by a session that had to peek it: **0072 §§ 6-7** (this
session's), **0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 4-5**, **0053 §§ 1-3**, **0007 §§ 2-3**,
**0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6** and
**0096 §§ 1-1a**; and in `modules`, `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-runtime/src/host.rs`, `crates/nvs-runtime/src/script.rs`,
`crates/nvs-stdlib/src/task.rs`, `crates/nvs-config/src/snapshot.rs`,
`crates/nvs-config/src/directive.rs`, `crates/nvs-types/src/layout.rs`,
`crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-types/src/expr/args.rs`,
`crates/nvs-types/src/retrieval.rs`, `attributes.rs`, `crates/nvs-stdlib/src/arr.rs`,
`serialize.rs`, `crates/nvs-hir/src/errors.rs` and `crates/nvs-ir/src/lower/call.rs`. `orient.py`
still warns that `crates/nvs-host/src/budget.rs` matches nothing, which is the forward anchor its
own comment describes.

## Next group

**D7's two remaining halves and the case that closes them.** The file set is
`crates/nvs-stdlib/src/json.rs` with `crates/nvs-types/src/derive.rs` and
`crates/nvs-runtime/src/object.rs`'s `CodecTy`; ADR 0071 §§ 1 and 5 are in the pack now.

- [ ] **D7a: an `array<T>` field decodes** — `docs/reference/findings.md:219` is the finding and
      `crates/nvs-stdlib/src/json.rs:844` its gap 2. The encode half already handles it; what the
      decode half needs is an element codec beside the class label, the way
      `crates/nvs-stdlib/src/json.rs:1350`'s `decode_nested` carries a nested field list and
      `crates/nvs-stdlib/src/json.rs:1005`'s `decode_each` already runs one decode per element for
      `decodeAs<array<C>>`. `crates/nvs-runtime/src/object.rs:495` is the `CodecTy` a field erases to.
- [ ] **D7b: an enum field decodes** — same finding, same gap. An enum's backing value is what
      crosses, so this is the reverse of the encode arm rather than a new shape:
      `crates/nvs-runtime/src/object.rs:518` is the `Opaque` variant that swallows it today and
      `crates/nvs-types/src/derive.rs:77` is where a field's declared type is read for the codec.
- [ ] **The case both are closed by** — `tests/conformance/core/json-derives-an-array-and-an-enum-field.nvst`,
      one round trip: encode, then `decodeAs<T>`, asserting the values back rather than the JSON
      text, so a decode that silently drops a field fails where a text comparison would not. The
      existing twin to write it beside is
      `tests/conformance/core/json-decodes-a-nested-field-and-a-promoted-one.nvst:1`, and
      `crates/nvs-stdlib/src/json.rs:844` is the gap both halves close.

## Backlog

- **D10** — `#[Api]`'s `tags`, `security`, `errors`, `example` reach no `--openapi` document;
  `docs/reference/findings.md:232`, and `crates/nvs-cli/src/openapi.rs` is the writer.
- **Item 35** — the docs findings; `no_registry_card_cites_an_adr` is its `cargo-named` check.
- **Stage 9** — ADR 0119's expression `catch`, items 21–23, anchors already written.
- **§ 7's `max_concurrent` and an isolate's drain** — both named in
  `crates/nvs-runtime/src/deferred.rs`'s known gaps, both waiting on the server.
- **A deferred closure's throw has nowhere to go but the diagnostic stream** — ADR 0072 § 6 wants
  ADR 0020's ladder and `Core\Log`, and neither exists; `deferred.rs`'s `run_one` is where the
  report is written.
- **`nvs test`'s runner drains no deferred work** — `crates/nvs-cli/src/runner.rs:331` spawns its
  own root task and never calls `run_deferred`, so a `#[Test]` that defers loses it silently.
