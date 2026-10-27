`callable` is satisfied by exactly two kinds of value: an anonymous function (`fn (...) => ...`) and a
method reference — `Core\Str::length`, `$user->getName(...)`, `self::helper` (early-bound),
`static::helper` (late-bound), one form per receiver (`rule:expressions/method-reference`). A method
reference names a member, and the checker records the resolved target. Both make the same kind of
value, a callable.

Three other ways to name a function are refused where they are written, each with a diagnostic naming the
replacement: a bare name string (`'strlen'`), a `"Class::method"` string, and a `[$obj, 'method']`
array. Two further refusals fall out of the same rule: `new C(...)` is `E0740`, because `new` names a
class rather than a callee, and `$m->method(...)` on a `mixed` receiver is `E0732`, because a callable
outlives the site and there is no class present to read a callee off
(`rule:types/erased-member-access`).

**There is no `__invoke`.** `$obj(...)` is a diagnostic whenever `$obj` is not itself a callable,
whatever methods the object's class declares, and a method named `__invoke` does not compile, like
every leading-underscore name (`rule:classes/no-leading-underscore-identifiers`). `()` therefore
stays one thing rather than a second operator any class can opt into. `$obj(...)` names no method at
the call site, which is the hidden dispatch `rule:classes/no-call-magic` closes, and a class that
could opt into `()` would be a second operator-overloading mechanism beside the one narrow,
interface-gated one (`rule:classes/comparable`).

A callee the checker cannot prove is a callable — one typed `mixed`, or a `?callable` that holds
`null` — is checked when the call runs. A value that is not a callable throws a catchable `LogicError`
naming what it is, the class the same call throws for an argument of the wrong type.

The rule is the same at every position typed `callable` — a parameter, a property, a return type, a
stdlib signature: the argument must already be a callable by the time it arrives, never a string, an
array, or an object the checker would have to interpret.

A string or array callable resolves by name at the call site, against whatever happens to be declared
there, so neither a reader, the checker nor an editor can follow it without reimplementing PHP's
lookup. A callable made by an anonymous function or a method reference always leads back to the place
that made it, so "go to definition" on any callable value lands on real code.

**Declined:**
- **Keeping all five of PHP's callable shapes**, an object with `__invoke` and the three by-name
  spellings beside the anonymous function. That keeps the by-name path next to the syntax that
  already replaces it.
- **Keeping `__invoke` as the one exception.** It is the hidden dispatch and the second
  operator-overloading mechanism above.
- **A dedicated `MethodRef` or `FunctionRef` type.** A second type with no behaviour of its own adds
  surface and nothing else (`rule:types/callable-is-the-only-function-type`).
