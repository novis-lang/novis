A callback a `Core` member invokes always receives `($value, $key)`, in that order, and a callable may
declare fewer parameters than the call site passes (`rule:types/callable-arity`). A callable wanting only
the value writes one parameter and never sees the key.

This kills the whole `ARRAY_FILTER_USE_KEY`/`ARRAY_FILTER_USE_BOTH` flag family, which exists in PHP only
because its callbacks have a fixed arity, and it removes the need for `map`/`mapWithKey` pairs that would
otherwise violate `rule:core-api/one-name-one-signature`. The order is value-first because that is the
argument almost every callback uses, so the common callback is `fn($v)` with nothing to skip.
