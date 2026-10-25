A bench method is `public`, it is not `static`, and it returns `void`. If an assertion inside a
bench fails, the bench fails. `#[Bench(skip: "reason")]` skips a bench and prints the reason.

The counts tell you if your code does more work. They do not tell you if Novis got faster.

Each bench runs once. Its counts are the same on every run, so a second run gives no new
information.

Each bench runs on its own, the same way each test does. A value that one bench puts in a `static`
property is not there when the next bench runs. So the counts of a bench do not depend on the
benches that ran before it.

A bench can have a `#[Fixture]` parameter. The fixture is built before the counting starts, so its
work is not part of the counts.

A bench cannot fix the clock or the random numbers. A bench that reads `Core\Time::now()` or calls
`Core\Random` can do different work on each run, and then its counts change too.

`#[Bench]` has two options, `skip:` and `because:`. Both are strings. Every other option of
`#[Test]`, such as `retries:`, `seed:` or `at:`, does not compile on a bench. A `#[TestWith]` row on
a bench does not compile either. A method that has `#[Bench]` and also `#[Test]` or `#[Fixture]`
does not compile.

`--filter` works with `--bench` and selects benches by `Class::method`. `--bench` does not work with
`--jobs` or `--mutate`, and it does not work with a `.nvst` file.

In the JSON report, a bench that ran has `counts` with `statements`, `calls`, `allocations`,
`bytes` and `gc`. It also has `advisoryWallMs`, the time it took in milliseconds. A skipped bench
has `reason` and no `counts`.

The exit status is `1` when a bench failed, and `0` otherwise.
