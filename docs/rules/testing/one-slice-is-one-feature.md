One slice takes one feature and writes **all five of its artefacts together**. The expensive thing a
session buys is understanding what the feature does at its edges, and the description, the test, the
examples, the bench and the attack all spend that same understanding; split across five sessions it
is bought five times, against a fixed per-session cost that does not shrink with the size of the work.

The generated work chain is one goal per group of features sharing an implementing file set, each
carrying a context manifest naming that file set and each gated by a command that exits non-zero.
Regenerating the chain is how it stays current: a group that owes nothing is left out, so a second
emission writes the chain that is *left*.
