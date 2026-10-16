Reads a schema out of the array that describes it.

`Core\Db\Schema::fromArray` takes an array of tables and columns and returns a `Core\Db\Schema`. That
value is what you plan against a database or apply to one.

The array has one key, `tables`, holding a list. A table is `name` and `columns`, and may also carry
`primary_key`, `unique` and `indexes`. A column is `name` and `type`, and may also carry `null`,
`identity` and `default`.

Every rule the schema vocabulary has is checked here, so a `Core\Db\Schema` that exists is one all five
databases can be asked for. An array that breaks a rule throws a `LogicError` naming the rule: a column
type nobody has, a table with no columns, an index over a column that is not there.

The value you get back holds the schema in one normalized form. Tables come in name order and every
key that was left out is filled in, so two programs that write one schema two ways still get the same
array from `Core\Db\Schema::toArray`.

**The examples below** write the tables a program needs, catch the error a wrong array throws, and
bring a database up to a schema that arrived as JSON.
