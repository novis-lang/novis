`.nvst` is a whole-program, expected-stdout conformance case — a deliberate superset of PHP's
`.phpt`, so importing PHP's corpus stays mechanical — and it is how **Novis's own conformance to its
specification** is proven. No case runs PHP: the expectation is what the rules say Novis prints.

`#[Test]` is how **a program written in Novis** tests itself.

The two formats answer different questions and are not unified, now or later. `nvs test` runs
both — a path of `.nvst` files, or a program's compiled test table — and reports each in the shape
that fits it.
