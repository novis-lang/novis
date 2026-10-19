`Outcome::attempt()` returns which run of the test this was. The first run is `1`.

A test with `retries:` in its `#[Test]` attribute runs again when it fails. Each new run adds one, so
the second run is `2`. A test without `retries:` always has `1`.

You get an `Outcome` as the parameter of a method with `#[AfterEachTest]`. The method runs after
every run, also after a run that failed. You cannot create an `Outcome` yourself.

**Good to know:** a file you name after the test is written again on each run. Put the attempt number
in the name to keep the file of every run.

**The examples below** show a log file for each run, a warning for a test that needed a retry, and a
CSV file with one row for each run.
