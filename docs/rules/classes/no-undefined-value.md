There is no `undefined` type and no `undefined` value. Whether a property may legitimately hold no
value is a fact about its value space, decided once at its declaration by writing `?T`; whether it has
been assigned yet is a different axis entirely, and conflating the two is what makes a universal
`undefined` look attractive.

Adding one would make every declared property implicitly `T|undefined`, which is precisely the "a
declared type silently holds something else" failure `rule:types/declaration` exists to close,
recurring one binding kind later. JavaScript's version is safe only because JS has no declared
property type to violate.

Per-type silent defaults are rejected for the same reason: a plausible-looking zero is worse than a
loud failure, because "forgot to initialize" then looks identical to "legitimately zero". What
remains for the case static analysis cannot reach is a throw, never a value
(`rule:classes/an-unwritten-property-read-throws`).
