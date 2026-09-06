The predicate has one implementation and several callers. **The lexer is an error**: one token — a
string literal, a comment, or an inline-HTML run — and additionally each line inside a multi-line one,
so a heredoc cannot hide a scope across its lines. A doc comment is that same span and reuses that
same check rather than growing a second one, so the tools that render one emit text the lexer has
already accepted. **The sinks substitute**: terminal output and the HTML escape each replace an
unmatched control with a visible replacement character.

The asymmetry is deliberate. Source is written by a developer who can fix it, and a failed build names
the file and line; runtime data arrives from outside, where refusing to print it would turn a display
concern into an availability one. Substituting rather than deleting keeps the property that a
neutralized byte is *visible* — the reader sees that something was removed instead of silently reading
a shorter string.

**No suppression at the lexer.** A file that legitimately needs an unterminated control does not
exist, and an attribute that switched off a security check would be the first exception this project
grants.
