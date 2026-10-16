`#[BeforeClass]` and `#[AfterClass]` mark `public static` methods returning `void` that run once
around a test class's tests, in the parent, and `#[AfterClass]` runs whenever `#[BeforeClass]` started.
They are for setup whose effect lives outside every isolate — a database created, its schema built and
its rows seeded once for the class, then dropped after its last test.

The order for one class is `#[BeforeClass]`, then the fixtures its selected tests need
(`rule:testing/fixtures`), then the tests, then `#[AfterClass]`. A fixture therefore reads what the setup
wrote. Neither hook runs for a class none of whose tests is selected, for the reason a fixture is not
built for one (`crates/nvs-cli/src/runner.rs:890`).

| What happens | Result |
|---|---|
| a test fails | `#[AfterClass]` still runs |
| `#[BeforeClass]` throws | no test runs, each selected test is reported failed with that error, and `#[AfterClass]` runs, because a setup that stopped halfway left something to remove |
| `#[AfterClass]` throws | the run fails, and the failure is reported against the class rather than a test |
| a test calls `exit()` or the process is killed | nothing after it runs (`rule:errors/escalation-ladder`) |

`#[BeforeClass]` takes no parameter. `#[AfterClass]` may declare one of type `Core\Test\Summary`, filled
by type. It is a `Core` instance, so it is read through methods: `class()`, the counts `passed()`,
`failed()`, `flaky()` and `skipped()`, and `failedTests()`, the names of the failed tests as the report
prints them. A hook that keeps the database when `failed()` is above zero and drops it otherwise is the
case it exists for.

This order is the whole of what the runner guarantees. Classes run at the same time
(`rule:testing/tests-run-across-every-core`), so two classes whose setups touch one database keep
apart themselves, for instance by naming the database after the class.

Tests of one class share what the setup wrote, so a test that writes to it runs under
`#[Test(db:)]` (`rule:testing/db-transaction`). Without that, one test's rows reach the next and the
order of the tests matters again, which is what an isolate per test exists to prevent
(`rule:testing/isolate-per-test`).

A hook that writes a static property in its own body is refused where it is written. Every test runs
in its own isolate and reads the static's declared initial value, so the write would look like setup
and set up nothing.

The fixtures are released before `#[AfterClass]` runs, so the hook can remove what a fixture held open.
A failed `#[AfterClass]` is printed under its class; a machine format writes it to stderr, and the run
exits with a failure either way.

- **Memory:** one `Summary` per class, four integers and the names of its failed tests, held in the
  parent from the class's first test to the return of `#[AfterClass]`. A class without the hooks pays
  nothing.

**Declined:**
- **A database setup written as a fixture.** A fixture is built only when a test declares its type, so
  a setup that returns nothing would never run, and a fixture has no cleanup.
- **A `#[BeforeClass]` that returns a value.** A value the tests receive is a fixture.
- **`#[BeforeAllTests]` and `#[AfterAllTests]`.** In a run of many classes, "all tests" reads as the
  whole run. `Class` names the scope.
