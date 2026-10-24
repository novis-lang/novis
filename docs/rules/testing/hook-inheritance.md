A test class has at most one `#[BeforeClass]`, one `#[AfterClass]` and one `#[AfterEachTest]`, and a
subclass inherits each of them, each `#[Fixture]`, each `#[Test]` and each `#[Property]` the way it
inherits any method. A base class's hook is therefore a default: a subclass that declares nothing runs
it, around its own tests.

| The subclass | Result |
|---|---|
| declares nothing | the inherited hook runs for the subclass |
| overrides the method by name | the override is the hook, and calls `parent::` to keep the base's steps |
| overrides with an empty body | the hook does nothing for that subclass |
| declares a second hook of that kind under another name | refused, naming both methods: two hooks of one kind on one class |
| overrides without repeating the attribute | refused, naming the method it overrides |

The override repeats the attribute so that a reader of the subclass alone sees that the method runs
around its tests. "Once per class" means once per test class: two subclasses of one base each run the
inherited `#[BeforeClass]` once, around their own tests, and an abstract base with no tests runs
nothing.

Fixtures, tests and properties follow the same table. A subclass's test receives an inherited fixture
by its type, an override by name replaces it, and a second fixture of the same type under another name
is the duplicate `rule:testing/fixtures` already refuses. Each subclass runs every `#[Test]` and
`#[Property]` it inherits, around its own hooks and with its own fixtures, and reports it under its own
name, so `ShopTest` running a test written in `DatabaseTest` reports `ShopTest::itConnects`. One
contract test written once in a base class then runs against every implementation. A base class that is
not abstract is a test class of its own as well, and runs its tests once for itself.

**Declined:**
- **Several hooks of one kind, run in declaration order.** The order would be a rule every reader must
  know and no line of the code shows. A failure in the second of two setups would also need each setup
  paired with its own cleanup. Several steps go in the one method, in the order they are written.
- **Only the tests a class declares itself.** A base class could then share setup but not the tests
  that check it, and one contract test would have to be copied into every implementation's class.
