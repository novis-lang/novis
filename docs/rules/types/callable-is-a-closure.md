`callable` is satisfied by exactly one shape of value: a closure. That means a `fn` literal, or a
first-class-callable-syntax reference — `Core\Str::length(...)`, `$user->getName(...)`,
`self::helper(...)` (early-bound), `static::helper(...)` (late-bound). Each of those names a member,
and the checker records the resolved target.

PHP's other three spellings are refused where they are written, each with a diagnostic naming the
replacement: a bare name string (`'strlen'`), a `"Class::method"` string, and a `[$obj, 'method']`
array. Two further refusals fall out of the same rule: `new C(...)` is `E0740`, because `new` names a
class rather than a callee, and `$m->method(...)` on a `mixed` receiver is `E0732`, because a closure
value outlives the site and there is no class present to read a callee off
(`rule:types/erased-member-access`).

**There is no `__invoke`.** `$obj(...)` is a diagnostic whenever `$obj` is not itself a closure,
whatever methods the object's class declares; a method literally named `__invoke` parses as an
ordinary method with no special meaning. `()` therefore stays one thing rather than a second operator
any class can opt into.

A callee the checker cannot prove is a closure — one typed `mixed`, or a `?callable` that holds
`null` — is checked when the call runs. A value that is not a closure throws a catchable `LogicError`
naming what it is, the class the same call throws for an argument of the wrong type.

The rule is the same at every position typed `callable` — a parameter, a property, a return type, a
stdlib signature: the argument must already be a closure by the time it arrives, never a string, an
array, or an object the checker would have to interpret.
