`#[Fixture]` marks a `public static` method, and what it supplies is its declared return type. It is
built **once per class**, in the parent, and reaches a test by that test declaring a parameter of
the type it returns. Resolution is by interned type and happens while compiling, so an unsatisfiable
parameter is a diagnostic naming the type it asked for rather than a null at run time. A fixture may
itself declare fixture parameters; a cycle is a compile error naming the whole chain.

Two fixtures of one class returning one type is a duplicate declaration, because resolution is by
type and two answers to one parameter is exactly the ambiguity that refusal prevents. Four
declarations cannot supply a value and are refused where they are written: one that is not `static`,
one that is not `public`, one returning `void`, and a method carrying both this marker and `#[Test]`.

Only a fixture that a test which will actually run asks for is built — a skipped test's fixture is
setup nobody wanted. Resolution runs once the whole class is collected rather than as the walk
descends, because a test may be written above the fixture that supplies it.

- **Memory:** a class's built fixtures are held from before its first test to after its last, and
  each test that declares one holds its own copy of it, made with the one graph copy `spawn` uses,
  for that test's life. A suite whose copies come to dominate its cost is answered by a copy-on-write
  path for immutable fixtures, measured first, never by sharing an isolate between tests.
