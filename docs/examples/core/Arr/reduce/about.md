Folds a whole array into one value. A function is called for every entry, and each call receives the
value the fold has so far.

You give `Core\Arr::reduce` an array, a function and a start value. The function receives the value
so far, the entry's value and the entry's key, and returns the next value. A function that needs
fewer of them declares fewer parameters.

The start value decides the type of the result. A fold that starts from `0` returns a whole number,
whatever else happens inside the function, and a fold that builds a text starts from `''`. Over an
empty array the start value is the result, and the function is not called at all.

**Good to know:** for a plain total use `Core\Arr::sum`, which is shorter and faster.
