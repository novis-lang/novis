Compares this instant with another one and returns `-1`, `0` or `1`.

The result is `-1` when this instant is earlier, `0` when both are the same moment, and `1` when
this instant is later. An instant is one exact point in time, so the time zone it was written in
does not matter. `2024-03-01T13:00:00+01:00` and `2024-03-01T12:00:00Z` are the same moment.

You can also write `<`, `>`, `<=`, `>=` and `<=>` between two instants. They all use `compareTo`,
so they always give the same answer.

**Good to know:** `==` compares two instants by their moment too. It is `true` for the two texts
above.
