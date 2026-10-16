`.nvst` is a whole-program, expected-stdout conformance case, and it is how **Novis's own conformance
to its specification** is proven. A case's expectation is what the rules say Novis prints. No test
and no check runs another language's engine to judge Novis, for this repository or for a user: the
rules and the conformance cases are the one reference.

`#[Test]` is how **a program written in Novis** tests itself.

The two formats answer different questions and are not unified, now or later. `nvs test` runs
both — a path of `.nvst` files, or a program's compiled test table — and reports each in the shape
that fits it.

**Declined:**
- **A PHP oracle**: an `--ORACLE--` section run under PHP, an `nvs test --php` mode, or a differential
  tree kept as an unrun archive. It is a second reference that disagrees with the rules wherever they
  diverge, and a reader cannot tell which one is normative.
