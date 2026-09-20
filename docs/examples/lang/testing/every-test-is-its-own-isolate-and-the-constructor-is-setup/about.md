Every test runs on its own, and shares no stored value with any other test.

A `static` value that one test writes is back at its declared starting value in the next test. The
test class is built again for every test, so a property one test changes starts fresh in the next
one. The constructor of a test class is where you prepare what a test needs. It runs once before
every test body.

A test class constructor takes no parameters. One that takes a parameter is reported as that test
failing. Values that a test method itself needs come from a fixture or from a data row.

**In plain words:** every test gets a clean desk. Nothing another test left on it is still there.

**The examples below** show a `static` that starts fresh in each test, a constructor that prepares
the same starting point for every test, and two tests that register the same address.
