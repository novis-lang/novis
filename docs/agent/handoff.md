# Handoff

## State

**The failing acceptance check is closed.**
`tests/conformance/core/a-route-table-answers-a-url-for-every-route-it-holds.nvst` is written and
passes. Eight routes over three classes spell every form of ADR 0077 § 2's grammar; completeness is
the *compilation* — an unknown literal name is a compile error, so a route the table never grew a
row for cannot reach the runtime and the file would not build — and correctness is counted over the
sweep, with the labelled lines catching the one failure the counters cannot see, a permutation.

**Item 11 is down to eight lines in `OWED_A_CASE`**, across five files. Seven sites were declared
unreachable this session, each judged at the site: `router.rs`'s three (the `value_to_string`
post-condition `crate::uri::scalar_text` already records, and the two prepared-link guards, whose
`template` is never a program's value at all — it is `nvs_types::UrlPiece::prepared`'s output
carried as the `ConstStr` `nvs_ir::lower` writes), `format.rs`'s `rendered` (the same
post-condition), `json.rs`'s three (the `u32` conversion behind the `1..=1024` throw three lines
above it, `decodeAs`'s slot-0 `ClassDesc`, which `E0442` refuses before any of it runs, and a
derive field naming a constructor parameter its own class does not have) and `regex.rs`'s group
`0` (no program constructs a `Core\Regex\Match`, and `built_match` — its only builder — appends
group `0` first).

**Untouched:** item 12's classification (`UNCLASSIFIED`, `crates/nvs-stdlib/src/registry.rs`).

**Found, not fixed:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale — the
table is built and both members are folded, which is what the new case exercises. Also carried: a
`bytes` array key ICEs in `nvs-ir` and `catch (Core\Error $e)` panics, both with playbook bullets
under *Writing a test case*.

**Orientation gaps.** `[context] adrs` still owes `0085 §§ 1-4` and `0071 §§ 2, 7`. New: the
playbook is filtered to the paths the *item* names, so an item-11 slice naming only its
`src/<class>.rs` never sees the three declaration-window bullets (`playbook.md:835`, `:2264`,
`:3013`) — whose rule is the one every such slice needs, since `unreachable from source` must sit
whole on **one** line within the eight above the `Fault::` line. Naming
`crates/nvs-stdlib/tests/conformance_coverage.rs` in each slice's own anchors pulls them in.

## Next group

**Item 11's last three slices, then item 12's first class. Shared file set:**
`crates/nvs-stdlib/tests/conformance_coverage.rs:603` — `OWED_A_CASE` is the worklist and a slice
is done when its lines are gone — plus one `crates/nvs-stdlib/src/<class>.rs` per slice. Probe each
site with `nvs check`/`nvs run` on a scratch file before writing the comment; the judgement is
which diagnostic refuses the call first, or that the guard is a post-condition of the call above it.

- [ ] **`validate.rs`'s two and `math.rs`'s one, goal § item 11.** Sites: `validate.rs:276`,
      `validate.rs:281`, `math.rs:1303`. Both `isIp` guards are behind a declared `4|6` literal
      union and `Core\Math::round`'s is behind a `Core\RoundMode` enum parameter, so all three are
      the argument-type shape the playbook's `Core\Test::assertCount` bullet works: three `E0401`
      probes settle them. Anchors: `crates/nvs-stdlib/tests/conformance_coverage.rs:603`.
- [ ] **`debug.rs`'s two and `path.rs`'s one, goal § item 11.** Sites: `debug.rs:147` (the
      writer's own `io::Error`, which is the terminal sink and may be a *boundary* rather than
      unreachable — judge it before declaring it), `debug.rs:191` and `path.rs:612` (the same
      "empty slot the array reported as live" invariant, twice).
- [ ] **`test.rs`'s two, goal § item 11.** Sites: `test.rs:678` (`compareTo` answering a non-`int`,
      a post-condition of a declared return type) and `test.rs:707` (`assertEqualsDeep` past
      `MAX_DEPTH`, which a program *can* reach and so wants a case rather than a declaration).
- [ ] **Item 12's next class**, `crates/nvs-stdlib/src/registry.rs`'s `UNCLASSIFIED` roster, ADR
      0088 § 2. Only once `OWED_A_CASE` is empty — it is the goal's own acceptance ratchet.

## Backlog

- Item 12's per-parameter qualifier classification, the whole remaining roster — ADR 0088 § 2.
- `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 no longer describe the tree.
- `Core\Router::match`/`methodsFor` stay out of scope — `docs/agent/loop-goal.md` § *Standing
  decisions*.
- A `bytes` array key ICEs in `nvs-ir` rather than being diagnosed — `docs/agent/playbook.md`.
- `catch (Core\Error $e)` panics in `nvs-ir` — `docs/agent/playbook.md`.
- The stage-5 `[[check]]` blocks' `cases` lists are in no orientation field, so an acceptance
  failure naming a `.nvst` is still triaged by grepping `docs/agent/loop-goal.toml`.
