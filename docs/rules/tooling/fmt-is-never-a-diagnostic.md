Nothing wires `nvs fmt` into `nvs check`, `nvs run`, `nvs test` or any other compiler command. An
unformatted file is never a diagnostic — not an error, not a warning — and never blocks compilation or
execution. `nvs fmt` is a separate tool a person or a CI job chooses to run, the boundary `cargo fmt` keeps
from `cargo build`. It is the deliberate counterpart of `rule:core-api/identifier-casing`: casing is a
hard compile error with no suppression, and formatting sits at the opposite end of the same axis, entirely
outside the compiler's diagnostic surface, where a stylistic diagnostic would blur "this program is wrong"
against "this program looks different than I would write it".

An editor may run `nvs fmt` and a set of quick fixes together on one keystroke, and that composition
happens in the client, never inside `nvs fmt`. Format-on-save beside a fix-all-on-save list is how both
target editors are already shaped. Each fix is a diagnostic-backed code action — a mis-ordered
`tainted secret string` (`rule:security/secret-qualifier`) is a parse error whose diagnostic already names
the fix — applied against the resilient tree, to a file that may not parse at all.

Keeping the two contracts apart is what `--check` needs: it fails for exactly one reason, so a CI job
never has to tell "laid out differently" from "semantically wrong". A separate `nvs fix` batch verb was
declined for the same reason — a second rule table beside the formatter's, which no one has asked for.
