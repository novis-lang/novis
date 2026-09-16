In PHP a function declared `: int` whose body reaches its own end returns `null`, whatever it
declared. Here that path writes no value at all, and there is no implicit conversion by which `null`
becomes the declared type, so accepting it would change what a binding holds behind its declaration —
the one thing the type system never does. The body is refused where it is written (`E0739`).

A written `return;` is the same promise broken at the other exit, and is refused the same way
(`E0822`); PHP raises a `TypeError` there rather than answering `null`. Both are read over every
block body checked against a declared type — a method's, a `get` hook's and a block-bodied `fn`'s
alike.

`void`, and a declaration that writes no return type at all (a constructor), promise nothing and are
untouched; so is a generator, whose body `rule:iteration/generators` leaves no return value to
produce. The analysis is asymmetric on purpose, and `nvs_types::returns` owns it: a `while (true)`
with no `break`, a `switch` with a `default`, a `try` every path of which exits, and a body that
always throws are all exits, and every shape it cannot prove reaches the end is treated as one — so
the refusal costs no program that ran. Whether a written `return;` is legal asks none of that: the
declared type is the whole answer, which is why a `never` body is refused one and is still never
asked about the falling-off path.
