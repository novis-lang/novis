A `Core` member whose output scales with its input takes one of exactly two obligations: it allocates
that output through the value allocator the memory ceiling guards, or it bounds the input it accepts.
There is no third option, and a member that took neither is how a request holds many times its own
`[limits] memory` in one call.

The ceiling is asked in front of a value allocation, so a member that grows a Novis `string` or
`array` as it goes is bounded by construction: the first allocation past the ceiling is refused and
the request is stopped. What escapes it is a member that assembles its result in its own buffer and
hands the finished thing over at the end — the ceiling is then asked once, after the bytes were
already held, and the residual is the member's whole expansion ratio.

So a member with an expansion ratio bounds the input instead, and the two shipped instances are the
shape to copy. `rule:core-classes/regex-two-tiers` bounds a backtracking search by steps rather than
by the size of what it produces. `rule:core-classes/decompression-bound` bounds a decode by
`min(input × ratio, ceiling)`, applies it while the output grows rather than to a buffer already
allocated, and refuses rather than truncating.

A bound written this way is part of the member's contract: it is stated in the member's own rule, the
breach it raises is named there, and a caller may lower it and never raise it. A member that instead
takes the first branch says so — its result is built through the value allocator and it holds no
buffer of its own that the ceiling has not seen.
