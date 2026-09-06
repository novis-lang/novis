# `rule:statements/nvs-is-the-only-open-tag` — `<?php` and `die` are rejected; `<?nvs` and `exit` are the only spellings kept

- **Status:** Accepted
- **Date:** 2026-08-22
- **Scope:** the `<?php` code-mode open tag (`TokenKind::OpenTagPhp`) and the `die` keyword
  (`Keyword::Die`); the corresponding diagnostics. `<?nvs`, `<?=`, and `exit` are untouched.
- **Amends:** [0021](0021-single-file-inclusion-construct.md)'s framing paragraph in
  `docs/spec/00-overview.md` § 1, which named `<?php` acceptance as testing "the pragmatic-superset promise
  applies to the tag itself, not only to what is inside it" — that promise is now spent on this one
  construct, the same way `rule:types/no-legacy-cast` spent it on the legacy cast syntax. [0034](0034-legacy-cast-syntax-rejected.md)'s
  own *Consequences* section, which named `<?php` as an example of PHP syntax this project "kept but
  reinterpreted," is corrected below — it no longer does.
- **Amended by:** 0100

> **In short:** `<?php` no longer opens code mode, and `die` no longer terminates the process. Each is a
> parse-time diagnostic (`E0229` and `E0228` respectively) naming the sole survivor — `<?nvs` for the tag,
> `exit` for termination. Both were pure duplicate spellings with zero behavioral difference from the form
> kept, unlike every other PHP language pair this project has examined (`include`/`require`, `and`/`&&`),
> which at least differed in precedence or scope before being collapsed. Now that PHP source needs a
> conversion pass through `nvs convert` regardless, keeping a second spelling of a construct that behaves
> identically to the first buys nothing.

## Context

- **`<?php` vs. `<?nvs`.** `docs/spec/00-overview.md` § 1 accepted `<?php` as a second, identical-parse
  spelling of `<?nvs` specifically to test how far the "pragmatic superset of syntax" promise should reach —
  it said so explicitly. That was a reasonable question to ask once; the answer, on reflection, is that a
  file-opening tag is exactly the kind of one-line, unambiguous, purely mechanical edit `nvs convert` was
  always going to make regardless (every `rule:types/bytes`/0023/0034/0043-family gap already documents converter work
  strictly harder than a literal string substitution). Keeping `<?php` bought nothing beyond one less
  find-and-replace in a tool that already exists.
- **`exit` vs. `die`.** Surveyed against ten other languages with a "terminate the process" primitive (C,
  Go, Rust, C#, Java, Python, Ruby, Perl, JavaScript, Swift), every one keeps at most one plain spelling of
  it. Where a second primitive exists at all (Python's `os._exit`, Ruby's `exit!`, Rust's
  `process::abort`), it is a genuinely different behavior — skips cleanup, can't be caught, crashes instead
  of exiting — never a bare synonym. PHP's `exit`/`die` pair is the outlier: `nvs-syntax` already parses both
  into the exact same AST shape with the exact same optional argument, and always has — there is no
  behavioral question here to preserve, only a naming one.
  - Notably, PHP's `die` is itself a naming accident: Perl's `die` (the origin of the name) raises a
    *catchable* fatal error, distinct from Perl's own `exit`. PHP borrowed the word but not the semantics,
    producing a plain duplicate rather than the two-tier distinction Perl actually has.
- Both are the same shape `rule:statements/nothing-gets-a-second-name` already rules against: a second runtime/syntax-reachable spelling of one
  thing, extended here from names (classes, imports) to a tag and a keyword.

## Decision

### 1. `die` is rejected; `exit` is the only process-termination keyword

`nvs-syntax` still recognizes `die` (and its optional `(status)`/`(message)` argument, parsed identically to
`exit`'s) so the diagnostic can point at the fix precisely — but it produces `ExprKind::Error`, not
`ExprKind::Exit`. `ExprKind`'s surviving variant is renamed from `ExitOrDie` to `Exit`, since `die` can no
longer reach it.

```php
die;             // rejected — "use `exit` instead — it is the only process-termination keyword Novis keeps"
die('bye');      // same diagnostic; `exit('bye')` is the exact replacement
exit;            // unaffected
exit(1);         // unaffected
```

Diagnostic: `E0228`.

### 2. `<?php` is rejected; `<?nvs` is the only code-mode open tag

The lexer still recognizes `<?php` and switches to code mode on it exactly as `<?nvs` does — purely so the
parser can name the fix instead of misreading the rest of the tag body as inline HTML. The parser reports a
diagnostic every time it consumes an `OpenTagPhp` token (file start or a mid-file reopen alike) and then
continues parsing the following code normally; nothing after the tag is affected.

```php
<?php echo 1; ?>          // tag rejected, "echo 1;" still parses as code
<?nvs echo 1; ?>          // unaffected
if ($x) { ?>html<?nvs }   // unaffected — <?nvs reopening mid-block was always legal
```

Diagnostic: `E0229`. `<?=` (short-echo) is untouched — it was never a spelling of `<?nvs`, it is sugar for
`<?nvs echo`.

**One file shape reaches code mode without a tag, and it is not a second spelling of one.** A file whose
first two bytes are `#!` is a script: line 1 is trivia and the file continues in code mode exactly as if
`<?nvs` stood there, so a CLI program written for the shebang the operating system already requires does not
also pay for a tag that has no surrounding template to delimit. `<?nvs` remains the only code-mode open
*tag*, and writing one in a shebang file before any `?>` is `E0009`. The rule, its bounds and why the
simplicity cost is accepted are `rule:tooling/shebang-opens-code-mode`
.

## Consequences

**Positive**

- Exactly one code-mode open tag and one process-termination keyword — nothing to disambiguate in
  documentation, `nvs-fmt`, or a code review comment.
- `ExprKind`'s `Exit` variant now names what it actually does; the rename removes the only case in the AST
  where a variant's name listed a keyword that could no longer produce it.
- Neither change touches the type checker or `nvs-ir`: `die`/`exit` were always type-checked and lowered
  identically (`crates/nvs-types/src/expr.rs`'s shared match arm), and `<?php`/`<?nvs` were always the same
  token downstream of the lexer/parser boundary. This is a pure surface-syntax narrowing, not a semantics
  change.

**Negative**

- **Two more line items for `nvs convert`'s (M11) mechanical rewrite pass**, on top of the ones `rule:statements/require-is-the-only-inclusion-construct` and
  `rule:types/no-legacy-cast` already added: `<?php` → `<?nvs` (a literal 5-character substring rewrite, no operand analysis
  needed) and `die(...)` → `exit(...)` (rename only, argument shape is already identical). Both are
  strictly simpler than the `(int)$x` → `$x as int` rewrite `rule:types/no-legacy-cast` already committed the converter to.
- **A further, small subtraction from the "pragmatic superset" promise**, in the same vein as `rule:types/no-legacy-cast` and
  `rule:expressions/no-keyword-logical-operators`. A PHP file opening with `<?php` or calling `die(...)` anywhere no longer parses unconverted.
  Smaller than either of those precedents: both rewrites here are pure renames with no operand or precedence
  reasoning involved at all.

## Alternatives rejected

- **Keep `<?php` and `die` as permanent aliases, change nothing.** The status quo — argued against above:
  neither spelling differs in behavior from the one kept, so a second spelling buys nothing but a second
  thing to teach, in a project that has already refused exactly that trade three times (`rule:statements/require-is-the-only-inclusion-construct`, `rule:types/no-legacy-cast`,
  `rule:expressions/no-keyword-logical-operators`).
- **Keep `die` but drop `<?php`, or vice versa.** Considered, since they're unrelated constructs bundled into
  one ADR only because they share a reasoning shape. Rejected as inconsistent: both are pure duplicate
  spellings with no behavioral distinction, so keeping one and not the other would need a reason neither
  case actually has.
- **Give `die`/`exit` (or `<?php`/`<?nvs`) the kind of two-tier distinction Ruby's `exit`/`exit!` or Python's
  `sys.exit`/`os._exit` have** (e.g. `die` skips some cleanup step `exit` runs). Rejected: no such
  distinction exists in Novis's request/process model to hang a second keyword on, and inventing one just to
  keep two spellings would be solving a problem this project doesn't have.

## Verification

- `crates/nvs-syntax/src/parser.rs`'s `die_is_diagnosed_naming_exit` test asserts `E0228` for `die;`,
  `die();`, and `die('bye')`, and that `exit` in every equivalent shape is unaffected.
- `crates/nvs-syntax/src/parser.rs`'s `php_open_tag_is_diagnosed_naming_nvs_tag` test asserts `E0229` for a
  file opening with `<?php`, and that the code following the tag still parses correctly rather than being
  swallowed as inline HTML.
- `ExprKind::ExitOrDie` is renamed to `ExprKind::Exit` in `crates/nvs-syntax/src/ast.rs`; every match arm in
  `nvs-types`, `nvs-hir`, and `nvs-syntax` (`casing.rs`) that referenced the old name was updated —
  mechanical, since none of them branch on which keyword produced the node.
- `crates/nvs-syntax/src/lexer.rs`'s existing `<?php`-lexing test is retitled
  `php_tag_still_lexes_as_its_own_token` and its doc comment now states plainly that the tag is
  lexer-recognized-but-parser-rejected, not accepted.
