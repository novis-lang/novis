A closure that needs to call itself may carry an optional name between `fn` and its parameter list:

```php
$fact = fn factorial($n) => $n <= 1 ? 1 : $n * factorial($n - 1);
```

`factorial` is visible **only inside that closure's own body**. It is not a capture — it is not among
the outer variables the body reads — not a second declared name reachable from anywhere else, and not
a runtime slot: it resolves the way a method resolves `self::`, entirely at compile time, with no cost
at literals that do not use it. It composes with both body shapes and is not a third closure form.

It does not reopen "every callable is a declared class member": that rule bars a free, globally
callable function existing outside a class, and this name is unreachable from anywhere but its own
body — the same status as a parameter name. A recursive helper that *is* reusable elsewhere still
belongs on a class as a named method; the self-name covers only the case where the sole reason a
closure would need a name is to call itself.
