# Handoff

## State

**The differential suite is at 200, all passing** (193 → 200), which **closes the Stage 5 depth
acceptance check** that wanted 200 and failed after session 0010. Conformance is unchanged at 951
and no case fails. **No `nvs-stdlib` source changed.**

**`Core\Regex` was the thinnest differential domain** — 2 oracle cases over 8 members, against 47
for `Core\Arr` and 40 for `Core\Str` — while PCRE is the definitive authority for every one of
them. `gaps.py --differential` reports 0 gaps and is right: every member with a **Replaces** entry
had *a* case. The gap it cannot see is depth, and that is where the seven new cases went.

**What the seven pin**, all in `tests/differential/core/`. `matches` as a predicate over 28 rows
paired so that each feature has the nearest subject that fails it, straddling both of ADR 0056's
tiers. `replace` over the replacement grammar the two languages *share* — `$0`, `$n`, `${n}`, a
reference naming no group, the `$10`/`${1}0` boundary and the limit — with `\1` and `$$` left to
the conformance case, because those are decisions rather than agreements. `matchAll` against
`preg_match_all` under `PREG_SET_ORDER | PREG_UNMATCHED_AS_NULL`, counting sets, entries, absent
and empty. `offset` against `PREG_OFFSET_CAPTURE`, with `{from:}` swept over one subject to
separate "where the search began" from "what the offset counts from", ASCII-only because `offset`
is a grapheme index. `replaceWith` against `preg_replace_callback`, including the computed answers
a template cannot spell. `split` against `preg_split` — and, beside it, the divergence case for
the three rows where it parts, which `nvs_core_regex_split`'s own doc comment already decided:
`limit` is `Core\Str::split`'s word, and `keepEmpty` drops empties *after* it.

**One open correctness divergence, found by writing these and not yet fixed** — it has a playbook
bullet and heads the next group. All four of § 5's iterating members drop a zero-width match that
starts where the previous match ended, because they share the `regex` crate's iterator; PCRE
reports it. Three of the seven cases say so in their comments and route around it.

**Untouched:** the `Core\Arr` floor-3 group the last handoff named (`replaceRange`, `withoutFirst`,
`underlay`) — the acceptance failure outranked it and it is below, unchanged. Item 12's roster
(10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`) and `catch (Core\Error $e)`
panicking both still have playbook bullets.

**Orientation.** The pack was complete for the item it named, but the item was not what the session
owed: the acceptance check that outranked it is a `tests/differential/` count, and nothing in
`[context]` prints the differential suite's shape. `docs/spec/01-core-library.md` § 5 and the
`[context] cases` selector for the failing check's own `cases` block are both still owed, as are
`hash.rs`, `test.rs`, `nvs-types/src/links.rs`, `routes.rs`, `validate.rs`,
`docs/spec/02-php-migration.md` and `tools/check-migration.py`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/regex.rs` and `tests/differential/core/`. The first
slice is a source fix and the two after it are the cases that could not be written without it.

- [ ] **A zero-width match starting where the previous match ended is reported, as PCRE reports
      it** (`nvs_core_regex_replace_with` at `crates/nvs-stdlib/src/regex.rs:1163`,
      `nvs_core_regex_split`'s `pieces_of` at `crates/nvs-stdlib/src/regex.rs:1208`, and the
      `captures_iter` loops `nvs_core_regex_replace`/`nvs_core_regex_match_all` share) — spec § 5.
      One iteration rule, four call sites: after a non-empty match ending at `e`, search again
      *from* `e` and accept an empty match there, then advance a cluster. Re-freeze whatever
      conformance expectations move; `regex-replace-with-sees-exactly-the-matches-match-all-reports`
      is the case that pins the members against each other and must stay green.
- [ ] **The zero-width rows come back**, in the three cases whose comments name the divergence:
      `regex-match-all-reports-the-sets-preg_match_all-reports-in-set-order.nvst` (restore
      `["baaac", "a*"]`), `regex-replace-matches-preg_replace-over-the-grammar-the-two-share.nvst`
      (add `["ab", "b*", "-"]`), and
      `regex-replace-with-answers-what-preg_replace_callback-answers.nvst`.
- [ ] **The four compile flags are the PCRE modifiers they are named for**
      (`COMPILE_OPTIONS` at `crates/nvs-stdlib/src/regex.rs:252`) — spec § 5. `caseInsensitive`,
      `multiline`, `dotAll` and `ungreedy` against `/…/i`, `/…/m`, `/…/s` and `/…/U`, over one
      table of subjects, with the pairs as well as the singles. The one member of § 5 with no
      oracle case at all; a shape literal takes only literals here, so the flag combinations are
      written out rather than swept from a variable.

## Backlog

- `Core\Arr`'s floor-3 members — `replaceRange`, `withoutFirst`, `underlay`, all named with their
  anchors in `git show 892cf33:docs/agent/handoff.md`. `gaps.py --coverage` re-derives them.
- `Core\Math` is beside `Core\Arr` at floor 3: `atan2`, `hypot`, `lcm`.
- `Core\Uri` has one differential case over 20 members, and `Core\Json` three — the same depth gap
  `Core\Regex` had, in the two next-thinnest domains.
- Item 12's `UNCLASSIFIED` roster, `crates/nvs-stdlib/src/registry.rs:1526`.
- `catch (Core\Error $e)` is an ICE rather than a diagnostic; playbook owns the spelling.
- `[context]` owes a selector for the failing check's own `cases` block, so a session that must
  close an acceptance check can read its worklist without opening `loop-goal.toml`.
