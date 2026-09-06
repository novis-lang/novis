**Byte-completeness.** Every non-trivia byte of input leaves the converter as either converted code
or commented-out source. A construct with no rule is treated as tier N: commented, annotated and
counted. The test for this is mechanical and runs over the whole corpus.

**Comments and docblocks survive**, re-attached to the construct they documented. A converter that
loses a library's documentation has not ported it.

**A file the front end cannot parse becomes a fully commented-out file** carrying the parse error and
the dialect it was tried under — never a missing file and never a silent skip
(`rule:tooling/convert-php-front-end`).

**Output is re-parsed with `nvs-syntax` before it is written.** A rule that produces unparseable
Novis is a converter bug: the run reports it against the rule id, and that file falls back to fully
commented-out, so a bad rule can never leave a tree that does not parse.
