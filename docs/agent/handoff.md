# Handoff

## State

**The OpenAPI emitter answers ADR 0085 § 1's *Enumerations* row.** A parameter whose declared type is
a closed set emits its members as `enum` — `nvs_types::RouteParam::allowed` handed to `schema` beside the
rendered type by `parameter` (`crates/nvs-cli/src/openapi.rs:199`), merged into whatever the type mapping
produced rather than replacing it. That closes ADR 0102 § 5's "the generated document emits
`enum: [en, de, fr]` with no further work", for a path capture and a `#[Query]` key alike.

**The set's members are emitted as JSON strings, including an `int` literal's**, and that decision's home
is `schema`'s doc paragraph: the row carries the set as the *segment text* each value is written with, so
`1|2|3` emits `["1", "2", "3"]` and no `"type"` is emitted beside a set — deciding a JSON type the row
does not carry is the one thing this module does not do.

**`Core\Uuid` emits `{"type": "string", "format": "uuid"}`.** The spelling is matched as text like every
other arm, because `nvs-cli` depends on `nvs-types` alone and cannot reach `nvs_stdlib::uuid::NAME`.

**The module's *Known gaps* 5 is now the enum-case subset alone** — `closed_set` returns no set for one on
purpose, and the emitter needs no further change when it does.

**The conformance corpus is at 912 against the driver's floor of 950**, unchanged this session: the
OpenAPI document is a build artifact, so none of this group could be a `.nvst` case
(`crates/nvs-cli/tests/openapi.rs`'s module doc says why). That check is still item 10, not a regression.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`), a
`bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have playbook
bullets under *Writing a test case*.

**Orientation gaps.** Still owed: `crates/nvs-cli/src/openapi.rs` in `[context] modules` (the map printed
only `nvs-cli/src/main.rs`), ADR `0085 § 1` and `0102 § 5` in `[context] adrs` — both were sliced by hand
this session and both are what this group is specified by. Also still owed, none added yet:
`crates/nvs-types/src/links.rs` and `src/routes.rs` in `[context] modules`,
`docs/spec/02-php-migration.md` and `tools/check-migration.py` in `[context] docs`, the stage-7 comment
header's per-goal floor table, a selector that prints the *failing* check's own `cases` block, a
`[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`, ADR 0088 § 1, 0063 R11, `0071 §§ 2, 7`, and
conventions.md's *four shapes a depth case takes* whenever the group is conformance depth.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/validate.rs` and `tests/conformance/core/`. `python
tools/gaps.py` ranks `Core\Validate` second-thinnest at depth 4.5 with **floor 4**, and its floor is the
number item 10 counts — three cases over its three thinnest members raise it, where three cases anywhere
else raise only the corpus total. Read `gaps.py --errors` for the boundaries first; the four shapes a
depth case takes are conventions.md's.

- [ ] **`Core\Validate::isMac`'s three spellings answer as one predicate** (`validate.rs:459`) — the
      *invariance over a sweep* shape: one address written every accepted way, asserted by **counting**
      the agreements rather than reading them off a line.
      `tests/conformance/core/validate-mac-takes-three-spellings-and-refuses-the-nearest-miss.nvst` is
      what exists; the new case asserts the sweep that one asserts by example.
- [ ] **`Core\Validate::isPrintable`'s range is named on both sides** (`validate.rs:488`) — the last
      accepted code point and the first refused one at each end, together.
      `validate-ascii-and-printable-name-their-own-bounds.nvst` is the neighbour to read before writing.
- [ ] **`Core\Validate::isIp`'s family argument at its edges** (`validate.rs:443`, arity 2) — what the
      second argument does to a subject the other family accepts, which
      `validate-ip-families-and-the-two-byte-classes.nvst` asserts one direction of.

## Backlog

- `nvs api diff` does not classify an `enum` change, though ADR 0085 § 4 names a removed case breaking —
  `crates/nvs-cli/src/api_diff.rs`, now that a document carries sets at all.
- An enum-case-subset capture still gets the empty schema — `openapi.rs:49` gap 5 owns why, and it waits
  on `Core\Router::match`'s segment spelling, which is out of scope for this goal.
- `E0451` (`nvs-diagnostics/src/lib.rs:729`) refuses a parameter default at a literal or union declared
  type, so ADR 0102 § 3's *optional* `#[Query]` cannot be written at a closed set at all.
- `Core\Test` is the thinnest class at depth 4.0, floor 3 (`assertThrows`) — `python tools/gaps.py`.
- Item 12's roster: 10 `UNCLASSIFIED` members, `crates/nvs-stdlib/src/registry.rs:1526`.
- A response body that is an object still has no schema — `openapi.rs` gap 1, waiting on the row carrying
  ADR 0071's codec roster.
