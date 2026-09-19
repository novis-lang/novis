`=` writes a value into a local variable, a property, a static property or an array element. It is
an expression as well as a statement: its value is what was just written, so `$a = $b = 3` sets both
and `($a = 5) + 1` gives 6.

Every compound form — `+=`, `-=`, `*=`, `.=`, `|=` and the rest — is the long way round written
once. `$x op= e` means `$x = $x op e`, with the target worked out a single time. That last part is
what makes `$totals[next()] += 1` safe: `next()` runs once, not twice.

A compound form cannot change the type of what it writes into, so `$n /= 2` on an `int` is refused —
division answers a whole number or a fraction, and the variable was declared to hold only whole
numbers. A local is declared with a type before anything is written to it, so assigning to a name
nobody declared does not compile either.

**The examples below** take these in turn: writing and updating a value, updating what is inside an
array and an object, and tallying a report as its rows arrive.
