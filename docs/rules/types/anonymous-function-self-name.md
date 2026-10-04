An anonymous function that needs to call itself may carry an optional name between `fn` and its
parameter list:

```php
$fact = fn factorial($n) => $n <= 1 ? 1 : $n * factorial($n - 1);
```

`factorial` is visible **only inside that anonymous function's own body**. It is not a capture — it
is not among the outer variables the body reads — not a second declared name reachable from anywhere
else, and not a runtime slot: it resolves the way a method resolves `self::`, entirely at compile
time, with no cost at anonymous functions that do not use it. It composes with both body shapes and is not a
third anonymous-function form.

**A call written through that name is checked against the anonymous function's own signature**, the
way `rule:types/callable-signature` checks a call through a written `callable(int): string`: each
argument is held to the parameter it fills, the count is exact, and the call answers the declared
return type rather than `mixed`. There is no value to be opaque here — the anonymous function being
checked is the one right there — so the per-argument tag check a call through bare `callable` pays is not what a
recursive call is finally held to. An anonymous function that declares no return type is checking its body to
find out, and a self-call there answers `mixed`; its parameter list has no such half-measure, being
complete before the body is entered.

It does not reopen "every callable is a declared class member": that rule bars a free, globally
callable function existing outside a class, and this name is unreachable from anywhere but its own
body — the same status as a parameter name. A recursive helper that *is* reusable elsewhere still
belongs on a class as a named method; the self-name covers only the case where the sole reason an
anonymous function would need a name is to call itself.
