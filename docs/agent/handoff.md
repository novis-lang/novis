# Handoff

## State

**The conformance corpus is at 909 and the driver's floor is 950**, so the failing acceptance check is
41 cases short and is not a regression — it is item 10's corpus floor, closing by depth cases at about
two a session. Item 12's roster is untouched: 10 members across five classes remain `UNCLASSIFIED` at
`crates/nvs-stdlib/src/registry.rs:1526` — `Core\Uuid` (2), `Core\Hash` (3), `Core\Hash\Stream` (1),
`Core\Router` (2), `Core\Csv` (2). Nothing consumes `Qual` yet, so a classification slice still cannot
change what a program does today.

**`Core\Router::url`'s path encoding is now pinned**, which was the group's item 1 and the one rule
§ 6 had no case for. The query half was already agreed against `Core\Uri::buildQuery`; the path half is
`Form::Component` and parts from it on exactly two bytes — `~` is unreserved in a path and escaped in a
query, a space is `%20` and `+` — and the new case sits them on adjacent lines. It also counts: over
eight hostile values the link keeps the declared path's three segments, which is the whole content of
§ 4's "launderer for the URL-path sink".

**`crates/nvs-stdlib/src/router.rs`'s *Known gaps* 1 and 2 are no longer stale.** They said the route
table was not built and that neither member's laundering was real; both had been false for some time —
a literal name folds, an unknown one is `E0754` before the program runs, a leftover key naming nothing
is `E0759`, and `substitute` percent-encodes every segment. What is actually left is ADR 0097 § 3's
mount prefix, which has nowhere to come from off the command line. `substitute`'s own doc paragraph
claiming the `E0759` refusal "cannot be" written was repaired in the same pass.

**A typed capture's closed set is not enforced when a link is *built*.** ADR 0102 § 5's own example,
compiled and run, answers `url("docs.show", ["lang" => "xx"]) == "/xx/docs/intro"` — a link to a path
that § 5 says falls through to `404` at match. Group item 3 was written as a test slice ("the last
accepted and the first refused"); there is nothing to assert yet, so it is a **feature** slice and the
next group opens on it.

**Untouched:** a `bytes` array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — both have
playbook bullets under *Writing a test case*.

**Orientation gaps.** Newly owed: `crates/nvs-types/src/links.rs` and `src/routes.rs` in
`[context] modules` (the link fold lives there, not in `nvs-stdlib`, and every router item reaches it),
and ADR `0102 § 5` in `[context] adrs`. Still owed from before, none added yet:
`docs/spec/02-php-migration.md` and `tools/check-migration.py` in `[context] docs`, the stage-7 comment
header's per-goal floor table, a selector that prints the *failing* check's own `cases` block, a
`[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`, ADR 0088 § 1, 0063 R11, `0085 §§ 1-4`,
`0071 §§ 2, 7`, and conventions.md's *four shapes a depth case takes* whenever the group is conformance
depth rather than a member.

## Next group

**Shared file set:** `crates/nvs-types/src/links.rs`, `crates/nvs-diagnostics/src/lib.rs` and
`tests/conformance/core/router-url-*.nvst`. The first two slices are one feature and its case; do them
in order, and read `links.rs` whole (326 lines) rather than around an anchor — `resolve` hands
`declared` and `covered` the same site and all three move together.

- [ ] **A link refuses a literal capture value outside the capture's closed set** (`links.rs:166`
      `resolve`, `links.rs:227` `declared`, `links.rs:281` `covered`) — the handler's parameter type is
      already in reach at the site where `E0759` is raised (`crates/nvs-diagnostics/src/lib.rs:2034`,
      next free in the band is **E0772**), and a `"en"|"de"|"fr"` union or an enum subset is the closed
      set to check a folded literal against. ADR 0102 § 5, ADR 0077 § 3. Only the *literal* value:
      `$params` with computed keys has nothing to check, exactly as `E0759` already accepts.
- [ ] **The case for it, both sides of the bound** (`tests/conformance/core/`) — the last accepted
      member of the set links, the first outside it does not compile, in one `--EXPECTF-ERROR--` pair
      with `router-url-encodes-a-capture-into-its-own-segment.nvst`'s route shape. *A bound asserted on
      both sides*.
- [ ] **Say what a *computed* out-of-set value does** (`router.rs:272` `substitute`) — decide throw or
      accept under AGENTS.md's ordering, record it in that function's doc comment beside the paragraph
      this session repaired, and add the `.nvst` line if it throws.

## Backlog

- **`urlAbsolute` with an origin actually configured** (group item 2) — blocked on shape, not design: a
  `.nvst` has no known way to give a unit an `[app] origin`, so only the refusal side is testable and
  it already is (`router-url-absolute-refuses-a-unit-with-no-configured-origin.nvst`). Owner: ADR 0102
  § 6.
- Item 12's 10 `UNCLASSIFIED` members — `crates/nvs-stdlib/src/registry.rs:1526`, ADR 0088 § 2.
- `router.rs`'s *Known gaps* 3 and 4 (`::match`, `methodsFor`, the verb parse) — out of scope by
  `docs/agent/loop-goal.md` § *Standing decisions*.
- A `bytes` array key ICEs in `nvs-ir`; `catch (Core\Error $e)` panics — `docs/agent/playbook.md`,
  *Writing a test case*.
- The corpus floor is 41 cases away at ~2 a session — `python tools/gaps.py` ranks the next class.
