---
summary: a list of values bound to one placeholder of a query, for `in` and `not in`. `Core\Db::inList` returns it
keywords: IN list, NOT IN, where in, array parameter, list parameter, bind an array, placeholders, several values, SQL IN condition
---

`Core\Db\InList` is a list of values for a SQL `in` or `not in` condition. `Core\Db::inList` returns
it. You write one `?` or one `:name` in the SQL, and you bind the list to it. The placeholder then
becomes a list in parentheses with one placeholder for each value, such as `(?, ?, ?)`. The values are
bound, so they never become part of the SQL text.

An `InList` has no methods. You use it only as a value in the `$params` array of a statement. One list
can be used in more than one statement. The values keep the order of the array, and its keys are
ignored. A plain array in `$params` throws a `LogicError`, because only an `InList` expands.

An empty array throws a `LogicError` when you call `inList`. Inside `in`, an empty list would match
nothing. Inside `not in`, it would match everything. Check for an empty array before you call
`inList`. Each value in the list must be a single value. An array or another `InList` in the list
throws a `LogicError` when the statement runs.

```nvs
<?nvs
var $db = Core\Db::open({ driver: Core\Db\Driver::Sqlite, path: ':memory:' });
$db->execute('create table task (id integer primary key, state text not null)', []);
$db->executeMany('insert into task (id, state) values (?, ?)', [
    [1, 'open'],
    [2, 'done'],
    [3, 'cancelled'],
    [4, 'open'],
]);

// The one `?` becomes `(?, ?)`, so this prints the tasks 2 and 3.
var $closed = Core\Db::inList(['done', 'cancelled']);
foreach ($db->query('select id from task where state in ? order by id', [$closed]) as Core\Db\Row $row) {
    echo 'closed: ', $row->int('id') ?? 0, "\n";
}

// The same list works with a `:name` placeholder and with `not in`.
foreach ($db->query('select id from task where state not in :states order by id', ['states' => $closed]) as Core\Db\Row $row) {
    echo 'still open: ', $row->int('id') ?? 0, "\n";
}

// An empty array throws a `LogicError`.
try {
    Core\Db::inList([]);
} catch (LogicError $e) {
    echo "an empty list is not allowed\n";
}
```
```output
closed: 2
closed: 3
still open: 1
still open: 4
an empty list is not allowed
```
