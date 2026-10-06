Adds the next piece of data to a digest that is still open.

Each call adds its data after the data from the calls before it. The order matters: the same pieces
in a different order give a different digest. How you cut the data does not matter, and an empty
piece changes nothing. The data can be a `string` or `bytes`.

**Good to know:** the stream keeps each piece until you call `finish`. If you pass the same text
many times, it is kept only once. Calling `update` after `finish` throws a `RuntimeError`.
