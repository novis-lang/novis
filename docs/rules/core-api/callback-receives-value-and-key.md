A callback a `Core` member invokes always receives `($value, $key)`, in that order, and a callable may
declare fewer parameters than the call site passes (`rule:types/callable-arity`). A callable wanting only
the value writes one parameter and never sees the key.

No member takes a flag saying whether its callback wants the value, the key or both, and no member needs
a `map`/`mapWithKey` pair that would otherwise violate `rule:core-api/one-name-one-signature`. The order is value-first because that is the
argument almost every callback uses, so the common callback is `fn($v)` with nothing to skip.
