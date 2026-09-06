Three neighbouring hazards are deliberately outside the check, and saying so is what stops a future
session from adding a fourth mechanism.

**Zero-width and invisible characters** are not targeted: the false-positive cost is real — one is
*required* in Persian, another in Indic scripts and in every multi-codepoint emoji sequence — and with
identifiers ASCII-only they can no longer make two names look alike. They can pad a string, which is a
deception with no mechanism behind it. **Homoglyphs** are closed structurally by the ASCII identifier
rule, so a confusables table would be dead weight. **The full bidirectional algorithm** is not
implemented, because Novis does not render text.

With the predicate's callers in place, the language is closed against the Trojan Source class —
identifiers by construction, everything else by the predicate. One cost is accepted and named: an
editor or diff viewer that does not itself neutralize these controls still misrenders a file *before*
the compiler sees it, so the check protects the merge and not the reading.
