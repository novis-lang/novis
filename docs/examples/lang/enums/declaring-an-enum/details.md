A case value is a whole number written in the code. It may be negative, and it may be written in hex
(`0x10`), in binary (`0b11`) or with underscores (`1_000`). Anything else is the error `E0436`. That
covers a calculation such as `1 << 3`, another case, a class constant, a `float` and a string.

A value that does not fit the backing type is the error `E0437`. `enum M: uint { A = -1 }` is one
example. A case with no value after the largest possible number is `E0437` too, because no number is
left to count on to.

A comma after the last case is allowed. An enum with no cases compiles.

Two cases with the same value are equal: `Suit::A == Suit::B` is `true` when both are `1`.

The enum name and every case name are PascalCase. A name that starts with a lowercase letter or has
an underscore, such as `hearts` or `Big_One`, is the error `E0110`. The help gives the fixed name.

These forms do not compile:

- `case` before a case name is the error `E0239`.
- A constant, a method, or a `;` between cases is the error `E0220`.
- `implements` on an enum is the error `E0218`.
- `enum Suit: string` is the error `E0219`. Any other backing type except `int` and `uint` is
  `E0438`.
- An enum with the same name as another enum or a class in the file is the error `E0304`.
- An enum declared inside a method is the error `E0233`. Declare it at the top level of the file,
  or in a class body.

A case that the enum does not have, such as `Suit::Clubs`, is the error `E0309`.
