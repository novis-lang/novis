There is no `case` keyword in an enum body. `case Hearts = 1;` is refused with **`E0239`**, raised
on the `case` keyword itself and naming the comma-list spelling that works
(`rule:enums/declaration`).

The case is *kept*: only the keyword and its `;` are consumed, so an enum written with `case` still
declares every member the rest of the program goes on to name, and one diagnostic per case is the
whole answer rather than a cascade of unresolved names behind it.

This is deliberately not `E0220` (`rule:enums/no-class-machinery`). A method or a constant in an
enum body belongs somewhere else and is told so; a case belongs exactly where it is written, and
only its spelling is wrong.
