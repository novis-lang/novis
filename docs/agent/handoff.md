# Handoff

## State

**All four `tests/differential/lang/` roster cases the driver's differential check names are written
and pass against PHP 8.5.** This session added the last two.
`every-math-member-with-a-php-twin-agrees.nvst` sweeps every `Core\Math` member and every constant the
spec gives a twin over seven domain-scoped corpora, and ends with two counted identity blocks — 120 of
120 over the reals, 28 of 28 over the integer pairs — that PHP's twins hold too.
`a-date-format-string-renders-as-phps-does.nvst` asks every CLDR field letter `cldr.rs`'s subset table
names of all three `format` members, each against the `date()` letter spec § 4 says it replaces, and
ends with 48 of 48 composition identities.

**The driver's failing acceptance check should now close.** Nothing else in the goal was regressing.

**The constraint that makes each roster case honest is in its own header, and is the thing to copy.**
For `Math` it is *every argument inside its member's domain, every sweep over one type* — the domain
because the IEEE edges have their own files, the single type because `min`/`max`/`clamp` bind one `T`
to the first argument. For the date grammar it is *the pattern table is a variable*, because ADR 0057
folds a literal pattern while checking and only a swept table reaches the runtime parser
`cldr.rs`'s *Known gaps* 1 describes. Seven twins across the two files are hand-written or computed
rather than named, each with its reason at the call site.

**Item 12's roster is untouched** and still opens at `Core\Bytes`; the acceptance check outranked it
twice now. The roster itself is not a doc — it is
`crates/nvs-stdlib/src/registry.rs:1526`'s `UNCLASSIFIED` list, a ratchet that only shrinks, and it
holds 62 members across ten classes: `Bytes` 12, `Path` 9, `Time` 6, `Uri` 9, `Test` 8, `Validate` 6,
`Hash` 4, `Uuid` 2, `Router` 2, `Csv` 2.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale (the route table
is built now), a `bytes` array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last two
have playbook bullets under *Writing a test case*.

**Orientation gaps.** Unchanged from last session and still owed: a `[context]` selector that prints
the *failing* check's own `cases` block (the four case names live only in
`docs/agent/loop-goal.toml:1345-1351`), and `[context] modules` naming an `nvs-stdlib` class module.
Newly found: nothing in the pack points at `registry.rs`'s `UNCLASSIFIED` list, which is item 12's
actual worklist, so finding out what the roster still owes cost four probes; a `[context] anchors`
entry for it would pay for itself every session that takes an item-12 slice. Still owed from before:
ADR 0088 § 1, ADR 0063 R11, `0085 §§ 1-4` and `0071 §§ 2, 7`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/registry.rs` — every slice deletes its class's lines from
`UNCLASSIFIED` at `registry.rs:1526` and reads the rule at `registry.rs:74` — plus one class module
each. Item 12 in roster order, largest class first.

- [ ] **`Core\Bytes`' twelve rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/bytes.rs:161`. Ten follow
      the two ordinary bullets; `pack` at `bytes.rs:246` and `unpack` at `bytes.rs:253` are ADR 0063
      R11's fourth grammar, which `Qual`'s third bullet makes `Sink`. Land the class as one commit — a
      member classified *and* still named in `UNCLASSIFIED` fails the gate just as an unclassified one
      missing from it does.
- [ ] **`Core\Path`'s nine rows** — `crates/nvs-stdlib/src/path.rs:72`. Every parameter is a filesystem
      path, so the question each row asks is whether a path is an instruction to the OS (ADR 0088 § 1)
      or data; `join` and `relativeTo` take two, and `withExtension`'s second argument is the one that
      is not a path at all.
- [ ] **`Core\Time`'s six rows** — `Time::fromIso` at `crates/nvs-stdlib/src/time.rs:1130`, `Time::parse`
      at `:1137`, and the three `format` members at `:914`, `:1259` and `:1392`, plus
      `Duration::parse` at `time.rs:190`. Spec § 4 already states the answer for four of them: *a CLDR
      pattern is a `tainted` sink* wherever one is taken, and the `$text` being parsed is data and
      stays contagious.

## Backlog

- `Core\Uri`'s nine rows, then `Test` 8, `Validate` 6, `Hash` 4, `Uuid` 2, `Router` 2, `Csv` 2 — the
  rest of `registry.rs:1526`'s `UNCLASSIFIED`.
- `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale — the route table is built.
- A `bytes` array key ICEs in `nvs-ir`; `catch (Core\Error $e)` panics. Both have playbook bullets.
- `[context]` in `docs/agent/loop-goal.toml` owes three selectors: the failing check's `cases` block,
  an `nvs-stdlib` class module pattern, and an anchor for `registry.rs`'s `UNCLASSIFIED`.
- `Core\Math::RoundMode::Up`/`Down` and `Core\Math::UINT_MAX` have no PHP twin and so no differential
  row; if either grows one, the roster case's header names the absence to delete.
