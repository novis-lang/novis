---
looper: 1
title: "every Str member matches and cuts on whole graphemes, and a prefix or suffix cut is one call"
summary: "Reached when startsWith, endsWith, replaceAll and the three trims match only whole graphemes like every other Core\\Str search, Str::afterPrefix and Str::beforeSuffix return the rest or null, the compiler warns W1036 on a prefix or suffix cut written by hand with a fix where the shape allows one, and the rule, the tests, the proofs and the change entries say so."
context: [docs/rules/types.md, crates/nvs-stdlib/src/str.rs, crates/nvs-stdlib/src/granularity.rs]
stages:
  - id: 1
    title: "startsWith, endsWith, replaceAll and the trims match only whole graphemes"
    status: done
    context: [crates/nvs-stdlib/src/str.rs, tests/conformance/core/str-after-and-before-cut-at-the-position-index-of-reports.nvst]
  - id: 2
    title: "Str::afterPrefix and Str::beforeSuffix"
    status: done
    context: [crates/nvs-stdlib/src/str.rs, docs/examples/core/Str/after/about.md, benches/members/core/Str/after.nvs, tests/hostile/core/Str/after/01-long-searches-and-repeated-separators.nvs]
  - id: 3
    title: "W1036 names a prefix or suffix cut written by hand"
    status: open
    context: [crates/nvs-ir/src/rereads.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/ir.rs, crates/nvs-diagnostics/src/code/warnings.rs, crates/nvs-cli/src/main.rs, crates/nvs-lsp/src/document.rs]
  - id: 4
    title: "the rules, the examples, the proofs and the website"
    status: open
    context: [docs/agent/prose.md, docs/rules/types.md, website/src/data/changes.json]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: boundaries
    stage: 1
    name: "startsWith, endsWith, replaceAll and the three trims never match or cut inside a grapheme"
    run: [cargo, test, --lib, a_match_at_the_edge_never_splits_a_grapheme]
    timeout: 1800
    tests:
      - a_match_at_the_edge_never_splits_a_grapheme
  - id: boundaries-conformance
    stage: 1
    name: "a program sees startsWith agree with after, and a trim keep a letter with its mark"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/str-every-match-and-cut-keeps-whole-graphemes.nvst]
    timeout: 1800
  - id: members
    stage: 2
    name: "afterPrefix and beforeSuffix return the rest at a whole-grapheme match and null otherwise"
    run: [cargo, test, --lib, after_prefix_and_before_suffix_cut_one_end_or_return_null]
    timeout: 1800
    tests:
      - after_prefix_and_before_suffix_cut_one_end_or_return_null
  - id: members-conformance
    stage: 2
    name: "a program cuts a prefix and a suffix in one call each"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/str-after-prefix-and-before-suffix-cut-one-end-or-return-null.nvst]
    timeout: 1800
  - id: warning
    stage: 3
    name: "W1036 names each hand-written cut, carries the fix where the shape allows one, and is silent where it is unsure"
    run: [cargo, test, --lib, hand_cuts]
    timeout: 1800
  - id: proofs
    stage: 4
    name: "every feature this goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, whole-grapheme-cuts]
    timeout: 1800
  - id: close
    stage: 5
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, whole-grapheme-cuts]
    timeout: 900
---

# every Str member matches and cuts on whole graphemes, and a prefix or suffix cut is one call

## Why

The user asked on 2026-11-23 for one call that checks a prefix and cuts it off. Their example is the
PHP shape `if (str_starts_with($key, "openingHours-")) { $id = substr($key, strlen("openingHours-")); }`:
the prefix is written twice or needs a variable of its own. The tree has the same shape twice
(`apps/registry/src/App.nvs:1039`, `apps/registry/src/ActivityTest.nvs:154`).

Looking into it showed the Novis form of that shape is also wrong today. `rule:types/string-is-utf8`
makes the grapheme the unit of every `string` position. `length`, `slice` and the whole search family
(`contains`, `indexOf`, `lastIndexOf`, `countOf`, `before`, `after`, `split`, `replace`) keep to it
through `whole_clusters` (`crates/nvs-stdlib/src/str.rs:3198`). Six members do not:

| Member | Today | Where |
|---|---|---|
| `startsWith` | `str::starts_with`, bytes | `str.rs:2115` |
| `endsWith` | `str::ends_with`, bytes | `str.rs:2125` |
| `replaceAll` | `starts_with` per position, and `match_at` when case-insensitive; no boundary test | `str.rs:3451-3475` |
| `trim`, `trimStart`, `trimEnd` | a set of code points, trimmed one code point at a time | `trimmed`, `str.rs:2620` |

Run on the release binary on 2026-11-23, with `$key = "e\u{0301}x"` (a decomposed `é`, then `x`):
`Str::startsWith($key, 'e')` is `true`, `Str::slice($key, Str::length('e') as int)` is `"x"`, so the
cut silently drops the `é`, and `Str::after($key, 'e')` is `null`. Two members of one class disagree on
one string. `find_from`'s doc (`str.rs:2785-2789`) says every search member reaches the boundary test,
and `replaceAll` does not.

The user's calls on 2026-11-23: every `Str` member keeps to graphemes, not only `startsWith` and
`endsWith`; a one-call prefix and suffix cut; and a compiler hint on the hand-written shape.

## Standing decisions

- **A match or a cut at either end of a match is on a grapheme boundary, in every `Core\Str` member.**
  `startsWith(s, p)` is `true` only when `s` begins with the bytes of `p` and `on_boundary(s, len(p))`
  (`str.rs:3202`). `endsWith(s, p)` likewise at `len(s) - len(p)`. An empty prefix or suffix is still
  found in every string. `replaceAll` takes a match at a position only when `whole_clusters` holds for
  it, in both its modes. *What it spends:* one `on_boundary` call per match found. Text that is ASCII
  around the match never reaches the segmenter (`str.rs:3208`), so the ASCII path adds two byte
  compares and no call or allocation.
- **A trim removes whole graphemes.** The `characters` set stays a set of code points, read once into
  `TrimSet` as today. A grapheme at the trimmed end is removed when the set holds **every** code point
  of it. So `trim("e\u{301}", { characters: "\u{301}" })` is `"e\u{301}"`, and the default set still
  removes a `\r\n` pair, which is one grapheme of two code points both in the set. *Why:* reading the
  set as graphemes would make the default set's `"\n\r"` one unit and stop it from trimming `\r\n`.
  *What it spends:* where the end is ASCII and not `\r\n`, nothing new. Elsewhere, the segmenter walks
  only the graphemes that are trimmed, plus one.
- **`lines`, `compare`, `upperFirst`, `lowerFirst` and `codePoints` stay as they are.** `lines` splits
  on ASCII line ends and takes `\r\n` whole. `compare` cuts nothing. The case members map the first
  code point, which is the base letter of the first grapheme. `codePoints` works on code points by name.
- **`Str::afterPrefix(string $s, string $prefix): ?string`** and **`Str::beforeSuffix(string $s,
  string $suffix): ?string`.** Each returns the rest of `$s` when `startsWith` / `endsWith` holds,
  otherwise `null`. An empty prefix gives `$s`. *Why these names:* they return `null` when there is
  no match, as `before` and `after` do, and the `after…`/`before…` names say so. `withoutPrefix` was
  the first name proposed. It was dropped because `Arr::withoutFirst` and `Path`'s `withoutExtension`
  never return `null`. *Why `null` and not `$s` unchanged:* the caller then needs no second
  `startsWith` to know whether it matched. The pair is listed with `before`/`after` in
  `rule:core-api/symmetric-names` (`docs/rules/core-api/symmetric-names.md:2`).
- **The new members cut with `Cut::window`**, the way `after` does (`str.rs:4089-4098`). The params are
  `CoreTy::Text(Qual::Contagious)` and `TextOrigin::Derived`, as `after`'s, so a tainted subject gives
  a tainted rest. *What it spends:* one byte compare of the prefix, one `on_boundary`, and the cut
  `after` makes. No search, no `length` and no `slice` walk.
- **`W1036` names a cut written by hand.** It is raised from the written tree with the checker's resolved
  calls, like `W1035` (`crates/nvs-ir/src/rereads.rs`). It is a new module,
  `crates/nvs-ir/src/hand_cuts.rs`, and it is not a row of `forms.rs`. The cheaper-forms table is for a
  form measured at 2x or more; this warning is about one call instead of three, and the prefix written
  once. The shapes, and only these:

  | Written | Named |
  |---|---|
  | `Str::slice(S, N)` reached only where `Str::startsWith(S, P)` is `true` | `Str::afterPrefix(S, P)` |
  | `Str::slice(S, 0, -N)` or `Str::slice(S, 0, Str::length(S) - N)` reached only where `Str::endsWith(S, P)` is `true` | `Str::beforeSuffix(S, P)` |

  `N` is `Str::length(P)`, with or without `as int`, or an integer literal equal to the grapheme length
  of a literal `P`. `S` is a plain variable not assigned between the test and the cut. `P` is a literal,
  a constant or a plain variable not assigned between them. "Reached only where" is either the `then`
  block of `if (Str::startsWith(S, P))`, or a later statement of the same block after
  `if (!Str::startsWith(S, P)) { return …; }` (or `throw`, `continue`, `break`). Anything else gets no
  warning: the match is conservative, as `rereads.rs`'s is.
- **The fix is offered only for one exact shape.** The `if` has no `else`, the first statement of its
  `then` block assigns or declares `$v` from the cut, and `$v` is not declared before the `if`. The fix
  is an `alternative` (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`) that writes
  `?string $v = Str::afterPrefix(S, P);` before the `if`, the condition `$v != null`, and the block
  without its first statement. Every other match warns with no fix, as `W1035` does. *Why:* any wider fix
  must pick a variable name or move a declaration across code the author wrote.
- **The tree's own hand-written cuts move to the new members** in stage 3, so the tree has no `W1036`.
- **Measured before it is written down:** the bench pair of stage 2 records the per-op counts of the
  hand-written cut and of `afterPrefix`. The `W1036` rule fragment states that figure. The warning is
  built whatever the ratio, because its reason is simplicity and not the cheaper-forms 2x bound.
- **`\r` or `\n` alone does not match inside a `\r\n` pair**, in `startsWith`, `endsWith` and
  `replaceAll`, as it already did not in `replace`, `indexOf` and the rest of the search family. So
  `replaceAll($s, ["\r" => ""])` leaves CRLF text unchanged, and `endsWith(Core\Env::EOL, "\n")` is
  `false` on Windows. A program names the pair: `["\r\n" => "\n", "\r" => ""]`. *Why:* `\r\n` is one
  grapheme, and an exception for it would make the one ASCII pair UAX #29 joins the one place a match
  splits a grapheme. The tree's twelve conformance cases that stripped or showed `\r` alone name the
  pair now, and stage 4's `replaceAll`, `endsWith` and `trim` pages say so with an example.
- **Out of scope:** `Core\Regex`, whose patterns name code points (`.` is one code point), and
  `Core\Bytes`, which is bytes by design.
- **Lanes:** stages 1 and 2 touch `str.rs` and run in order. Stage 3 touches `nvs-ir`, the diagnostics,
  the CLI and the LSP only, and could run beside stage 1 as a side goal once stage 2's registry rows
  exist (it needs the two new names to resolve). Stage 4 follows all three.

## Stages

### 1. startsWith, endsWith, replaceAll and the trims match only whole graphemes

**Does:** puts the boundary test into the six members, rewrites the test that pins the old answer, and
adds a test and a conformance case.

**Split:** none, one session.

- [x] `crates/nvs-stdlib/src/str.rs:2115` `nvs_core_str_starts_with` and `:2125`
  `nvs_core_str_ends_with`: the bytes compare and `on_boundary` at the cut, per *Standing decisions*.
  Each doc comment says so.
- [x] `str.rs:3451-3475` `nvs_core_str_replace_all`: a match at `at` counts only when
  `whole_clusters(subject, at, at + matched)`, in both modes. The comment near `:3470` says grapheme
  boundaries, not char boundaries.
- [x] `str.rs:2620` `trimmed`: remove whole graphemes per *Standing decisions*, keeping the ASCII path
  free of the segmenter. Its doc (`str.rs:2600-2614`) says what is removed, and `TRIM_DOC`
  (`str.rs:1301`) and the two cards beside it say it in user words.
- [x] `str.rs:2785-2789` `find_from`'s doc: "every member of the search family" now also covers
  `startsWith`, `endsWith` and `replaceAll`, and names `on_boundary` for the first two.
- [x] `str.rs:6039-6067` `contains_starts_with_and_ends_with_match_exact_bytes_and_find_the_empty_needle_everywhere`:
  the assertion at `:6067` that `startsWith("👩‍💻", "👩")` is `true` becomes `false`, and the doc
  comment and the test's name stop saying "exact bytes".
- [x] `str.rs:6588`, beside `a_search_never_matches_inside_a_grapheme`: the new test
  `a_match_at_the_edge_never_splits_a_grapheme`. It covers a mark alone, half a flag, a base letter
  without its mark, a ZWJ emoji's first code point, `\r` of a `\r\n` pair, for `startsWith`, `endsWith`,
  `replaceAll` in both modes and the three trims, and checks that the ASCII and empty cases are as
  before. Its `// covers:` line names the six members. Run the check `boundaries` once.
- [x] `tests/conformance/core/str-every-match-and-cut-keeps-whole-graphemes.nvst:1`: the probe of
  *Why* (`startsWith` agrees with `after`), a `replaceAll` that skips a mark, and a `trim` that keeps
  `e` with its mark and removes a `\r\n`. Run the check `boundaries-conformance` once.

### 2. Str::afterPrefix and Str::beforeSuffix

**Does:** adds the two members with their reference cards, tests, examples, benches and attacks.

**Split:** the four example directories, the two bench pairs and the two attacks go to one writing
subagent once the members build, with [prose.md](../agent/prose.md) and the `after` proofs
(`bun nv proofs --id 'Core\Str::after'`) in its brief. The session writes the members and the tests.

- [x] `str.rs:363`, after the `after` row: two `CoreMethod` rows, `afterPrefix` (`names: ["s",
  "prefix"]`) and `beforeSuffix` (`names: ["s", "suffix"]`), typed as *Standing decisions* says.
- [x] `str.rs:4099`, after `nvs_core_str_after`: `nvs_core_str_after_prefix` and
  `nvs_core_str_before_suffix`, each a `nvs_helper!` with `Cut::window` and the `apply` arm `after` has.
- [x] `str.rs:977`, after `AFTER_DOC`: `AFTER_PREFIX_DOC` and `BEFORE_SUFFIX_DOC`
  (`rule:core-api/reference-card`), in user words: what each returns, and `null` when the string does
  not start or end with it.
- [x] `str.rs`, beside `after_cuts_past_the_first_or_last_needle_and_answers_null_when_absent`
  (`:5988`): `after_prefix_and_before_suffix_cut_one_end_or_return_null`, covering a match, no match,
  the empty prefix, the whole string, a needle inside a grapheme and a tainted subject. Run the check
  `members` once.
- [x] `tests/conformance/core/str-after-prefix-and-before-suffix-cut-one-end-or-return-null.nvst:1`.
  Run the check `members-conformance` once.
- [x] `benches/members/core/Str/afterPrefix.nvs:1`, `afterPrefix.scale.nvs`, `beforeSuffix.nvs`,
  `beforeSuffix.scale.nvs`, shaped as `after.nvs` and `after.scale.nvs`. One more bench,
  `benches/members/core/Str/afterPrefix.nvs`'s second `#[Bench]`, runs the hand-written
  `startsWith` + `slice(length)` cut on the same input. Record both per-op counts in the handoff for
  stage 4's rule.
- [x] `tests/hostile/core/Str/afterPrefix/01-…nvs` and `tests/hostile/core/Str/beforeSuffix/01-…nvs`:
  a very long subject with a prefix that matches up to its last byte, and a prefix that ends inside a
  grapheme.
- [x] `docs/examples/core/Str/afterPrefix/` and `docs/examples/core/Str/beforeSuffix/`: `about.md` and
  three examples each. The first `afterPrefix` example is the user's shape with neutral names: a form
  key `openingHours-12` gives `12`.

### 3. W1036 names a prefix or suffix cut written by hand

**Does:** adds the warning, its pass and its fix, wires it into `nvs check` and the editor, and moves
the tree's own cuts to the new members.

**Split:** the list of hand-written cuts in the tree's `.nvs` files goes to an `Explore` subagent
(`Str::startsWith` or `endsWith` followed by `Str::slice` of the same variable, answer `file:line`).

- [ ] `crates/nvs-diagnostics/src/code/warnings.rs:381`: `W_HAND_CUT = Code::new("W1036")` with a card
  in user words, after `W_REPEATED_READ`.
- [ ] `crates/nvs-ir/src/hand_cuts.rs:1`: `find(files, exprs, checked_types)` over the written tree for
  the two shapes of *Standing decisions*, and `warnings(&[HandCut])` raising `W1036` with the fix where
  the exact shape holds. The module doc states the shapes, the fix rule and *What it spends*: one walk of
  each body while checking, nothing at run time.
- [ ] `crates/nvs-ir/src/ir.rs:56`: `Program::hand_cuts`, beside `rereads`.
  `crates/nvs-ir/src/lower/mod.rs:1577`: call `hand_cuts::find` beside `rereads::find`.
- [ ] `crates/nvs-cli/src/main.rs:1522` and `crates/nvs-lsp/src/document.rs:1405`: extend the
  warnings with `nvs_ir::hand_cuts::warnings(&program.hand_cuts)`.
- [ ] `crates/nvs-ir/src/hand_cuts.rs`, its test module: one test per shape that warns (`then` block,
  early return, suffix with `-N`, suffix with `length(S) - N`, literal `N`), the fix text for the exact
  shape, and one silent test per clause that fails (`S` assigned between, `N` not the prefix's length,
  an `else`, a `P` that is a call). Run the check `warning` once.
- [ ] The tree's cuts from the subagent's list, `apps/registry/src/App.nvs:1039` and
  `apps/registry/src/ActivityTest.nvs:154` among them, use `afterPrefix` or `beforeSuffix`. Then
  `cargo build` and `nvs check` on each changed app give no `W1036`.

### 4. the rules, the examples, the proofs and the website

**Does:** states the boundary rule and the warning in the rules, rewrites the changed members' pages,
and pays every proof the goal owes.

**Split:** the four `about.md` rewrites of the changed members go to one writing subagent once the rule
fragments are written, with [prose.md](../agent/prose.md) in its brief. The session writes the rules and
the change entries.

- [ ] `docs/rules/types.md:529` `rule:types/string-is-utf8` (its fragment under `docs/rules/types/`):
  every `Core\Str` member that matches or cuts does so at grapheme boundaries, and the trim set is read
  per *Standing decisions*. The six members and `lines`' exception are named.
- [ ] A new fragment under `docs/rules/expressions/` for `W1036`: the shapes, the fix rule, the measured
  counts of stage 2, and a Declined list: "a fix for every shape" (it would pick a variable name) and
  "a row of the cheaper-forms table" (its reason is not speed). Cite it from `hand_cuts.rs`. Then
  `bun nv rules --render`.
- [ ] `docs/rules/core-api/symmetric-names.md:2`: add `afterPrefix`/`beforeSuffix`.
- [ ] `docs/examples/core/Str/startsWith/about.md`, `endsWith/about.md`, `replaceAll/about.md` and
  `trim/about.md` (with `trimStart` and `trimEnd` if they have their own) say in one sentence that a
  match never splits a character, with an example where it shows.
- [ ] `website/src/data/changes.json`: `new` for `Core\Str::afterPrefix` and `Core\Str::beforeSuffix`;
  `fixed`, `breaking: true` for `Core\Str::startsWith`, `endsWith`, `replaceAll`, `trim`, `trimStart`
  and `trimEnd`, with an `instead`; `new` for the `W1036` warning. Every page `bun nv site --check stale`
  names is reread and stamped (`bun nv site --stamp`).
- [ ] `bun nv proofs --owed --goal whole-grapheme-cuts` names nothing.

### 5. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of the change entries, the two new `about.md` files and the `W1036` card
  ([prose.md](../agent/prose.md) § *A newcomer reads it*).
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done. `Core\Str::afterPrefix` and `beforeSuffix` are registered (`str.rs:377`,
symbols in `address` at `str.rs:1832`), cut with `Cut::window` after `startsWith`/`endsWith`'s own
test, and have cards, the unit test `after_prefix_and_before_suffix_cut_one_end_or_return_null`, the
conformance case, two `about.md` with three examples each, benches with growth, one attack each, and
every proof reviewed and stamped. The checks `members` and `members-conformance` are green, and
`bun nv proofs --id` owes nothing for either. A new member's symbol must also be added to `str.rs`'s
`address` match, or every program panics at registry build (3410 suite cases red at once).

The hand-written cut is `benches/members/core/Str/afterPrefix.twin.nvs` (the project's twin
convention, `benches/members/README.md`), not a second `#[Bench]`. Per op, on the same input:
`afterPrefix` 6.00 statements, 1.00 calls, 0.50 allocations, 24.75 bytes; the twin 9.00 statements,
2.00 calls, 0.50 allocations, 24.75 bytes (counted by `nvs run --count`; the ledger keeps the twin's
clock only, ratio 1.29x). Stage 4's `W1036` fragment states these. No test is expected red.
Stage 3 will warn on `afterPrefix.twin.nvs` itself: it is the hand-written cut on purpose, so it
needs the warning's allow, not a rewrite.

### Next

Stage 3, `W1036` (`crates/nvs-ir`, diagnostics, CLI, LSP), split as the stage's `**Split:**` line says.

- [ ] `crates/nvs-diagnostics/src/code/warnings.rs:375`: `W_HAND_CUT` after `W_REPEATED_READ`.
- [ ] `crates/nvs-ir/src/hand_cuts.rs:1`, modelled on `crates/nvs-ir/src/rereads.rs`.
- [ ] `crates/nvs-ir/src/ir.rs:56` `Program::hand_cuts`; `crates/nvs-ir/src/lower/mod.rs:1592` the
  `find` call.
- [ ] `crates/nvs-cli/src/main.rs:1522` and `crates/nvs-lsp/src/document.rs:1406`: the warnings.
- [ ] `crates/nvs-ir/src/hand_cuts.rs:1`, its test module, then the check `warning`.
- [ ] `apps/registry/src/App.nvs:1039` and the rest of the Explore subagent's list of the tree's
  hand-written cuts, with `benches/members/core/Str/afterPrefix.twin.nvs:1` kept.

### Backlog

- `W1036` could also name `Str::slice(S, N)` after `startsWith(S, P)` where the literal `N` is **not**
  the prefix's length: that is a bug, not a style point (`docs/rules/expressions/`).
- `Core\Regex` could offer a grapheme mode (`\X` or a flag) for patterns that must not split one.
