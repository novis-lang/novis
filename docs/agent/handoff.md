# Handoff

## State

**Stage 5's item 10 has a gate, and it is a ratchet.**
`every_core_class_has_a_conformance_floor_of_three` in `crates/nvs-stdlib/tests/conformance_coverage.rs`
puts a floor of three cases under every registered member. It landed over an existing violation set of
26, so the members below the floor are named in `BELOW_THE_FLOOR` (`:247`) and the test fails both on a
member that is *not* listed and below the floor **and** on a listed member that has reached it — the
list cannot go stale in either direction, and nothing can be added to it. **Twenty of the twenty-six
closed this session; six remain**, and they are the next group.

**The driver's failing acceptance check is closed.** `nvs-stdlib (the coverage gates)` reported
`every_core_class_has_a_conformance_floor_of_three did not run`; the test now exists and passes.

**The counting rule is `gaps.py`'s, ported.** `Attribution` in that file is the same attribution
`tools/gaps.py`'s `coverage` performs — a case holds a class it names, or one that something it called
answers an instance of, to a fixed point — so the gate and the worklist cannot rank a member
differently. That tool's docstring stays the only home for *why*; see the playbook bullet for the one
place the two genuinely part, which is that `gaps.py` misses whole classes.

**The stage-4 `#[Api]` group is untouched.** The previous handoff's group — § 2's `tags`/`security`,
`errors`/`example`, and § 1's object response schema — is still open and unstarted, in the backlog
below with its anchors intact.

**Orientation gap, tenth session running:** `[context] adrs` still does not carry `0085 §§ 1-4`. New
this session: nothing in the pack describes Stage 5's item 10, so the goal TOML's stage-5 `[[check]]`
block had to be read by hand to learn what the named test was for — `[context]` has no field that
prints a stage's own comment header, and the playbook already warns that block *is* the specification.

## Next group

**The six members still under the floor, § item 10 — delete each line from `BELOW_THE_FLOOR` as its
case lands, or the gate fails on the stale entry. Shared file set:** `tests/conformance/core/`,
`crates/nvs-stdlib/tests/conformance_coverage.rs:247`.

- [ ] **`Core\Regex::quote` and `Core\Math::atan2`, one case each** — the two that need no program
      structure at all. `quote` is **launder** for the pattern sink (spec § 5), so its second question
      is that a quoted metacharacter string matches itself literally and nothing else;
      `crates/nvs-stdlib/src/math.rs:222` is `atan2`, whose second question is the quadrant its two
      signs pick out, asserted by comparison against `Core\Math::PI` rather than by printing a float.
- [ ] **`Core\Attributes::all`, one case** — the row is `crates/nvs-stdlib/src/attributes.rs:53`, and
      it is folded at compile time (`address()` maps both members to `folded_at_compile_time`). The
      existing case is `tests/conformance/core/an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst`;
      the unasked question is the empty retrieval — `all<T>` where no attach site satisfies `T` answers
      the empty array rather than refusing, which is what makes it the plural of `get`'s `null`.
- [ ] **`Core\Program::implementing`, two cases** — it is at 1 and needs two. One of them is already
      on the goal's own worklist:
      `tests/conformance/core/program-implementing-enumerates-every-implementor.nvst` is listed in
      `docs/agent/loop-goal.toml`'s stage-5 `cases` block and does not exist. ADR 0061 § 3 is in the
      orientation pack; the existing case is `…-expands-to-new-expressions-at-the-call-site.nvst`.
- [ ] **`Core\Router::urlAbsolute` (two) and `Core\Time\TimeOfDay::compareTo` (one)** — `urlAbsolute`
      is `crates/nvs-stdlib/src/router.rs:146` and is the hard one: a program declaring no `#[Route]`
      has an empty table, so today's only reachable claim is the refusal
      (`router-url-and-url-absolute-refuse-the-same-unknown-names.nvst` already asks it once) and a
      third case of that shape is the "another row of the same shape" conventions.md forbids. Its two
      cases probably want the route-table case the goal also owes, or the unasserted throw at
      `router.rs:351`. `compareTo` is `crates/nvs-stdlib/src/time.rs:1420` and is ordinary: the bound
      asserted on both sides, plus agreement with the other three `compareTo` members.

## Backlog

- Stage 4 § 2: `tags`/`security` reach the document — `crates/nvs-types/src/routes.rs:659`
  (`API_OPTIONS`), `:750` (`check_api`), `:321` (`Route`), `crates/nvs-cli/src/openapi.rs:147`
  (`operation`). Gap 3 in that emitter's module doc.
- Stage 4 § 2: `errors` and `example`, the two structured `#[Api]` fields. Same file set.
- Stage 4 § 1: an object response schema — `DerivedCodec` at `crates/nvs-types/src/derive.rs:178`,
  gap 1 in `crates/nvs-cli/src/openapi.rs`'s module doc.
- `[context] adrs` in `docs/agent/loop-goal.toml` still owes `0085 §§ 1-4`; ten sessions have sliced it
  by hand.
- `python tools/gaps.py` drops whole classes from its ranking (playbook, *Tooling*). Fixing it to read
  `registry::CLASSES` would make the worklist agree with the gate.
- 72 unasserted `Fault` sites remain (`python tools/gaps.py --errors`); `router.rs:351` and
  `csv.rs:512` are the two `thrown` ones a case can catch.
