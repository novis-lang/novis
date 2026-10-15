`#[AfterEachTest]` marks a `public` instance method returning `void`, and the runner calls it on the
same object the test ran on, inside that test's isolate, after every attempt, whether the attempt
passed or failed. It exists for residue that outlives the isolate — a file, a row, a remote object.
Everything else needs no teardown: the isolate dies and takes everything inside it with it.

The hook knows which test ran because it runs on that test's own `$this`: whatever the constructor and
the test stored there is what it cleans up. It may also declare one parameter of type
`Core\Test\Outcome`, filled by type the way a fixture fills a test's parameter:

| Field | Value |
|---|---|
| `class` | the test class's name |
| `name` | the method's name, with `#N` for a `#[TestWith]` row (`rule:testing/data-rows`) |
| `attempt` | `1` for the first run, one more for each retry (`rule:testing/runner-is-strict`) |
| `passed` | whether this attempt passed |
| `failures` | the messages the report prints for this attempt, empty when it passed |

`Outcome` is for deciding what to keep and what to report — the evidence of a failed test kept under
its own name, the rest deleted. Cleanup that only one test needs belongs in that test's own `finally`,
not in a hook that branches on `$outcome->name`. Any other parameter, or a second one, is refused where
it is written.

A retry is a fresh instance in the same isolate, so each attempt gets its own call on its own object.
A skipped test is never constructed, so the hook does not run for it. A constructor that throws leaves no object, so the hook does not run and the test fails with the
constructor's error. A hook that throws fails that attempt, and its failure is reported with the
test's own.

There is no `#[BeforeEachTest]`: the constructor is that hook (`rule:testing/constructor-is-setup`).

A test runs with the program's own grants (`crates/nvs-cli/src/runner.rs:384`), so the hook can remove
whatever the test was allowed to create.

**Not on disk.** `#[AfterEachTest]` compiles and is checked (`crates/nvs-types/src/test_hooks.rs:114`),
but the runner does not call it (`crates/nvs-cli/src/runner.rs:785`), and `Core\Test\Outcome` does not
exist.

- **Memory:** one `Outcome` per attempt, a few strings and one list of failure messages, released when
  the hook returns. A class without the hook pays nothing.

**Declined:**
- **`#[After]`.** It does not say after what, beside `#[BeforeClass]` and `#[AfterClass]`
  (`rule:testing/class-hooks`).
- **A static call such as `Test::current()`.** It would return a different value depending on runner
  state the method's signature does not show. A parameter is visible, optional and checked while
  compiling.
- **A `Throwable` in place of `failures`.** A test can fail several assertions and throw nothing, and
  the ledger already records each one as a message.
