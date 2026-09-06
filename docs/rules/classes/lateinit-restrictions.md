`lateinit` is admitted only on a non-nullable property whose type is a class or an interface. A scalar
is refused, naming the fix — a scalar's "no value yet" is already free, spelled `= 0`, which is exactly
the placeholder an object type does not have. `?T lateinit` is refused, because nullability already
spells "may hold no value" and a nullable `lateinit` read would leave open whether a `null` is the
deliberate value or the unwritten marker surfacing. A promoted constructor parameter is refused,
because binding the parameter already is the assignment there is nothing left to defer.

`lateinit readonly` is refused as well. `readonly` promises "assigned exactly once, during
construction"; `lateinit` promises "assigned after construction, and freely reassignable". They are
opposite claims about one property rather than a composable pair.

Each refusal is its own diagnostic, which is one more modifier and three more rejected combinations a
developer has to learn are refused rather than silently allowed.
