In PHP a function declared `: int` whose body reaches its own end returns `null`, whatever it
declared. Here that path writes no value at all, and there is no implicit conversion by which `null`
becomes the declared type, so accepting it would change what a binding holds behind its declaration —
the one thing the type system never does. The body is refused where it is written (`E0739`).

`void`, and a declaration that writes no return type at all (a constructor), promise nothing and are
untouched; so is a generator, whose body `rule:iteration/generators` leaves no return value to
produce. The analysis is asymmetric on purpose, and `nvs_types::returns` owns it: a `while (true)`
with no `break`, a `switch` with a `default`, a `try` every path of which exits, and a body that
always throws are all exits, and every shape it cannot prove reaches the end is treated as one — so
the refusal costs no program that ran.
