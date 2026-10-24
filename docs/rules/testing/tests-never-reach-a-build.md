A `#[Test]` method may be declared in any file — beside the class it tests, or in a separate tree.
What makes a method a test is the attribute, never the path.

**Test code is a method that carries a testing marker:** `#[Test]`, `#[Fixture]`, `#[BeforeClass]`,
`#[AfterClass]` or `#[AfterEachTest]`. A data row is an attribute on a `#[Test]` method, and an
assertion message, a double and an anonymous function are written inside a test's body, so each of
them is test code with the method it belongs to. The class that declares them is not test code:
its constructor and its other methods are compiled as any class's are, because a class with a
`#[Test]` method may be the class under test as well.

`nvs test` compiles and runs test code. `nvs run`, the reload of a running program, and `nvs build`
do not lower it at all: no test method is compiled, and no class's method table names one. A built
artifact carries no test surface, and a run pays nothing for one. One program is therefore two
compiled units, and the artifact cache keys them apart, so neither command loads the other's.

**A call into test code from other code does not compile** (`E0889`). That covers a call, a method
reference and a class-reference call whose method resolves to test code, written anywhere but inside
test code. It is a compile error under every command, `nvs test` included, so a program compiles the
same way for a run and for its tests. Without it, a call into a test would check and then name a
function the build left out.

`nvs check` does type-check test code. That is the one place tests and non-tests are treated alike,
and deliberately so: a test cannot rot silently while the code around it changes.

**What it costs:** one set of test method labels per compile, O(test methods), and nothing at run
time.

**Declined:**

- *A test class is test code as a whole.* A class with a `#[Test]` method may be the class under
  test, so leaving the class out would leave production methods out of a build.
