`#[Test(db: "test")]` opens a transaction against the named connection before the test and rolls it
back after. The test sees a pristine database and writes no cleanup code, and a transaction opened
*inside* the test becomes a savepoint, so code under test that manages its own transaction behaves
normally.

The transaction is opened **on the test's own context, from inside its isolate**, and that is what
makes the savepoint sentence true with no special case anywhere: a connection is memoized on the
context it was opened on, so the test's own `connect("test")` reaches *this* connection and its own
`transaction()` sees a non-zero nesting depth. Opened on the suite's context instead, the two would
contend rather than nest.

The rollback is armed around the whole retry allowance rather than around one attempt, because a
retry calls the method again.
