How a number that lands exactly between two neighbours is settled when you round it.

Four of the six rules only decide a tie — a value like `2.5`, where neither neighbour is closer.
`HalfUp` sends it away from zero, and is what you get when you say nothing. `HalfDown` sends it
toward zero. `HalfEven` sends it to the even neighbour, which is what a long column of money or
measurements usually wants, because it does not drift the way always-up does. `HalfOdd` sends it to
the odd neighbour. The other two ignore ties and move every value that has anything to discard: `Up`
away from zero, `Down` toward it.

**Good to know:** all six work at any precision, so they can settle the second decimal, not only the
whole number. They are about distance from zero rather than about the number line, so `Up` on a
negative value moves it further below zero. They replace PHP's four `PHP_ROUND_HALF_*` constants and
reach further than those do.
