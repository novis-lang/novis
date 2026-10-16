`ArrayAccess` and `Countable` do not exist, and no other interface makes a syntactic form dispatch to
a method. `$obj[$k]` on anything that is not an array does not compile.

A collection exposes ordinary members instead: `->get($k)`, `->set($k, $v)` and `->count()`. Each of
those buys notation at the cost of a call that does not look like one, which is the same implicit
dispatch already rejected for `$obj->prop`; and reading as an array while none of `Core\Arr` applies
is a second cost with no offsetting capability. Iteration is the one place a capability is bought —
streaming a cursor without materialising it — and it has its own interfaces
(`rule:iteration/two-interfaces`).

`Countable` would have nothing to hook: it exists to make a global `count($x)` dispatch to a method,
and there are no free functions (`rule:classes/no-free-functions-or-constants`). Its equivalent would
be an array function dispatching on an object argument, which is worse than the `->count()` it
replaces.

**Declined:**
- **Keeping `ArrayAccess`.** It reads familiarly, but implicit property dispatch is already gone, and
  keeping implicit subscript dispatch would leave the language inconsistent about the same question.
  It would also make whether `$obj[$k]` gives a `tainted` value depend on an implementation the call
  site cannot see (`rule:security/tainted-qualifier`).