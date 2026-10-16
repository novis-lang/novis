A debug dump shows a class's real declared properties and their real current values. There is no hook
to filter, rename or synthesize what appears: `__debugInfo` does not exist and cannot be declared, so
what you declare is what a dump shows. What the view looks like belongs to the diagnostic record
(`rule:errors/debug-dump`); the rule here is the absence of the hook.

The alternative would be a customization surface with no unsafe default to close — the dump is a
fixed, built-in operation a developer did not write and cannot call with attacker-influenced arguments
to bypass anything.

That is unrelated to reflection, which enforces the same visibility check an ordinary access would:
one is a built-in view, the other a call site a script constructs, and they have different threat
models. A `secret`-typed property is redacted wherever it is dumped, which is the qualifier's rule and
not an exception to this one.
