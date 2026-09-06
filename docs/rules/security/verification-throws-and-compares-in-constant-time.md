Verification returns claims or throws. It never returns a falsy value that a loose comparison could
mistake for success, which is the same reasoning a budget exhaustion gets: a result that can be
misread as a result is worse than an error.

**Every comparison of a secret-derived value is constant-time, and no member exposes the raw value for
the caller to compare themselves.** Where a token is compared, the comparison *is* the exposed
operation, so a caller cannot write `==` over it by accident — and where a program compares its own
credentials, the operator is right by construction anyway
(`rule:security/secret-comparison-is-constant-time`).

A refusal says one sentence and the same sentence for every forgery, so the failure carries no oracle
about which part of the token was wrong.
