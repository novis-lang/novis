# Handoff

## State

**Item 11 is closed.** `OWED_A_CASE` in `crates/nvs-stdlib/tests/conformance_coverage.rs:583` is
empty and the const's doc says why it stays: an empty allowance is what makes
`every_error_path_is_asserted_or_declared_unreachable` a ratchet rather than a roster. Every
`Fault::` site in `nvs-stdlib` with a findable stem is now either caught by a case or carries a
`unreachable from source` comment naming what refuses the call first.

**The driver's failing acceptance check is closed.**
`tests/conformance/reject/a-route-table-refuses-its-seven-contradictions.nvst` is written and
passes. One program carries all seven — `E0748`, `E0751`, `E0750`, `E0753`, `E0752`, `E0754`,
`E0759` — and the assertion is that the compilation reports *all* of them rather than stopping at
the first, which is the one thing the seven unit tests in `crates/nvs-types/tests/routes.rs`
cannot see from one error each.

**This session's five judgements.** `validate.rs`'s two `isIp` `version` guards and `math.rs`'s
`round_mode` are declarations: `IP_VERSION`'s `4|6` and `ROUND_OPTIONS`' `Core\RoundMode` refuse
every other value at `E0401`, literal and binding alike, and the enum's six cases are the whole
roster the match arms cover. `debug.rs:191` and `path.rs:612` are `next_slot`/`value_at`'s shared
post-condition. `debug.rs:147` is the odd one: no *diagnostic* refuses it — the only diagnostic
sink that can fail is `OutputSink::Stderr`, and nothing in the language moves the channel, so it
is the host's failure and no case can spell it.

**Found, not as the handoff predicted:** `test.rs:677` (`compareTo` answering a non-`int`) is
**reachable** from source, not a post-condition — nothing requires a compared class to implement
`Comparable`, so a `compareTo(): string` reaches the fatal. It got a case, not a declaration.

**Untouched:** item 12's classification (`UNCLASSIFIED`, `crates/nvs-stdlib/src/registry.rs`).
Also still carried: `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale, a
`bytes` array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last two have
playbook bullets under *Writing a test case*.

**Orientation gaps.** `[context] adrs` still owes `0085 §§ 1-4` and `0071 §§ 2, 7`; the pack
printed no ADR 0077/0102 section for the route grammar this session's first slice needed, which
cost nothing only because `crates/nvs-types/tests/routes.rs` holds the seven shapes verbatim.
Carried: the playbook is filtered to the paths the *item* names, so an item-11 slice naming only
its `src/<class>.rs` never sees the three declaration-window bullets (`playbook.md:835`, `:2264`,
`:3013`); naming `crates/nvs-stdlib/tests/conformance_coverage.rs` in each slice's anchors pulls
them in.

## Next group

**Item 12, the qualifier classification — ADR 0088 § 2. Shared file set:**
`crates/nvs-stdlib/src/registry.rs` (the `UNCLASSIFIED` roster and what a row may say, `:490`)
plus one `crates/nvs-stdlib/src/<class>.rs` per slice. The judgement per parameter is which of the
four marks it is — *contagious* for the overwhelming majority, *sink* where the content becomes an
instruction, *launder* where the member is the boundary that clears it, *neutral* where the bytes
are never read as anything.

- [ ] **The first class off the `UNCLASSIFIED` roster.** Run the gate
      (`every_member_parameter_carries_a_qualifier_classification`) to get the roster in the order
      it reports; take the class it names first. Anchors:
      `crates/nvs-stdlib/src/registry.rs:490`, `crates/nvs-stdlib/tests/conformance_coverage.rs`.
- [ ] **The next two classes on the same roster**, same file set, provided the first slice left
      you well short of the ceiling — the marks are one judgement per parameter and a class is
      cheap once `registry.rs`'s row shape is loaded.
- [ ] **Item 10's floor, as the fallback** if item 12 turns out to want a design call: `python
      tools/gaps.py` names the three thinnest members of the thinnest class, with anchors.

## Backlog

- `router.rs:39`'s *Known gaps* 1 and 2 are stale — the table is built and both members fold
  (`crates/nvs-stdlib/src/router.rs`).
- A `bytes` array key ICEs in `nvs-ir`; `catch (Core\Error $e)` panics rather than diagnosing
  (`docs/agent/playbook.md`, *Writing a test case*).
- ADR 0013's `compareTo` is not enforced at the declaration, so `test.rs:677`'s fatal is a live
  backstop — `crates/nvs-stdlib/src/test.rs:656`'s *known gap 1*.
- `[context] adrs` owes `0085 §§ 1-4` and `0071 §§ 2, 7` (`docs/agent/loop-goal.toml`).
