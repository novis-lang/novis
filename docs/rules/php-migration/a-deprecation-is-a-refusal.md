Novis compiles, and it has no deprecation channel. A change PHP must phase in over a major version —
announced in 8.x, warned about at run time, removed in 9 — is here a refusal at the line it is
written, the posture already taken for legacy cast syntax (`rule:types/no-legacy-cast`), the keyword
operators (`rule:expressions/no-keyword-logical-operators`) and `list()`
(`rule:expressions/bracket-destructuring`). Parity is owed to programs PHP 9 will still run, not to
spellings PHP itself is removing, so a refusal of this kind never produces a program PHP runs and
Novis answers differently: nothing diverges at run time, because nothing reaches it. Each is a
front-end check with zero run-time cost and no run-time state. Building a warning channel to permit,
temporarily, code that can be refused statically at the exact line is the alternative that lost.

PHP 8.6 lands as four of these and one non-adoption: no `return` leaves a `finally`
(`rule:php-migration/no-return-leaves-a-finally`), a constructor's `return` carries no value
(`rule:php-migration/a-constructor-return-carries-no-value`), `let` and `is` are reserved
(`rule:php-migration/let-and-is-are-reserved`), a `readonly` property declares no default
(`rule:php-migration/a-readonly-property-declares-no-default`), and partial application is not
adopted (`rule:php-migration/no-partial-application`); beside them, one session rule with no off
switch (`rule:php-migration/a-session-id-the-store-did-not-issue-is-rejected`). The rest of 8.6 asks
nothing: `clamp` is `Core\Math::clamp`, `Time\Duration` is `Core\Time\Duration`, written
like `1h30m` (`rule:types/duration`), `Io\Poll` has no landing spot because readiness is
runtime-internal and user code gets structured concurrency, and nearly every other deprecation names
surface Novis never shipped.
