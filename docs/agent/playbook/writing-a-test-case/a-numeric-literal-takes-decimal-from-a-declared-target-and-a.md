- **A numeric literal takes `decimal` from a declared target, and a ternary is not one.**
  `decimal $postage = $heavy ? 4.90 : 2.50;` is `E0401: expected decimal, found float`, because the
  arms are checked with no expectation to place them in and `float` is what a bare fraction is.
  Declare the binding with one literal and assign the other in an `if`, or write `as decimal` on
  each arm. [until: reviewed 2026-09-19]
