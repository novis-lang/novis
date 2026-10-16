A hook is a method that the test runner calls around your tests. You use one to clean up something a
test leaves outside itself, such as a file or a database table.

- `#[BeforeClass]` runs once, before the tests of the class. It is a `public static` method.
- `#[AfterClass]` runs once, after the last test. It is a `public static` method. It can take a
  `Core\Test\Summary`, which has the counts of passed and failed tests.
- `#[AfterEachTest]` runs after each test, on the same object as the test. It can take a
  `Core\Test\Outcome`, which has the name of the test and whether it passed.

The after hooks also run when a test fails. `#[AfterClass]` also runs when `#[BeforeClass]` throws
an error.

To prepare each test, use the constructor. To share a value with the tests, use a fixture.

**Good to know:** a class has at most one hook of each kind. A subclass inherits the hooks of its
parent class.
