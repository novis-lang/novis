Nothing Novis rejects may be coloured as though it were valid. `===` and `!==` are not operators
(`rule:expressions/one-equality-operator`), `(int)$x` is not a cast (`rule:types/no-legacy-cast`), `|>` is
not a call of a callable on its right (`rule:expressions/pipeline-substitution`), and the alternative colon syntax
(`if (...): ... endif;`) is not syntax at all.

A grammar that colours these confirms a mistake in the editor before the server contradicts it, which is
worse than no colour. `editors/vscode/test/grammar/rejected.test.ts` asserts each of these receives no
scope that reads as valid, and the grammar suite beside it holds the positive cases — a `#[Route]`
attribute that is not a comment, a nowdoc that does not interpolate, inline HTML outside `<?nvs`.
