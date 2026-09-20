Returns a list of the whole numbers between two bounds. Both bounds are part of the list.

You give the first number and the last number. The list counts up when the first number is smaller,
and it counts down when the first number is larger. When both are the same, the list holds that one
number. The `step` option sets the distance between neighbours, and it is `1` by default. A step is
always a positive number, because the direction comes from the two bounds. A step that would pass
the last number stops at the last number inside the bounds. A step of `0` or less throws a
`RuntimeError`. It replaces PHP's `range`.

**The examples below** show a plain count, a count down and a count in steps of two, and the page
numbers under a list of search results.
