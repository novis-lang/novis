Compares this duration with another one and returns `-1`, `0` or `1`.

The result is `-1` when this duration is shorter, `0` when both have the same length, and `1` when
this duration is longer. A negative duration is shorter than zero and shorter than every positive
one.

You can also write `<`, `>`, `<=`, `>=` and `<=>` between two durations. They all use `compareTo`,
so they always give the same answer.

**Good to know:** `==` between two durations checks whether they are the same object. It does not
compare their length, so `90m == 1h30m` is `false`. To test for the same length, check that
`compareTo` returns `0`.
