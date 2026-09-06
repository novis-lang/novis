An argument bound to an `inout` parameter carries `inout` at the call, and an argument bound to a by-value
parameter must not:

```nvs
int $n = 5;
echo Adder::bump(inout $n), " ", $n, "\n";   // "6 6"
```

This is not inference-assisted and it is not optional. A marker the compiler fills in silently is the
invisible mutation the rule exists to remove; it would make the requirement a style rule rather than a
rule, and a program that omitted it everywhere would still compile and still hide the write. Two mistakes,
two diagnostics: `E0713` is an argument that binds an `inout` parameter without saying so, `E0714` one that
says so where nothing binds it — including a call through a `callable`.

`foreach` and destructuring have no call site, so for them the declaration spelling
(`rule:statements/inout-is-the-by-reference-spelling`) is the whole of it.
