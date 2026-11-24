Counts how many entries of an array fall under each distinct value.

You get back one entry per group. Its key is the value that was counted and its value is how many
times that value appeared. The groups are in the order their first entry appeared, not in
alphabetical order and not by size.

An array key can only be a whole number or text, so a counted value has to be one of those two. The
number 1 and the text "1" name the same group, because that is how every array key works. A value
that is neither a whole number nor text stops the program with an error.

The `by` option changes what is counted. It is a function that receives an entry and returns the name
of the group that entry belongs to. With it you can count by one field of a record, or by any rule
you can write.

The examples show how often each value appears, how to count by a field, and how to build a report of
orders per day.
