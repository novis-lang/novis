Some values move into a wider place on their own, with nothing written at the spot where it happens.

A whole number goes where a fraction is wanted. Any value goes into a place that also accepts `null`,
into a place that accepts several types, and into a place that accepts anything. An object goes where
one of its parents, or one of the interfaces it carries out, is wanted. One of a small fixed set of
words goes where any text is wanted.

Only the first of those changes the value itself, and it is checked while the program runs. A whole
number past nine thousand million million cannot be held exactly as a fraction, so the program stops
there instead of quietly handing you a different number.

**Good to know:** nothing else in the language converts on its own. Text into a number, a number into
text, or a whole number into one that cannot be negative — each of those is written with `as`.

**The examples below** take whole numbers into fractions first, then the wider places, then a price
list that uses both.
