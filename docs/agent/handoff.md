# Handoff

## State

**M4's Stage 8, depth.** The tree is at **866 conformance plus 189 differential**, all
green. Nothing is blocked.

Two cases landed over one file set — `crates/nvs-stdlib/src/bytes.rs` and `crates/nvs-stdlib/src/str.rs`
read, `tests/conformance/core/` written — both about the three `Core\Bytes` predicates, and both
asking a question no existing case asks:

- **`Core\Bytes` is not `Core\Str`, on a subject where they can be told apart.** Every other case
  about the six same-named predicates uses an ASCII subject, where the two classes are
  indistinguishable. Over `"aé漢b"` (4 characters, 7 octets) the difference is in the *domain*, not
  the answers: all 15 character-aligned needles get the same answer from both trios (45/45), each
  character's `Core\Str::indexOf` is its character index while `Core\Bytes::indexOf` is the octet
  length of the prefix before it (2 of 4 differ), and of the 28 non-empty octet windows exactly 10
  are namable as a `string` — those 10 being exactly the character sweep's own — while `contains`
  finds all 28. The other 18 split a character and `Core\Str::contains` cannot be called with any of
  them.
- **`contains` is the two ends, taken over every window.** The existing corpus derives all three
  predicates from `indexOf`, `slice` and `compare` and none of them from each other, so a `contains`
  with its own search and a `startsWith` with its own comparison pass everything already written.
  Here `contains` is asserted to be "some suffix begins with it" and "some prefix ends with it" over
  156 needles (78 present, 78 near-missing), and the *number* of positions each disjunction holds at
  is the needle's occurrence count from each side — 12/12 for the empty needle, 5/5 for `"a"`, 2/2
  for `"abra"`, 1/1 for the whole subject.

**The session's real finding, and why the next group is not another depth item:** `gaps.py`'s
per-member count has stopped tracking coverage at the top of its table. The group's first item was
already landed verbatim, and spot-checking the thinnest members of `Core\Test`, `Core\Random`,
`Core\Time` and `Core\Regex` disqualified all four the same way. `python tools/loop.py --list` also
reports **0 named cases not written yet** in both suites. The playbook bullet added this session owns
the check that makes this cheap.

## Next group

**Reconcile the guard-name debt**, which `docs/implementation-plan.md`'s *Open now* names as an open
item and which blocks the acceptance gate at its first stale name. One shared file set:
`docs/agent/guard-name-debt.md` and `docs/agent/loop-goal.toml`, with
`cargo test -p <crate> -- --list` as the authority. Cause 1 (the work landed under a different name)
is a pure rename in the toml and is the commonest.

- [ ] **The five confirmed cause-1 renames**, listed with their real names at
      `docs/agent/guard-name-debt.md:26-41` — `an_instanceof_narrows_its_operand`,
      `a_literal_comparison_narrows_its_operand`,
      `a_fixture_attribute_is_resolved_for_the_cases_that_name_it`,
      `a_test_with_attribute_expands_to_one_case_per_row`,
      `an_abandoned_generator_resumes_to_unwind`. Rewrite each in `loop-goal.toml`'s `tests = [...]`
      and tick it at its `- [ ]` line in the debt file.
- [ ] **Stage 3's remaining 7**, at `docs/agent/guard-name-debt.md:57-69` — `nvs-ir (control flow)`
      6 of 7 and `nvs-types (calls and loops)` 2 of 6. Run
      `cargo test -p nvs-ir -- --list` and `cargo test -p nvs-types -- --list` once each and sort
      every name into cause 1 (rename), 2 (the check belongs to a `.nvst` suite) or 3 (genuinely
      unwritten); rename the first group, and record the verdict for the rest in the debt file.
- [ ] **The rest of the 54, by stage**, from `docs/agent/guard-name-debt.md:53` onward, same three-way
      sort. Re-derive the count with the loop at `guard-name-debt.md:14-17` before trusting the
      header — that file says of itself that it is a snapshot, not a source.

## Backlog

- `every_refusal_is_a_diagnostic_or_decided` is cause 3 and load-bearing: `python tools/holes.py`
  still reads 17 standing refusal sites — `docs/agent/guard-name-debt.md:46`.
- `nvs-stdlib (the assertion roster)` names a Rust test tree that does not exist; the roster was
  pinned in conformance cases — decide which tree owns it, `docs/agent/guard-name-debt.md:42`.
- `Core\Bytes` depth is spent: `contains`/`startsWith`/`endsWith` now carry the octet/character
  boundary and the mutual-derivation identity as well as the `indexOf`/`slice`/`compare` ones.
- `crates/nvs-stdlib/src/csv.rs:512`'s `thrown` is unreachable from source and owes no case — the
  playbook bullet under *Writing a test case* owns why.
- `orient.py`'s `[context] modules` still has no `nvs-stdlib/src/*.rs` pattern, so no `Core` class's
  module line is ever printed; three handoffs have now reported it.
