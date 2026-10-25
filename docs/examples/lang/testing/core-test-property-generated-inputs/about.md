A property is a test that runs many times, with new inputs on every call. You say what must be
true for every input, and the runner looks for an input where it is false.

`#[Property]` goes on a method with the same shape as a `#[Test]` method. Each parameter gets a
generated value of its declared type. The runner calls the method 100 times, and `runs:` changes
that number. `gen: { level: "0..1000" }` gives `$level` a whole number from 0 to 1000.

When a call fails, the runner tries smaller inputs that still fail. The report shows the smallest
input it found, the input that failed first, and the seed. Write that seed as
`#[Property(seed: n)]`, and the test runs with the same inputs again.

**Good to know:** a generated string can have any character, including emoji and accents. A
property that only holds for English letters fails.
