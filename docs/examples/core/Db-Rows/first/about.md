Gives the first row a query answered, or `null` when it answered none.

`Core\Db\Rows::first` returns the first row of the result, in the order the query asked for. A query
that matched nothing returns `null`, so a lookup that found nothing is an answer your program tests
for, and not an error to catch.

There is no cursor to move on. Two calls return the first row both times, which is what makes it safe
to read inside a condition and read again after it.

A result from `queryAs` returns an object of the class you named there, in the same way the whole
result does.

**The examples below** read the one row a lookup found, say so when a lookup found nothing, and read
the first row as an object of your own class.
