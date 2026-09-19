Changing a value into another type is written one way in Novis: `as`, followed by the type you want.

A conversion either hands you a value of that type or it stops the program where it was written. It
never rounds, never drops a fraction, and never quietly substitutes a zero or an empty string because
the answer would not fit. Text becomes a number only when the whole of it spells one, and rounding is
something you ask for by name. Writing a question mark in front of the type asks the same question and
answers with nothing at all where the plain form would have stopped — which is allowed only where the
plain form really can fail.

**Good to know:** `as` takes the smallest piece of the line next to it, so a sum written around one is
not swept into it, and conversions can be written one after another.

**The examples below** read numbers out of text that came from outside the program, then show the
answer-with-nothing form for input you cannot trust, then an order total that uses both.
