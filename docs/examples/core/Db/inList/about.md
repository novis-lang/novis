Marks an array as a list of values, so one placeholder in a statement stands for all of them.

A placeholder normally binds one value. `where id in ?` with an array of three ids is not what a
database expects: it wants `where id in (?, ?, ?)`. Most programs build that text themselves,
counting the values and gluing that many question marks together. `Core\Db::inList` does it for you.
You write `?` once, and the marker tells Novis how wide the list is.

The marker is an ordinary value. You can put it in a variable and hand it on, and you bind it in the
`$params` array like any other value. It works the same inside `in` and inside `not in`.

An empty list throws a `LogicError`. Inside `in` an empty list matches nothing, and inside `not in`
it matches everything, so there is no answer that is right in both places. Test the list before you
mark it, and take the other branch when it is empty.

**Good to know:** the SQL text never changes. Your program does not build it from a count, so a
value from a visitor cannot reach the statement as text.
