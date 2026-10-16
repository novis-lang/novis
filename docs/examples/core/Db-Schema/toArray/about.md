Returns the schema as the array that describes it.

`Core\Db\Schema::toArray` gives you back the array form of a `Core\Db\Schema`. It is the same shape
`Core\Db\Schema::fromArray` reads, so a schema can go from a value to an array and back as often as you
like. `Core\Json::encode` turns that array into the text `nvs schema dump` writes.

The array you get is normalized. The tables are in name order, and every key a description left out is
written down: a column always has `null` and `identity`, and a table always has `primary_key`, `unique`
and `indexes`. This means one schema has exactly one array form. Two programs that describe the same
tables in different words get the same array from this method, so you can compare two schemas by
comparing their arrays.

The array is the value the schema already holds, so calling this method twice costs nothing the second
time. Writing into the array you got back does not change the schema.

**The examples below** read the tables a schema describes, show that two descriptions of one schema give
one array, and check that a schema file stored next to a program is still up to date.
