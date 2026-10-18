Novis has one true-or-false type and four number types.

- `bool` is `true` or `false`. You cannot use it in arithmetic.
- `int` is a whole number that can be negative.
- `uint` is a whole number that cannot be negative. It can count twice as high as an `int`.
- `float` is a fast number with a fraction. It is not always exact, so use it for a weight or a
  ratio.
- `decimal` is an exact number with up to 28 decimal places. Use it for money.

A number written in the code gets its type from where you put it. `float $weight = 3;` makes `3` a
`float`, and `decimal $price = 1.10;` makes `1.10` a `decimal`.

If a whole number gets too big or too small for its type, the program throws an `ArithmeticError`.
`int` and `uint` cannot be used in one sum. `decimal` and `float` cannot be used in one sum either.
Convert one side with `as`.

**The examples below** show whole numbers that are too big, money and a measurement, and a parcel
priced at a shop counter. The last one adds a `decimal` and a `float`, so it does not compile.
