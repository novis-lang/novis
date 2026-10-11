A `match` with no arms is `E0476` where it is written. PHP parses one and throws `UnhandledMatchError` on
every evaluation, so no program that ran is lost. A `match` is an expression, and one whose every path
throws has nothing for the position it sits in to bind, pass or return, and no value for a merge with
no incoming edge to carry. A written arm, or `default => throw …`, says the same thing on purpose.
