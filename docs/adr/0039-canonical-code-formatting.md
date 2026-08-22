# ADR 0039 — `mwl fmt` is the one canonical, unconfigurable formatting style, run on demand only

- **Status:** Accepted
- **Date:** 2026-08-22
- **Scope:** the formatting *rules* `mwl fmt` (M10) applies — indentation, brace placement, blank lines,
  modifier order, quoting, trailing commas, import ordering, and the reflow model — plus its CLI surface
  (`mwl fmt`, `mwl fmt --check`, `mwl fmt --diff`). Not `mwl-lsp`'s `textDocument/formatting` wiring or the
  editor clients, which [ADR 0016](0016-ide-integration.md) already settles; not the parser or checker.
- **Relates to:** [0016](0016-ide-integration.md) (names `mwl-fmt` as the one formatting implementation
  both editors call — this ADR is what that implementation actually does); [0029](0029-identifier-casing-is-checked.md)
  (the closest existing precedent: a style question given one hard, unconfigurable answer); every ADR whose
  construct needs a layout rule with no PER precedent — [0007](0007-explicit-type-system.md) (type
  grammar), [0010](0010-enums-are-a-value-type.md) (enum cases), [0024](0024-taint-tracking-for-injection-sinks.md)/
  [0033](0033-secret-qualifier-for-confidential-values.md) (`tainted`/`secret`), [0031](0031-callable-is-the-only-closure-type.md)
  (`fn` closures), [0036](0036-anonymous-object-shapes.md) (object literals, shape types),
  [0038](0038-lateinit-property-modifier.md) (`lateinit`).

> **In short:** `mwl fmt` rewrites a `.mwl` file into one canonical layout, deterministically — running it
> twice produces byte-identical output the second time. The style is [PER](https://www.php-fig.org/per/coding-style/)
> (PHP-FIG's Evolving Recommendation, PSR-12's successor) wherever MWL's grammar matches PHP's, extended
> with explicit rules for the constructs PER has never seen (`fn` closures, `tainted`/`secret`, `lateinit`,
> shape types, `match`). Three things make it fully deterministic rather than merely "PER-flavoured": it
> never reflows an expression to fit a width — an author's own line breaks inside an expression are
> preserved, and only what surrounds them is normalized (gofmt's model, not Prettier's); it takes **no
> configuration at all**, ever — no config file, no style-changing flag, the same "no suppression
> mechanism" stance ADR 0029 already takes for casing; and it is a separate, opt-in tool, never wired into
> `mwl check`/`mwl run` — an unformatted file is not a compiler warning, let alone an error. `mwl fmt`
> rewrites in place; `mwl fmt --check` is the non-mutating mode that reports which files would change and
> exits non-zero, the "warns without touching anything" behaviour a CI job or pre-commit hook runs.

## Decision

### 1. Base style: PER, for every construct MWL shares with PHP

- 4-space indentation, no tabs; one statement per line; files end with exactly one trailing newline; no
  trailing whitespace on any line.
- **K&R brace placement** for every control structure — `if`/`elseif`/`else`, `while`, `do`/`while`, `for`,
  `foreach`, `switch`/`case`/`default`, `try`/`catch`/`finally`: the opening brace stays on the same line as
  the keyword/condition, preceded by one space; the closing brace starts its own line; `elseif`/`else`/
  `catch`/`finally` continue on the same line as the preceding closing brace. `elseif` is one word, never
  `else if`, matching PER.
- **Allman brace placement** for every declaration with a body — `class`/`interface`/`trait`/`enum`, and a
  named function or method: the opening brace starts its own line at the declaration's own indentation.
- Exactly one blank line after a `namespace` declaration, one after the `use`-import block (§ 6), and one
  between two class members that each have a body (methods, and enum cases that carry one); no blank line
  is inserted between adjacent simple property or constant declarations.
- **Modifier order is canonical, not author-chosen**: `abstract`/`final`, then visibility (`public`/
  `protected`/`private`, including the asymmetric-visibility form `private(set)`), then `static`, then
  `readonly`, then `lateinit` — one space between each. The parser itself accepts these in any order
  (`crates/mwl-syntax/src/parser.rs::parse_modifiers` loops over the modifier keywords with no ordering
  check), which is exactly why this needs a formatting rule: without one, `static public $x;` and
  `public static $x;` would both compile and never converge.

### 2. No reflow — the gofmt model, not Prettier's

`mwl fmt` never decides whether an expression, call-argument list, array/object/shape literal, `match` arm
list, or enum-case list spans one line or several — that choice is the author's, and the formatter
preserves it exactly. What it *does* normalize around that choice: indentation of continuation lines,
spacing, and brace placement per § 1. Concretely, `mwl fmt` never collapses a hand-wrapped multi-line call
onto one line, and never splits a long one-line call across several. There is deliberately no line-length
rule anywhere in this ADR, soft or hard: with no reflow decision to make, a width limit would be advisory
prose with nothing in the tool to enforce it.

### 3. Zero configuration, permanently

No config file, no per-project or per-directory override, no CLI flag that changes output. `mwl fmt`'s
result for a given input is a pure function of that input and nothing else — the same stance
[ADR 0029](0029-identifier-casing-is-checked.md) already takes for identifier casing, extended from naming
to layout. A configurable knob would let two files in the same project, or two projects run through the
same tool, disagree about what "formatted" means — exactly the property ADR 0016's own verification line
depends on ("both editors agreeing byte-for-byte on the same file's formatted output"). The only flags
`mwl fmt` accepts are I/O-mode ones: `--check`, `--diff`, `--stdin`, and the target path(s)/glob — never a
style knob.

### 4. Quote normalization

String literals are rewritten to single quotes, except when the literal needs interpolation (only a
double-quoted string or a heredoc can interpolate, unchanged from PHP) or contains a literal single quote
that single-quoting would otherwise force to be escaped — either case uses double quotes instead. Heredoc/
nowdoc bodies and comments are left byte-for-byte untouched: rewriting a heredoc's body would change the
program's own string value, not just its layout.

### 5. Trailing commas

Every comma-separated list that spans more than one line — call arguments, parameter lists, array
literals, shape-type fields, object literals, `match` arm lists (including the `default` arm), multi-line
enum-case lists — gets a trailing comma after its last element. A list kept on one line never gets one.
This removes the one place PHP/MWL's grammar leaves a genuinely free stylistic choice with no way to derive
the "right" answer from context.

### 6. Import (`use`) statement ordering

MWL's `use` grammar (`crates/mwl-syntax/src/ast.rs::UseDecl`) holds exactly one imported path per
statement — there is no PHP-style grouped `use A\{B, C};` form to worry about. `mwl fmt` sorts consecutive
`use` declarations lexicographically by their full path, ascending, case-sensitive, with no blank line
between them; exactly one blank line separates that block from the `namespace` line above and the first
real declaration below, per § 1.

### 7. Constructs with no PER precedent

- **`tainted`/`secret` qualifiers** ([0024](0024-taint-tracking-for-injection-sinks.md),
  [0033](0033-secret-qualifier-for-confidential-values.md)) sit one space before the type they qualify, and
  before a leading `?`: `tainted ?string $x`, never `?tainted string $x` (the grammar already fixes this
  order — this is purely the spacing rule).
- **`lateinit`** ([0038](0038-lateinit-property-modifier.md)) takes its place in the modifier order from
  § 1.
- **`fn` closures** ([0031](0031-callable-is-the-only-closure-type.md)): brace placement follows § 1's
  split by *kind of body*, not by whether the closure is named — a closure is an expression, so its body
  brace stays on the same line as its parameter list/return type, the same rule PER already gives PHP's
  anonymous functions: `fn (int $x): int { return $x + 1; }`, or, with a multi-line body, the opening brace
  stays on that first line and only the closing brace gets its own.
- **`match` expressions**: each arm on its own line unless the whole arm list was already written on one
  line (§ 2's no-reflow rule applies same as anywhere else); a trailing comma after the last arm per § 5.
- **Object literals and shape types** ([0036](0036-anonymous-object-shapes.md)): one space after `{` and
  before `}` when kept on one line (`{a: 1, b: 2}`); across multiple lines, one field per line, indented one
  level from the opening brace, trailing comma per § 5.
- **Enum cases** ([0010](0010-enums-are-a-value-type.md)): one per line when the author already wrote them
  that way (no-reflow), trailing comma when multi-line.
- **Attributes**: not applicable — MWL has no annotation syntax ([0011](0011-functions-and-constants-are-class-members.md)).

### 8. Idempotence, and what "formatted" means for a range

`mwl fmt path...` must be a fixed point: reformatting its own output changes nothing. This is what makes
`--check` well-defined (§ 9) and is the property M10's plan verify line already names ("`mwl fmt` is
idempotent across the whole corpus"). `mwl-lsp`'s `textDocument/rangeFormatting` applies the identical rule
set to a sub-range of a file — a range restriction on where the rules apply, never a second rule set.

### 9. CLI surface and enforcement posture

- `mwl fmt <path>...` rewrites the named file(s) in place.
- `mwl fmt --check <path>...` (alias `--dry-run`) writes nothing; it prints which files would change and
  exits non-zero if any would — the mode a CI job or pre-commit hook runs, mirroring `rustfmt --check`.
- `mwl fmt --diff <path>...` prints a unified diff instead of a bare file list.
- **None of this is wired into `mwl check`, `mwl run`, `mwl test`, or any other compiler command.** An
  unformatted file is never a diagnostic — not an error, not a warning — and never blocks compilation or
  execution. `mwl fmt` is a separate, opt-in developer tool a person or a CI job chooses to run, the same
  boundary `cargo fmt` keeps from `cargo build`. This is the deliberate counterpart to
  [ADR 0029](0029-identifier-casing-is-checked.md): casing is a hard compile error with no suppression;
  formatting is the opposite end of the same axis, entirely outside the compiler's diagnostic surface.

## Consequences

**Positive**

- Two files from two different authors, or two different editors (ADR 0016), converge to the same bytes
  once formatted — a testable invariant this ADR is what makes well-defined, rather than an aspiration
  sitting only in ADR 0016's *Verification* section.
- No reflow algorithm to design or implement (§ 2) keeps `mwl-fmt` close to a whitespace/brace/order
  normalizer walking the existing parse tree, not a new doc-printer engine — a small, second consumer of
  `mwl-syntax`, in the same spirit as ADR 0016 § 1 already frames `mwl-fmt` and `mwl-lsp` as the only two
  consumers of "language smarts."
- Zero configuration (§ 3) removes an entire category of PR bikeshedding and a `.mwl-fmt.toml` nobody needs
  to review — the same benefit ADR 0029 already banked for casing, extended to layout.
- `mwl fmt --check` (§ 9) gives a team exactly the "warn when it doesn't match" workflow this ADR was asked
  to provide, without the compiler ever holding a stylistic opinion.

**Negative**

- Preserving the author's own line breaks (§ 2) means `mwl fmt` cannot repair a badly-wrapped multi-line
  call by itself — a human still decides when an expression is long enough to wrap, and the formatter
  faithfully re-indents whatever shape it is given. Accepted: a Prettier-style reflow needs a materially
  larger implementation (a width-fitting doc printer) this project has no other user of, and the no-reflow
  model stays trivially byte-for-byte deterministic as the parser evolves.
- Several rules in § 7 have exactly one contributor — this ADR — rather than an existing PER convention to
  defer to. If a different convention emerges later, changing one is a breaking rewrite of every
  already-formatted `.mwl` file, the same cost class ADR 0029 already accepted for casing.
- Quote normalization (§ 4) and trailing-comma insertion (§ 5) rewrite bytes beyond pure whitespace, so a
  future change to either rule is a real diff across an entire codebase, not a settings change — mitigated
  by treating `mwl-fmt`'s rule set as stable once M10 ships it, the same promise `rustfmt`'s stable subset
  makes.

## Alternatives rejected

- **Prettier-style width-based reflow.** Rejected in § 2: more opinionated output, at the cost of a
  materially larger implementation this project has no other use for.
- **A configurable style** (a small stable-knob set, or a full config file). Rejected in § 3 for the same
  reason ADR 0029 gives casing no suppression mechanism: configurability reopens the exact "which style is
  this file in" question a canonical formatter exists to close.
- **Folding formatting into `mwl check` as a warning diagnostic.** Rejected in § 9 — the compiler already
  carries real hard-error surface (casing, definite assignment, taint); adding a purely stylistic one would
  blur "this program is wrong" against "this program looks different than I'd write it," and the user
  asking for this feature explicitly wants it kept a separate, opt-in tool.
- **A whitespace-only formatter that leaves quotes and trailing commas exactly as written.** Rejected: two
  semantically identical files would still differ byte-for-byte after formatting, which undercuts the
  entire point of having one canonical formatter.

## Revisiting

- A future ADR could add width-based reflow for the narrow set of positions where PER itself names a soft
  column limit, if long unformatted lines turn out to be a real pain point in practice. This ADR declines
  to build that for v1; it does not close the door on it.
- Range-formatting edge cases beyond what [ADR 0016](0016-ide-integration.md) already scopes (partial
  statements, mid-expression selections) are LSP wiring, not a rule change here.

## Verification

(M10, once `mwl-fmt` exists — mirrors the plan's existing M10 verify line, now specified precisely enough
to test against)

- `mwl fmt` run twice on the same file produces byte-identical output the second time, across the whole
  fixture corpus (idempotence, § 8).
- `mwl fmt --check` exits `0` on an already-formatted file and non-zero (naming the file) on one that
  isn't, in both cases without writing to disk.
- One fixture per section above: modifier reordering (`static public readonly $x;` → canonical order, § 1);
  quote normalization and its interpolation/escaping exceptions (§ 4); trailing-comma insertion on a
  multi-line list and its absence on a single-line one (§ 5); `use`-block sorting (§ 6); brace placement
  compared pairwise across a control structure, a class body, an `fn` closure, and a `match` expression
  (§§ 1, 7); and a no-reflow case where a hand-wrapped multi-line call is re-indented but never
  collapsed or re-wrapped (§ 2).
- A `tainted`/`secret`-qualified property and a `lateinit` property both format consistently (§ 7) —
  `mwl fmt` never needs to check whether either is used *legally*, only that it is laid out consistently;
  legality stays covered by ADR 0033's and ADR 0038's own checker fixtures.
