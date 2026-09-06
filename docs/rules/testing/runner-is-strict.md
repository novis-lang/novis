A test that asserts nothing fails — the ledger already knows the count, and
`Core\Test::assertDoesNotThrow(callable)` is how a test states "this must merely not throw",
explicitly.

A skip states a reason: `skip: true` is a compile error and a written sentence is not. A retry
states one too — `retries:` with no `because:` beside it is refused where it is written — and a test
that passed on a later attempt is its own verdict, `flaky`, never green: its own mark, its own count
in every summary, and the element CI systems already read as "passed, but it flaked". It carries the
**last failed** attempt's message, the attempt that passed having produced nothing to report. Only a
failure is retried: a skip never ran, and an `exit(n)` ended the program rather than the test. A
flaky test does not fail the run — what this takes away is the silence, not the green build.

Report order is **declaration order**, even though execution is unordered. Total isolation makes
execution order semantically irrelevant, so nothing is bought by randomizing it and stable output is
worth a great deal.
