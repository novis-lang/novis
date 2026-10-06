`f(?, $x)` and every application-with-holes shape do not parse. The value it would produce is
exactly what the callable machinery reduced to one shape (`rule:types/callable-values`), and
an anonymous function already spells every partial application: `fn($a) => f($a, $x)`
(`rule:types/anonymous-function`). A second callable-producing spelling, for no capability `fn` lacks,
is what was rejected.

A PHP 8.6 `?` placeholder is written as that wrapper, each `?` a fresh parameter, in order. The
pipeline operator is unaffected: its `$_` is a parse-time substitution, not an application
(`rule:expressions/pipeline-substitution`), and the hole diagnostic
(`rule:expressions/pipeline-hole-once`) already teaches the difference.
