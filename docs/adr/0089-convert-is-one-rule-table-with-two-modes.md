# ADR 0089 — `mwl convert` is one rule table with two modes, and every emitted line is classified

- **Status:** Accepted
- **Date:** 2026-08-25
- **Scope:** what `mwl convert` promises and what it refuses — the two modes and the per-rule tier that
  produces them, what "equivalent" means and who proves it, the determinism contract, the
  nothing-is-dropped rule, the annotation and report format, where the three translation tables live, and
  the PHP front end with its 7.0–8.5 dialect range. Not in scope: which `Core` member a given PHP name
  becomes, which is [docs/spec/02-php-migration.md](../spec/02-php-migration.md)'s one row per name; the
  migration path for a construct MWL dropped, which belongs to the ADR that dropped it; M11's schedule and
  its verification targets, which are [the plan](../implementation-plan.md)'s; and database schema
  migration, which is an unrelated topic owned by [0082](0082-the-first-party-framework.md) § 7.
- **Amends:** [0080](0080-the-audience-mwl-is-built-for.md) § 3 — the porting-aid bullet now names the two
  modes and the report that carries the honest number, instead of describing the tool in prose.
  [docs/spec/02-php-migration.md](../spec/02-php-migration.md) — a row's **MWL** cell is machine-read when
  it is exactly one `Core` member spelling; any other cell must carry a rule id, so the converter never
  guesses at prose.
  [docs/implementation-plan.md](../implementation-plan.md) — M11's enumerated catalogue of rewrites moves
  into the rule table this ADR defines; the milestone keeps its schedule, its inference pass and its
  verification.
- **Relates to:** 0004, 0007, 0008, 0009, 0011, 0014, 0015, 0019, 0021, 0031, 0034, 0037, 0039, 0043,
  0049, 0050, 0051, 0052, 0061, 0062, 0063, 0065, 0068, 0069, 0071, 0079, 0081

> **In short:** `mwl convert` is **one rule table read through two filters**, not two translators. Every
> rule carries a **tier**: **E** — proven to behave identically, **D** — a mechanical MWL destination
> exists but the behaviour may differ, **N** — no mechanical destination at all. **`--mode=equivalent`**,
> the default, emits only E and comments out everything else with the *idiomatic* MWL replacement beside
> it: the output is a worklist that will not run, and is honest about why. **`--mode=runnable`** emits E
> and D as code, each D site carrying `TODO(convert:<id>)` naming exactly how it may differ, and comments
> out only N: the output usually runs and is explicitly not what MWL is for. **A tier is a claim someone
> discharged** — an E branch with no differential case against the PHP oracle fails CI, so "100% correct"
> is a test, never an opinion. **Determinism is a contract**: same bytes in, same bytes out, no clock, no
> hash order, no fixpoint, no model, no network. **Nothing is ever dropped** — every non-trivia byte of
> input leaves as either converted code or commented-out source. The front end is an existing Rust PHP
> parser behind our own facade, dialect-aware across PHP 7.0–8.5, and extending that range is a row.

## Context

- **This is an adoption path, and it is the only one that scales.** Every construct MWL removed has a
  named migration path in the ADR that removed it — [0008](0008-static-and-global.md)'s `global`,
  [0043](0043-interface-default-methods-and-delegation-replace-traits.md)'s traits,
  [0031](0031-callable-is-the-only-closure-type.md)'s `use (&$y)`,
  [0014](0014-property-observer.md)'s `__get`. Those paths are written for a human reading an ADR. A PHP
  library is tens of thousands of lines, and nobody applies twenty ADRs by hand across it.
- **The failure mode of every "PHP to X" tool is silence.** They emit code that looks converted, runs, and
  differs somewhere nobody looked. For [0080](0080-the-audience-mwl-is-built-for.md)'s audience —
  multi-tenant and regulated platforms — a silent behaviour change during onboarding is the worst thing
  the project can ship, because it arrives disguised as success.
- **But refusing to guess is also a failure.** A tool that converts only what it can prove leaves a
  developer with a file of comments and no sense of progress. Both audiences are real: one wants to see
  the thing run tonight, the other wants a list they can work through and sign off.
- **The two are the same table.** A rewrite is either proven, or unproven-with-a-destination, or has no
  destination. That is a property of the *rule*, not of the mode, so a mode is a filter over one table and
  not a second code path. This is the whole design, and everything below follows from it.
- **A converter is run many times.** Across a large tree, in CI, by several people, before and after
  cleaning up the source. If two runs over the same input differ by a byte, the diff is unreviewable and
  the tool is worthless as a repeatable step in a migration.
- **The name.** `mwl convert` is the spelling every existing document uses, and *migration* in this
  repository already means database schema migration ([0082](0082-the-first-party-framework.md) § 7).
  There is no second name and no alias, per [0015](0015-no-name-aliasing.md).

## Decision

### 1. One table, two modes, three tiers

Every rewrite the converter knows is a **rule** in one table. Every rule carries exactly one tier per
branch, and the two modes are that tier read through a filter:

| Tier | What it means | `--mode=equivalent` (default) | `--mode=runnable` |
|---|---|---|---|
| **E** | The converted construct behaves identically to the PHP one, for every input the converted program's type checker accepts | emitted as code, no annotation | emitted as code, no annotation |
| **D** | A mechanical MWL destination exists, and the behaviour may differ | original commented out, with the **idiomatic** MWL shape beside it | emitted as code, with `TODO(convert:<id>)` naming the difference |
| **N** | No mechanical destination exists | original commented out, with the idiomatic shape | original commented out, with the same note |

So `--mode=equivalent` output **will not run** — it is a worklist, and the header says so. `--mode=runnable`
output usually runs, is explicitly not idiomatic MWL, and every place it may diverge is one `grep` away.

**A rule has ordered branches, and a branch's tier may be predicated on a side condition.** The condition
is decided by the converter's own analysis; a condition it cannot decide is **false**, so control falls to
the next branch and the weaker tier applies. Fail-closed, in the same direction as
[0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md).

A rule record is data, and carries exactly these fields:

| Field | What it holds |
|---|---|
| `id` | stable for the life of the project, never reused — a domain letter plus four digits, e.g. `S0140` |
| `match` | the PHP construct or built-in name this branch set applies to |
| `when` | the side condition, in the closed predicate vocabulary of § 6.3; empty means "always" |
| `tier` | `E`, `D` or `N` |
| `rewrite` | the MWL shape emitted, for `E` and `D` |
| `diverges` | one sentence, for `D` — the input class where behaviour differs. This is the `TODO` text |
| `idiomatic` | for `D` and `N` — what MWL actually wants here. This is the comment `equivalent` mode leaves |
| `dialect` | the PHP version range the branch applies to (§ 7) |
| `proof` | for `E` — the differential case that discharges the claim. CI refuses an `E` branch without one |

`diverges` and `idiomatic` are different sentences on purpose: one says *what will break*, the other says
*what to write instead*. A `D` rule needs both, because the two modes ask it different questions.

### 2. "Equivalent" is a discharged proof obligation, never an assertion

A branch may be **E** only when, for every input the converted program's type checker accepts, the two
programs agree on all of: values returned, bytes written to each sink, which `Throwable` escapes, and the
order of externally visible side effects — evaluated against the *declared target dialect* (§ 7), not
against "PHP in general".

**Nothing is E because someone was confident.** An E branch names a differential case in `tests/convert/`
that runs the PHP fragment on the oracle build this repository already uses and the converted fragment
under `mwl test` ([0079](0079-testing-is-a-language-feature.md)), and compares. CI refuses an E branch
whose `proof` is missing or whose case does not run. A branch claiming E across several dialects owes one
case per dialect it claims; where no oracle exists for that dialect, the branch is **D**, not E.

**A tier is usually a property of the site, not of the construct.** Two worked examples, which are the two
shapes every rule in the table takes:

- **`==` → `===`.** E when both operands are proven to be the same scalar type — there PHP's two operators
  agree. D otherwise, and the `diverges` sentence differs per operand shape: two arrays compare
  order-insensitively under `==` and order- and type-sensitively under `===`; two objects compare by
  property values under `==` and by identity under `===`; and a cross-type comparison changed meaning in
  PHP 8.0, so the same source is a different rule branch under a 7.x dialect than under an 8.x one.
- **`strlen($s)`.** E when the argument is proven `bytes`, where both count bytes. D when it is `string`,
  because [0009](0009-string-and-bytes.md) counts grapheme clusters — the rewrite compiles and the number
  differs on any non-ASCII input, which is precisely a `TODO`, not a blocker.

**Where inference cannot decide a type, `mixed` is an E answer, not a divergence.**
[0007](0007-explicit-type-system.md) § 6 makes `mixed` the one unchecked position, which is exactly PHP's
own discipline; emitting it preserves behaviour honestly where a guessed annotation would not. The `TODO`
that accompanies it names the binding and says what it costs, per M11.

### 3. Determinism is a contract, and it is tested

Output is a pure function of: the input bytes of every file in the unit, the mode, the target dialect, the
rule-table digest, and the explicit flags. Nothing else. Concretely, and each of these is a rule the
implementation may not break:

1. **File discovery is sorted** by byte-wise path, and units are processed in that order.
2. **No ambient input.** No clock, no locale, no environment, no network, no random seed, no absolute path
   in output, no hash-map iteration order anywhere a decision or an emission order depends on it.
3. **The pass pipeline is fixed and each pass runs once**, over the tree in source order. There is no
   run-to-fixpoint: a fixpoint hides an order dependence that will surface as a diff a year later. A rule
   needing rewritten input names the earlier pass that produces it.
4. **Rule precedence is total**: within a pass, the innermost matching node first, and among rules matching
   one node, by rule id. Two rules that could both apply at one site are a table error CI catches, not a
   coin toss.
5. **Every generated name is a pure function of source facts** — the class a file's top-level functions are
   grouped into, a tracker class for a stateful trait, a fresh binding for a rewritten `$$var`. Where a
   counter is unavoidable it is per-file and in source order, and the rule says so.
6. **Formatting is not the converter's business.** It emits a tree and prints it through `mwl fmt`'s one
   unconfigurable style ([0039](0039-canonical-code-formatting.md)). The converter has no formatting
   options at all, and output is UTF-8 without a BOM with `\n` line endings on every platform.
7. **Every output file carries a header** naming the source path, the source digest, the rule-table digest,
   the mode and the dialect — so two runs that differ are attributable to one of those five inputs.

**No model, no heuristic outside the table, no probability.** A rewrite the table does not state does not
happen. This is not a performance decision or a purity one: it is what makes the output reviewable and the
tool re-runnable, and it is the reason § 8 refuses LLM assistance outright rather than as a matter of taste.

### 4. Nothing is dropped, and nothing unparseable is written

- **Byte-completeness.** Every non-trivia byte of input leaves the converter as either converted code or
  commented-out source. A construct with no rule is treated as tier N: commented, annotated, counted. The
  test for this is mechanical and runs over the whole corpus.
- **Comments and docblocks survive**, re-attached to the construct they documented. A converter that loses
  a library's documentation has not ported it.
- **A file the front end cannot parse becomes a fully commented-out file** carrying the parse error and the
  dialect it was tried under — never a missing file and never a silent skip.
- **Output is re-parsed with `mwl-syntax` before it is written.** A rule that produces unparseable MWL is a
  converter bug: the run reports it against the rule id and that file falls back to fully commented-out, so
  a bad rule can never leave a tree that does not parse.

### 5. Annotations, and the report that carries the honest number

One greppable spelling each, stable across releases:

```
// TODO(convert:S0140): Core\Str::length counts grapheme clusters; strlen counted bytes.
var $n = Core\Str::length($blob);
```

```
// convert:C0004 — PHP compares an int against a string here, and 8.0 changed what that means.
// Idiomatic MWL: convert once at the boundary, then compare — `$id === ($raw as int)`.
// if ($id == "1") { … }
```

`mwl convert --check` writes no files and emits a report — TOML, for the reason
[0064](0064-configuration-file-format.md) picked it, ordered by path then rule id so it diffs cleanly.
It carries per-tier counts, per-rule counts, the share of input constructs emitted as code in each mode,
and the rules that fired most often without an E branch — which is the work queue for the table itself.

**That report is the number [0080](0080-the-audience-mwl-is-built-for.md) § 3 obliges the project to
publish** instead of a compatibility claim. `--explain <id>` prints one rule: its branches, their tiers,
their conditions and their proofs.

### 6. Three tables, three homes, no fourth copy

1. **Names → [docs/spec/02-php-migration.md](../spec/02-php-migration.md)**, which already exists, already
   has a row per PHP built-in and is already CI-checked by `tools/check-migration.py`. The converter's name
   mapping is *generated* from it. To make that possible without a second copy: **a row whose MWL cell is
   exactly one `Core` member spelling is machine-read as a mechanical rename; any other cell must carry a
   rule id**, because prose like "`Core\Str::format` into `$file->write`" is a rewrite, not a rename. A
   `dropped` row with neither is a checker error once the converter exists.
2. **Constructs → `crates/mwl-convert/rules/*.toml`**, one file per PHP domain, one record per § 1's field
   list. It is **data, not code**, so a rule can be reviewed by someone who does not read Rust, and the
   browsable copy under `docs/spec/` is **generated and CI-checked identical** — never hand-edited, the
   same discipline [0065](0065-third-party-attribution-and-mwl-info.md) applies to notices.
3. **Semantic deltas → the ADR that created each one.** Grapheme counting is
   [0009](0009-string-and-bytes.md); string array keys are [0007](0007-explicit-type-system.md) § 5;
   array combination is [0069](0069-array-combination-is-key-type-independent.md); the closed doors are
   [0052](0052-closed-doors.md). A rule's `when` predicates are drawn from a **closed vocabulary** — the
   inferred type of an operand, its qualifier, a literal's shape, the dialect, whether a name resolves —
   and its `diverges` sentence cites the ADR by number rather than restating its reasoning.

The table will grow for years. That is the accepted price, and it is why the growth is one data row rather
than one branch in a match.

### 7. The front end is an existing Rust PHP parser behind our own facade

The converter needs an AST for PHP **7.0 through 8.5**, extendable. Requirements, in the order they decide
the choice:

- **Pure Rust**, per AGENTS.md's default and because a C parser eating source from a package you are
  evaluating is exactly [0051](0051-standard-library-tiers.md) § 4's question 2.
- **Spans on every node**, so a diagnostic and a `TODO` can point at the original line.
- **Comments and docblocks retrievable and attachable**, which § 4 requires.
- **A version knob**, because the same source means different things under 7.x and 8.x, and because PHP 8.0
  *removed* syntax that 7.x code uses — `$s{0}`, `(real)`, `create_function`, `each` are the closed list to
  check first.
- **Error recovery**, so one bad file yields a report rather than a stack trace.
- **A license this project can ship** and a maintenance record that will survive PHP 8.6, per
  [0068](0068-dependency-currency-and-the-version-contract.md).

**No PHP binary is required to convert.** PHP stays the differential oracle that *proves* an E rule (§ 2),
which is a development-side dependency this repository already has; requiring an installed PHP at convert
time would make output depend on which build the user happens to have, breaking § 3.

**The choice, with what is known today:**

- **`mago-syntax` — the leading candidate.** Pure Rust, MIT/Apache-2.0, ~30k SLoC, released within days of
  this ADR, the parser half of a PHP toolchain whose linter and formatter mean trivia and spans are
  first-class; it exposes a `cst` module and a companion `mago-php-version` crate modelling a version as
  `(major, minor, patch)`, which is the version knob in the shape this ADR needs.
- **`php-ast` — the named fallback.** Pure Rust, BSD-3-Clause, documents PHP 7.4–8.5, a typed AST with
  spans, comments stored separately, and a parser that always yields a tree. Against it: a single
  maintainer and a small user base, which is a sustainability risk this ADR would rather carry as a
  fallback than as the primary.
- Neither documents PHP 7.0–7.3 coverage. **That, not the API, is what the spike must measure**, and it is
  why the choice is not locked here.

**Locked in only after a spike**, whose acceptance test is stated in *Verification*. Until then and after,
the passes see only `mwl_convert::php` — our own facade over node kinds, spans and comments — so replacing
the dependency is a bounded change in one module rather than a rewrite of every pass.
[0065](0065-third-party-attribution-and-mwl-info.md) covers the notice either way.

**The parser is behind a Cargo feature and is not linked into the server binary.** A PHP front end has no
business on a machine serving requests.

**Extending the range is a row**, not a redesign: a new PHP version adds a dialect value, plus a branch on
any rule whose behaviour it changed.

### 8. What the converter never does

- **Never runs the input.** No `eval`, no autoload execution, no `composer install`, no bootstrap file.
  Conversion is a read of bytes.
- **Never converts `vendor/` by default.** [0080](0080-the-audience-mwl-is-built-for.md) § 3 scopes this
  tool to an application's own code; a dependency is reported against [0081](0081-packages-are-digests-resolution-is-a-maximum.md)'s
  registry — as a package that exists, one that does not, or a C extension that needs a Tier 1 `.mwlx`,
  which the converter cannot synthesise and says so.
- **Never invents a name binding.** A name that does not resolve under [0061](0061-compile-time-autoload-and-program-discovery.md)
  is reported, not guessed.
- **Never applies a rewrite that is not in the table**, and never asks a model for one.
- **Never claims compatibility in its own output.** The header states mode, dialect, digests and the tier
  counts; the forbidden phrasings of [0080](0080-the-audience-mwl-is-built-for.md) § 3 bind the converter's
  own text as much as any other document.

## Consequences

- **The maintenance cost is per rule, not per mode.** There is one table, one pass pipeline and one
  printer; the modes differ by a filter over a tier field. A second translator would have doubled every
  future rule.
- **`--mode=equivalent` will emit little on typical PHP at first, and that is the honest signal.** The
  share is measured by `--check` and published; the way to move it is E branches with proofs, one at a
  time, which is a queue anyone can work.
- **An E rule costs a differential test.** That is deliberately expensive: it is the only thing separating
  this design from every best-effort transpiler, and it is what lets the project say "identical" out loud.
- **`--mode=runnable` produces code MWL is not for**, on purpose, and says so per file. The `TODO` count is
  the cleanup backlog, and the same table's `idiomatic` field is what the cleanup is *toward* — so the two
  modes are the two ends of one road rather than two products.
- **Tradeoffs, per AGENTS.md.** *Performance:* none — the converter is development-time, off the request
  path, and feature-gated out of the server binary. *Memory:* not applicable for the same reason.
  *Usability:* a PHP developer gets either a working draft or a worklist, and in both cases every uncertain
  site is annotated rather than silent; against that, the default mode's first run looks discouraging.
  *Simplicity:* one table and two filters is simpler than the tool anyone would otherwise build, and the
  language gains nothing to remember — no rule here changes MWL itself.
- **This ADR adds no language surface, no `Core` member and no runtime cost.** Every decision it takes is
  about a development-time tool.

## Alternatives rejected

- **One best-effort mode, no tiering** — the usual shape. Rejected: it makes a silent behaviour change the
  default outcome for [0080](0080-the-audience-mwl-is-built-for.md)'s audience, and it gives the project no
  number it can honestly publish.
- **A confidence slider, or four or five levels** — rejected because nobody can state what level three
  promises. Two levels are two sentences: *proven identical*, and *has a destination, may differ*.
- **A `mwl/php-compat` package** carrying PHP's dropped semantics — an int-keyed array class, a loose
  comparison helper, byte-string members — for `runnable` mode to call. Considered seriously and rejected:
  the constructs that appeared to need it already have a language destination, because
  [0007](0007-explicit-type-system.md) § 5 normalises an integer subscript to its decimal string and `[] =`
  appends from PHP's own counter. What remains is a *difference in a result*, which is a `TODO` sentence,
  not a library. A shim would also let a converted application sit on PHP's semantics forever, which is the
  opposite of what the `idiomatic` field exists to drive.
- **A PHP-hosted front end** (`nikic/PHP-Parser` invoked through a PHP binary) — the most complete PHP
  parser in existence, and rejected anyway: output would depend on the installed PHP build, breaking § 3's
  contract, and it puts a PHP runtime in the path of a tool whose entire purpose is leaving one behind.
- **`tree-sitter-php`** — excellent recovery and wide grammar coverage, but a C parser reading source that
  arrives from outside ([0051](0051-standard-library-tiers.md) § 4 question 2), and a CST without the typed
  node vocabulary the rewrite passes need. Rejected on both counts.
- **Writing our own PHP front end.** Rejected under AGENTS.md's standing rule — anything with an external
  specification is a dependency — and on cost: PHP's grammar moves every year, and chasing it is a
  permanent tax paid to avoid a bounded facade. Reconsidered only under *Revisiting*.
- **LLM-assisted conversion for the unproven cases.** Rejected outright and recorded here so it is not
  re-opened as an obvious win: it is non-deterministic by construction, which § 3 forbids; it cannot
  produce a rule id, a tier or a proof; and its failure mode is a confident, wrong rewrite — the exact
  outcome this ADR exists to prevent. A model may help a *human* write a rule for the table, where the
  differential case checks it.

## Verification

**Now, in CI: nothing.** `mwl-convert` does not exist; M11 is where this becomes code, and the crate is
created when that milestone starts, per AGENTS.md. What this ADR fixes today is the shape M11 must build,
so that milestone's own catalogue of rewrites is now this table's contents rather than a second list.

**The spike that locks § 7's dependency**, before any pass is written: parse a corpus at every dialect from
7.0 to 8.5 — the `php-src` checkout this repository already uses for
`crates/mwl-syntax/tests/corpus_parse.rs`, plus a set of widely used libraries — and report, per candidate:
files parsed without panic, files parsed without error, whether every comment is retrievable with a span,
and behaviour on the closed list of constructs PHP 8 removed. A candidate that cannot represent 7.0–7.3
source is not disqualified by itself; it is disqualified if it cannot *report* what it could not parse.

**When M11 opens**, these are the tests the milestone owes, each traceable to a section above:

- **Every E branch has a differential case** that runs on the PHP oracle and under `mwl test`, and CI fails
  on an E branch without one (§ 2). This is the check the whole design rests on.
- **Byte-completeness** over the corpus: every non-trivia input byte accounted for in output (§ 4).
- **Determinism**: two runs byte-identical; a run with a permuted discovery order byte-identical; the
  Windows and Linux legs byte-identical (§ 3).
- **Output always parses**, in both modes, over the whole corpus (§ 4).
- **The rule table is total and unambiguous**: no two rules match one node at one precedence, every `D`/`N`
  branch has both `diverges` and `idiomatic`, every id is unique and never reused (§ 1).
- **The generated tables match their sources**: the name mapping regenerated from
  [02-php-migration.md](../spec/02-php-migration.md), and the browsable rule doc regenerated from the TOML,
  both byte-identical to what is committed (§ 6).
- **The published number**: `--check` over a fixed corpus, tracked per release the way M11 already tracks
  its `.phpt` pass rate.

## Revisiting

- **If the spike shows no crate can be made to cover 7.0–8.5** with spans and comments, § 7's rejection of
  our own front end is reopened — and only that section, since the facade means nothing else changes.
- **If the published equivalent-mode share stays so low that nobody runs the default mode**, the thing to
  revisit is the *definition* of E — for instance, admitting "equivalent under a stated, checked assumption
  about the input" as a fourth tier with its own annotation — not the two-mode split, which is what makes
  the claim auditable at all.
- **If a dialect older than 7.0 is asked for**, it is a dialect value and a set of rule branches, and this
  ADR does not need to change.
