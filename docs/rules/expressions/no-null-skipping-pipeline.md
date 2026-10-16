There is no `?|>`. A pipeline step that returns `?T` hands its `null` to the next step like any other
value, so a chain through a nullable result is split with an `if` where the `null` is handled.

A second operator needs a second reason: evidence that a `?T` in the middle of a composition breaks
chains often enough to cost more than the `if` does.
