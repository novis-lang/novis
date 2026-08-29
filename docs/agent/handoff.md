# Handoff

## State

**Two of the four `tests/differential/lang/` roster cases the driver's differential check names are
written and pass against PHP 8.5.** `every-str-member-with-a-php-twin-agrees.nvst` sweeps every
`Core\Str` member the spec gives a twin over one ASCII corpus of eight subjects and four needles;
`every-arr-member-with-a-php-twin-agrees.nvst` sweeps `Core\Arr` over five single-typed lists, four
word lists and one string-keyed map. Each ends with a counted identity block — ten identities for
`Str`, eight for `Arr` — that PHP's twins hold too, so the count is comparable rather than internal.

**The check is not closed.** `every-math-member-with-a-php-twin-agrees.nvst` and
`a-date-format-string-renders-as-phps-does.nvst` are still unwritten, and the driver will name
whichever is missing first. Both are in the *Next group* below.

**The two constraints that make a roster case honest are stated in each file's header and are the
thing to copy, not the row list.** For `Str` it is *ASCII only*, because ADR 0009 § 2's grapheme unit
is where half the roster is supposed to disagree. For `Arr` it is *single-typed lists, values-rendered
unless the member's contract is its keys*, because spec § 2's divergence column is almost entirely
about key renumbering and PHP's loose comparisons. Six `Core\Arr` members are deliberately absent —
`flattenDeep`, `from`, `groupBy`, `column`, `overlayDeep`, `mapKeys` — each with its reason written
at the head of the file.

**Item 12's roster is untouched this session** and still opens at `Core\Bytes`; the acceptance check
outranked it.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale (the route table
is built now), a `bytes` array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last
two have playbook bullets under *Writing a test case*.

**Orientation gaps.** The four `tests/differential/lang/…` case names live only in
`docs/agent/loop-goal.toml:1345-1351`, and nothing in the pack prints a check's `cases` list, so
learning what the failing check actually wanted cost a peek at the TOML. A `[context]` selector that
prints the *failing* check's own `cases` block would pay for itself every session the driver reports
one. `[context] modules` also names no `nvs-stdlib` class module, so both member rosters were
delegated to a subagent rather than read. Still owed from before: ADR 0088 § 1, ADR 0063 R11,
`0085 §§ 1-4` and `0071 §§ 2, 7`.

## Next group

**Shared file set:** `tests/differential/lang/`, with the two cases landed this session as the shape
to copy, plus one `crates/nvs-stdlib/src/<class>.rs` per slice for the member roster. Slices 1 and 2
close the driver's failing check; slice 3 resumes item 12 where the handoff before this one left it.

- [ ] **`every-math-member-with-a-php-twin-agrees.nvst`** — spec § 3 at
      `docs/spec/01-core-library.md:369`, roster at `crates/nvs-stdlib/src/math.rs:52`. Stay out of
      the two divergences that already have their own files (`math-min-and-max-diverge-…`,
      `math-to-base-and-from-base-refuse-what-php-silently-repairs`), and read the playbook bullet on
      `CoreTy::Var("T")` first: `min`/`max`/`clamp` bind `T` to the first argument, so a mixed pair is
      `E0401` and not the runtime refusal a case would be trying to reach.
- [ ] **`a-date-format-string-renders-as-phps-does.nvst`** — spec § 4 at
      `docs/spec/01-core-library.md:420`; the three `format` rows are `crates/nvs-stdlib/src/time.rs:914`,
      `:1259` and `:1392`. ADR 0057 folds a *literal* template while checking, so a swept table of
      templates has to come from a variable if it is to reach the runtime path.
- [ ] **`Core\Bytes`' twelve rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/bytes.rs:161`, roster at
      `crates/nvs-stdlib/src/registry.rs:1526`, the rule at `registry.rs:74`. Ten follow the two
      ordinary bullets; `pack` at `:246` and `unpack` at `:253` are ADR 0063 R11's fourth grammar,
      which `Qual`'s third bullet makes `Sink`. Land the class as one commit — a member classified
      *and* still on the roster fails the gate.

## Backlog

- `Core\Path`'s nine rows, ADR 0088 § 2 — `crates/nvs-stdlib/src/path.rs:72`.
- `Core\Validate`'s six rows, ADR 0088 § 2 — `crates/nvs-stdlib/src/validate.rs:159`.
- The classification is still declaration-only: `nvs_types::core_lib.rs:286` lowers `CoreTy::Text(_)`
  to a plain interned `string`. Enforcement is a separate item nobody has taken.
- `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale.
- The six `Core\Arr` members the roster case names as absent still have no differential coverage of
  their own — `column` and `overlayDeep` are the two worth a case first.
