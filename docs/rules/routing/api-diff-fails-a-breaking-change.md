`nvs api diff <old.json> <new.json>` compares two generated documents as JSON values rather than as
text, prints one line per change worst-first, and exits non-zero if any change is **breaking**.
Breaking is the ordinary API-compatibility reading: a removed operation, a removed or newly
required parameter or property, a narrowed type, a removed enum case, a removed status code, a
tightened constraint. **Additive** is a new operation, a new optional parameter or property, a new
enum case, a widened type. **Cosmetic** is everything else — a title, a version, a summary, an
`operationId`, a tag.

The classification is mechanical, derived from schemas the compiler produced, not a reviewer's
judgement, and it errs towards breaking: a removed parameter is breaking whether or not it was
optional, and a changed `format` is breaking because the two schemas accept different strings.
**There is no suppression mechanism** — a gate with an escape hatch is advisory, and if one proves
necessary it is an additive change later.

A document that cannot be read or is not JSON is a failure naming the file, never an empty diff:
"nothing changed" and "could not tell" may not share an exit code in a gate. Put the command in CI
against the last release's document and a broken client turns from an incident into a red pipeline.
