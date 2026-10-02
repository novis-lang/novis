A list, or a lookup table, whose values all have the same type.

`var $ids = [1, 2];` creates an `array<int>`. The keys are not part of the type. Every value you add
later must have that type. Write the type yourself, as in `array<float> $weights = [2, 1];`, when the
array starts empty, when its values mix types, or when you will add values of a wider type later.

Keys are always strings: `$row[3]` and `$row["3"]` are the same entry. Entries keep the order you
added them in. An array of arrays is how you build a table or a grouped report.

An array is a value. When you pass it to a function, the function gets its own copy.

**Good to know:** reading a key that does not exist throws an error. Check first when a key may be
missing.

**The examples below** show a list made with `var` and one with a written type, then a lookup table,
then a grouped report that starts from an empty array.
