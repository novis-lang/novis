# Handoff

## State

**ADR 0007 § 2's conversion table now runs for every scalar source, `mixed` included, so `mwl-ir` gap 20
is down to its two non-scalar rows.** `$any as int`/`as uint`/`as float` are `Helper::TaggedToInt` and its
two twins — one helper per *target*, dispatching on the operand's runtime tag, throwing exactly where
ADR 0066's `Helper::ToIntOrNull` answers `null` over the same row set in `mwl_runtime`
(`helpers.rs:396`'s `to_int` is that set; `to_decimal`'s two-entry-point arrangement is what it now
copies). What still panics in `Lowering::convert` is ADR 0009 § 3's `string` ↔ `bytes` pair and
`array<T> as array<U>`.

**One ordering rule changed with it, and it is the part to know.** A `Ty::Tagged` operand into a
**literal** set is still tested against its own runtime tag *before* the base conversion, so
`1 as "1"|"b"` throws rather than being rendered into the set. An **enum** target is now the opposite:
the operand converts to the enum's backing scalar first and the membership chain compares two integers,
because ADR 0010 § 5 words that row as "exactly the shape `as uint` already has for untrusted input" and
its own example converts `Core\Request::query('status')` — a string at run time — into a case whose value
is an integer. `lower/expr.rs:3052` is the branch; its comment owns why.

`python tools/verify.py` green, 1393 tests. Stage 0 is empty, so the group after the one below is the
first to come from Stage 3 (`examples/collect.mwl`; `Core\Path` is its cheapest slice). **No conformance
case covers any of the above yet** — that is the group below, and it is why it comes first.

## Next group — conformance cases for the `mixed` → scalar and `mixed` → enum rows

**Shared file set:** `tests/conformance/lang/` only. Both cases are new files, picked up with no
registration. Read the throw messages off `crates/mwl-runtime/src/helpers.rs:432` (`"cannot convert this
value to \`int\`"`) rather than guessing them, and copy the caught-throw shape from
`tests/conformance/lang/a-lossy-conversion-throws.mwlt` — `catch` and `$e->message` do not lower at file
scope (playbook).

- [ ] **`a-mixed-value-converts-to-a-scalar-on-request.mwlt`** — ADR 0007 § 6's headline shape without a
      request: a `mixed` holding `"42"`, `2.5` and `7` into `int`/`float`/`uint`, each throw named, and
      the `as ?int ?? -1` twin beside it so the pair reads as one operation. `.agent-tmp/tagged-scalar.mwl`
      is that program already, and it runs; it needs the class-method wrapper and an `--EXPECT--` block.
- [ ] **`a-mixed-value-converts-into-an-enum-case.mwlt`** — ADR 0010 § 5's row 2 from a `mixed`: a
      `mixed` holding the *string* `"1"` reaching `Mode::Read` (this is the rule the ordering change above
      exists for, and the case that pins it), a `mixed` holding `9` throwing
      ``` `9` is not one of `Mode::Read`, `Mode::Write`, `Mode::Admin` ```, and the same two against
      ADR 0047 § 3's named subset `Mode::Read|Mode::Write`. Enum syntax is one line —
      `enum Mode: int { Read = 1, Write = 2 }`. Anchors: `lower/expr.rs:3052`,
      `tests/conformance/lang/a-conversion-into-a-closed-set-of-enum-cases-is-checked-at-run-time.mwlt`.
- [ ] **Delete `.agent-tmp/tagged-scalar.mwl` and `.agent-tmp/tagged-enum.mwl`** once both cases exist;
      they are the same two programs and are untracked scratch.

## Backlog

- `Core\Path`, then `Encoding`/`Hash`/`Uuid`, then `ObjectSet`/`ObjectMap` — Stage 3's
  `examples/collect.mwl`, `docs/agent/loop-goal.md` § *Stage 3*.
- ADR 0009 § 3's `string` ↔ `bytes` conversion rows — `mwl-ir` gap 20's remainder.
- `array<T> as array<U>`'s O(n) element walk — ADR 0007 § 2 row 6, `mwl-ir` gap 20.
- ADR 0007 § 4's promotion table: `$n + $f` and `$n < $f` still fail in codegen — `mwl-ir` gap 19.
- The opaque `object` top has no representation arm — `mwl-ir` gap 21.
- `mwl_types` does not yet refuse ADR 0066 § 3's "cannot fail" `as ?T`, so `convert_or_null` panics on it
  — `mwl-ir` gap 20's neighbour, `lower/expr.rs`'s `convert_or_null` doc comment.

`orient.py` printed everything this group needed; no `[context]` field was missing a selector.
