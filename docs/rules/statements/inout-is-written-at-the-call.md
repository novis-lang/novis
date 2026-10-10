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

Without the marker, `Adder::bump($n)` and `Adder::sum($a, $b)` look the same at the call, and only one of
them writes its argument. Somebody reviewing code they did not write has to see that write where it
happens, so an invisible mutation is a security question and not a matter of taste. The price is typing:
`bump(inout $n)` is longer than `bump($n)`, and it says what `bump($n)` did not.

**Declined:**
- **`&` at the call site, beside `&` at the declaration.** `f(&$x)` reads as an address-of or a bitwise
  AND, and `&` keeps every one of its meanings (`rule:statements/ampersand-is-not-a-by-reference-marker`).
  PHP removed call-time `&` for exactly that confusion.
- **The declaration renamed, with no marker at the call.** It spends a breaking change on the name and
  leaves the mutation invisible, which is the larger of the two problems.
- **The marker inferred at the call, with a warning where it is missing.** That is the compiler filling it
  in, which the paragraph above already rules out.
