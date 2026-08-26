# Handoff

## State

**Reading a `string` argument is one tag check across the whole of `mwl-stdlib`, not just in
`str.rs`.** The nine sibling `text()`/`text_of()`/`subject()` helpers — `csv.rs`, `encoding.rs`,
`json.rs`, `path.rs`, `regex.rs`, `time.rs`, `uri.rs`, `uuid.rs`, `validate.rs` — now call
`mwl_runtime::Value::as_text`, and so do the seven other sites that carried the same
`as_str_bytes` + `std::str::from_utf8` chain (`bytes.rs`'s two format readers, `format.rs`'s
`rendered`, `json.rs`'s serializer, `time.rs`'s two zone-slot reads, `uri.rs`'s three). Each keeps
its own wrong-tag message; every one lost an O(n) pass ADR 0009 § 3's tag already discharges.

**Stage 0's item 22 check is green.** `no_member_revalidates_a_string_argument` exists beside
`a_str_member_allocates_its_result_once` in `crates/mwl-stdlib/tests/allocation_policy.rs`: a source
scan over each module's non-test half, banning an `as_str_bytes` whose result reaches `from_utf8`
within twelve lines. `Value::as_bytes` is deliberately outside the ban — a `bytes` carries no
encoding guarantee, so `Core\Encoding`'s `Utf8` scheme validating one is the real check.
Verify is green: **1585** tests, 74 suites, clippy and fmt clean. No refcount edge changed, so no
valgrind run was owed, and no bench sweep either — this removes work, it does not move a design.

**Stage 3 is now the first red check, and it stops at `Core\Out::capture`.** `examples/collect.mwl:47`
is the one fixture of the seven that does not produce its frozen output. `Core\Out` has no module at
all, and `§12 Out::capture` is the single remaining key in
`crates/mwl-stdlib/tests/spec-members-outstanding.txt:17` — so one member closes both the ratchet and
Stage 3. `[context]` in `docs/agent/loop-goal.toml` is re-pointed at it: `crates/mwl-runtime/src/ctx.rs`
in `modules`, `0088 §3` and `0088 §5` in `adrs`, and the twelve conformance-case traps kept
deliberately for Stage 4's corpus rather than left as a stale `--gap` block.

## Next group

`Core\Out::capture`, sharing a new `crates/mwl-stdlib/src/out.rs` with `registry.rs`, `lib.rs` and
`crates/mwl-runtime/src/ctx.rs`. Slice 1 decides what the carrier is; 2 and 3 cannot start before it.

- [ ] **The sink's carrier is a `Core` instance.** ADR 0088 § 5 with spec § 12's paragraph under the
      `Out::capture` row (`docs/spec/01-core-library.md:888`) — the return is **not** a `string`,
      because the captured bytes have already been through the sink and re-emitting them as text
      would escape them twice. `Cli\Text` is the carrier outside an HTTP request and is the one to
      build; `Core\Html\Markup` is M8's. Anchors: `crates/mwl-runtime/src/ctx.rs:138` (`Ctx`),
      `ctx.rs:465` (`buffered`), `crates/mwl-runtime/src/lib.rs:283` (the `OutputSink` re-export).
- [ ] **`capture` redirects the sink for the closure's dynamic extent and always swallows.** ADR 0088
      § 3 — `echo` always has a sink, so this pushes one rather than intercepting a byte stream.
      Buffers nest by call nesting; there is no `ob_*` stack and no implicit flush. `{through:}` takes
      and returns the same carrier. Anchors: `crates/mwl-stdlib/src/registry.rs` (the row, and
      `WRITTEN_CLASS_MEMBERS` if the options bag needs it), `crates/mwl-stdlib/src/lib.rs`
      (`mod out;` — private, so its `CLASS`/`NAME` are `pub(crate)`).
- [ ] **The `.mwlt` case, then strike the ratchet's last key.**
      `crates/mwl-stdlib/tests/spec-members-outstanding.txt:17` is `§12 Out::capture` and nothing
      else, so removing it registers spec Part I whole. `examples/collect.mwl:47` then produces its
      frozen output and Stage 3 goes green.

## Backlog

- `produced()` still builds a `String` and copies it into an `MwlStr`: `bytes.rs:425`, `path.rs:430`,
  `regex.rs:674`, `uri.rs:694`. `crates/mwl-stdlib/src/str.rs` § *A result is written once* is the
  pattern, and its two rejected designs — docs/perf/userland-gap.md.
- `docs/perf/history.ndjson` does not exist; plan item 19 owes it — docs/implementation-plan.md.
- Stage 4's counts are their own work: conformance 436 of 600, differential 90 of 150 —
  docs/implementation-plan.md § *Open now*.
- No `mwl-stdlib` member row carries a qualifier classification, so ADR 0088's fail-closed default
  for an unclassified `string`/`bytes` parameter does not exist — docs/plan/m4s.md.
- `Core\Json::decodeAs<T>` is § 6's one unregistered member — `mwl_stdlib::json` gap 2.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
