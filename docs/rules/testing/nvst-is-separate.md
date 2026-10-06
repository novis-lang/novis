`.nvst` is a whole-program, expected-stdout conformance case, and it is how **Novis's own conformance
to its specification** is proven. A case's expectation is what the rules say Novis prints.

`#[Test]` is how **a program written in Novis** tests itself.

The two formats answer different questions and are not unified, now or later. `nvs test` runs
both — a path of `.nvst` files, or a program's compiled test table — and reports each in the shape
that fits it.
