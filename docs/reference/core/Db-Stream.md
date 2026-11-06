---
summary: the rows of a query, read from the database one at a time in a `foreach` loop. `stream` and `streamAs` return it
keywords: stream, streaming, large result, big table, one row at a time, constant memory, iterate rows, foreach, unbuffered query, typed rows, cursor
---

`Core\Db\Stream<T>` is the result of `Core\Db\Connection::stream` and `Core\Db\Connection::streamAs`.
`Core\Db\Transaction` has the same two methods. You read a stream with a `foreach` loop. Each round
of the loop reads one row from the database. Only the current row is in memory, so a large result
uses as much memory as a small one.

With `stream`, each item is a `Core\Db\Row`. With `streamAs<City>`, each item is a new `City` object
built from the row. `query` works differently. It reads every row into memory before it returns, and
it returns a `Core\Db\Rows`. You can loop over a `Core\Db\Rows` many times. A stream is read once, and
a second loop over it gives no rows.

The connection is busy from the call to `stream` until the loop reads the last row. Another statement
on the same connection in that time throws a `LogicError`. Read all the rows first, or open a second
connection. A loop that ends early with `break` keeps the connection busy until the request ends. To
read only the first rows, write a `limit` in the SQL.

A row that does not match the class of `streamAs` throws a `ParseError`, and the loop stops at that
row. The `timeout` option is the time for the whole loop. When it passes, the loop stops with an
`IOError`.

```nvs
<?nvs
#[Core\Db\Derive]
class City
{
    public uint $id;
    // Text from the database is `tainted`, because it comes from outside the program.
    public tainted string $name;

    public function constructor(uint $id, tainted string $name)
    {
        $this->id = $id;
        $this->name = $name;
    }
}

var $db = Core\Db::open({ driver: Core\Db\Driver::Sqlite, path: ':memory:' });
$db->execute('create table city (id integer primary key, name text not null)', []);
$db->executeMany('insert into city (id, name) values (?, ?)', [[1, 'Berlin'], [2, 'Hamburg'], [3, 'Bremen']]);

Core\Db\Stream<Core\Db\Row> $rows = $db->stream('select id, name from city order by id', []);

// No row is read yet, but the connection is busy. This query throws a `LogicError`.
try {
    $db->query('select count(*) as n from city', []);
} catch (LogicError $e) {
    echo "the connection is busy\n";
}

foreach ($rows as Core\Db\Row $row) {
    echo $row->int('id') ?? 0, ' ', $row->string('name') ?? '', "\n";
}

// The stream is read. A second loop over it prints nothing.
foreach ($rows as Core\Db\Row $row) {
    echo "this line is never printed\n";
}

// `streamAs<City>` builds one `City` object from each row.
foreach ($db->streamAs<City>('select id, name from city where id > ? order by id', [1]) as City $city) {
    echo $city->id, ': ', $city->name, "\n";
}
```
```output
the connection is busy
1 Berlin
2 Hamburg
3 Bremen
2: Hamburg
3: Bremen
```
