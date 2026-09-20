A fixture is a value your tests share. It is built once, and every test that needs it is called with
it.

You write a `public static` method and put `#[Fixture]` above it. The type that method returns is
what the fixture supplies. Every test in the same class that declares a parameter of that type is
called with the value. The name of the parameter does not matter, only its type. A fixture may
declare a parameter as well, so one fixture can be built from another.

The method runs once for the whole class, before the tests. Each test then works on its own copy, so
a change one test makes is not visible to another. Setup that costs time, such as reading a file or
building a large object, happens one time.

**Good to know:** a fixture belongs to its own class, and a test cannot use another class's fixture.
A parameter that no fixture supplies does not compile.
