A `Core` member signals failure by throwing. `false` is never returned to mean failure, no member returns
an error code, and there are no error globals — no `json_last_error`, no `error_get_last`. A `?T` return
means something else entirely: that the absence is an ordinary, expected outcome the caller should handle,
not that something went wrong.

A search that returns either a position or `false` makes position `0` and failure easy to confuse, and the
union types that make it expressible here also make it unnecessary: the two outcomes are already two different things in the type,
so collapsing them into one return value buys nothing. A member's verb tells the caller which of the two it
is (`rule:core-api/verb-lexicon`) before the signature is read.

The cost is that a converted program's error handling has to be rewritten rather than translated: a
`if ($r === false)` has no mechanical equivalent, because the information it tested for now arrives as a
throw the caller must decide where to catch.
