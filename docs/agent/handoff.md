# Handoff

## State

**ADR 0088 § 2's classification has a home in the registry, and its gate is a ratchet.**
`CoreTy::Text(Qual)` / `CoreTy::Blob(Qual)` are the classified spellings of a `string`/`bytes`
**parameter**; bare `CoreTy::Str`/`Bytes` in parameter position now mean *unclassified*, which ADR
0088 § 2 makes a refusal rather than a default. `Qual` (`registry.rs`, above `CoreTy`) is the four
marks the spec's Q column renders. `every_member_parameter_carries_a_qualifier_classification`
(`crates/nvs-stdlib/src/registry.rs:1652`) fails on a member with an unclassified string parameter
that is **not** in `UNCLASSIFIED` (`:1509`) and on a listed member that **has** been classified — the
list cannot go stale in either direction and nothing can be added to it. It landed at **118**;
`Core\Regex` is the one class already classified and so is absent from it.

**Only the data landed, not the enforcement.** `nvs_types::core_lib::lower` maps `Text(_)` to the
same interned `string` as `Str`, so no observable behaviour changed and the playbook's bullet about
no `Core` member accepting a `tainted` argument still holds. Wiring `Qual` into
`nvs_types::expr::args` is a separate slice, in the backlog.

**The driver's acceptance check is still red, and it will name item 11 next.** `nvs-stdlib (the
coverage gates)` lists five tests; four now exist, and
`every_error_path_is_asserted_or_declared_unreachable` does not. That is the first slice below.

**One acceptance check was flaky and no longer is.** `nvs-cli (determinism and the diff)`'s four
tests shared one temp path per fixture stem across threads; `crates/nvs-cli/tests/openapi.rs`'s
`document` now takes a fresh file per call. See the playbook bullet — the flake's triage points at
whatever you happen to be holding.

**The stage-5 floor group is untouched.** The six members still under the floor are unchanged in
`BELOW_THE_FLOOR` (`crates/nvs-stdlib/tests/conformance_coverage.rs:255`), still the group the
previous handoff named, and still open.

**Orientation gap, eleventh session running:** `[context] adrs` still does not carry `0085 §§ 1-4`.
New this session: nothing in the pack said which of the coverage gates' five named tests already
exist, so the failing check had to be triaged by `grep`ping for four `fn` names — the goal TOML's
stage-5 `[[check]]` block is the only place that list lives and `[context]` has no field printing it.

## Next group

**Item 11's gate, then item 12's list shrinking. Shared file set:** `crates/nvs-stdlib/src/registry.rs:1509`
(the ratchet), `crates/nvs-stdlib/tests/allocation_policy.rs:92` (the source-scan shape), and one
`crates/nvs-stdlib/src/<class>.rs` per classification slice.

- [ ] **`every_error_path_is_asserted_or_declared_unreachable`, goal § item 11.** The last unwritten
      test in the failing check, so it outranks everything else here. **Read `python tools/gaps.py
      --errors` first**: there are ~300 `Fault::` sites in `crates/nvs-stdlib/src`, of which that tool
      calls 68 unasserted, and only it can tell the two apart — a Rust test cannot, so the gate is a
      source scan over *declaration comments* plus a frozen list, not a coverage computation.
      `crates/nvs-stdlib/tests/allocation_policy.rs:92`'s `no_member_revalidates_a_string_argument`
      is the source-scan shape; `registry.rs:1652` is the two-way ratchet shape. Judge before
      writing: item 11 says a `Fault::fatal` may be an invariant no program reaches, and then the
      answer is a comment at the site.
- [ ] **`Core\Validate` (6) and `Core\Uuid` (2), all `Qual::Neutral`.** Every `Validate::is*` returns
      `bool` and `Uuid::parse`/`tryParse` answer an opaque instance, so the result carries nothing
      from the argument. Rows at `crates/nvs-stdlib/src/validate.rs:162` and `uuid.rs:134`; delete
      their eight lines from `registry.rs:1509` in the same commit or the gate fails on the stale
      entries.
- [ ] **`Core\Str`'s 37 rows, the largest single block of the 118.** The spec's Q column
      (`docs/spec/01-core-library.md` § 1) already answers them: nine *Inspection* members and
      `codePoints` are `neutral`, `format`'s template is `sink` (ADR 0063 R11 — one of the four
      grammars), and every remaining cell is blank, which that file's § *How to read an entry* says
      means `contagious` was chosen. `crates/nvs-stdlib/src/str.rs:125` is the first row.
- [ ] **The six members still under the conformance floor**, unchanged from the previous handoff:
      `Core\Regex::quote` + `Core\Math::atan2` (`crates/nvs-stdlib/src/math.rs:222`), then
      `Core\Attributes::all` (`attributes.rs:53`), `Core\Program::implementing`,
      `Core\Router::urlAbsolute` and `Core\Time\TimeOfDay::compareTo`. Delete each line from
      `BELOW_THE_FLOOR` (`tests/conformance_coverage.rs:255`) as its case lands.

## Backlog

- Wire `Qual` into the checker so a `Contagious` parameter accepts a `tainted` argument and a
  `Launder` one strips it — ADR 0088 §§ 3-5, `nvs_types::expr::args`. Nothing in item 12 asks for it.
- The 16 `string`-typed `CoreOption`s (`csv.rs:171`, `math.rs:462`, `str.rs:438`, `test.rs:124`,
  `uri.rs:464`) are parameters too and are counted by the gate through their member's row.
- Stage 4's `#[Api]` group — § 2's `tags`/`security`, `errors`/`example`, § 1's object response
  schema — still open and unstarted; `docs/agent/goals/1-core-depth.md` items 6-9 own it.
- `[context] adrs` wants `0085 §§ 1-4`, and `[context]` wants a field that prints a stage's
  `[[check]]` comment header — the playbook already warns that block *is* the specification.
