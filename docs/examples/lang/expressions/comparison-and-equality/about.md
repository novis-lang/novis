`==` tests whether two values are equal, and `!=` tests whether they are different.

Neither operator converts a value, so `1 == "1"` does not compile. Numbers of different types do
compare, so `1 == 1.0` is true. Two arrays are equal when they have the same keys and values in the
same order. Two objects of your own class are equal only when they are the same object. Dates, times
and durations from `Core\Time` are compared by their value, so `90m == 1h30m` is true.

`<`, `<=`, `>`, `>=` and `<=>` put two values in order. They work on numbers, on `bool` values and
on objects of a class that implements `Comparable`. Use `Core\Str::compare` to order two strings.

**The examples below** compare text, arrays, objects and a missing value, and put two offers in
order. Two show compile errors. The last one compares dates and durations.
