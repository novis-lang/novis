The predicate has one implementation and several callers. **The lexer is an error**: one token — a
string literal, a comment, or an inline-HTML run — and additionally each line inside a multi-line one,
so a heredoc cannot hide a scope across its lines. A doc comment is that same span and reuses that
same check rather than growing a second one, so the tools that render one emit text the lexer has
already accepted. **The sinks substitute**: terminal output, whose span is one `echo` operand or one
`Core\Cli::write` call, and `Core\Html::escape`, whose span is one call, each replace an unmatched
control with `�` (U+FFFD). The diagnostic record applies the same predicate once, when a record is
built, so a log line, a dump and a stack trace carry the same answer into every rendering
(`rule:errors/record-transformations`). Deciding the rule separately at each caller is how one rule
acquires several slightly different definitions, so a second copy of the predicate is a bug.

The asymmetry is deliberate. Source is written by a developer who can fix it, and a failed build names
the file and line; runtime data arrives from outside, where refusing to print it would turn a display
concern into an availability one. Substituting rather than deleting keeps the property that a
neutralized byte is *visible* — the reader sees that something was removed instead of silently reading
a shorter string.

**No suppression at the lexer.** A file that legitimately needs an unterminated control does not
exist, and an attribute that switched off a security check would be the first exception this project
grants.

**Declined:**
- **A warning rather than an error at the lexer.** A warning nobody fails a build on is not a
  control, and the project has no suppression story for spelling rules
  (`rule:core-api/identifier-casing`).
- **Handling it only in `nvs fmt`.** The formatter is never wired into `nvs check`
  (`rule:tooling/fmt-is-one-canonical-style`), so a formatter-only rule is one an attacker's change
  simply does not run.
- **Handling it only at the sinks, not in source.** That inverts the severity: the source case is the
  one with a working exploit against code review, and the runtime cases are display deception.
