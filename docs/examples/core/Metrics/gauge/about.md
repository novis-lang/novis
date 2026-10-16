`Core\Metrics::gauge` sets a gauge: a number that goes up and down, such as how many jobs are
waiting or how many connections are open.

Each call replaces the value. It does not add to it. When your monitoring system collects the
metrics, it sees the last value that was set. The name is written in lower case letters, digits and
`_`, and the first use decides that this name is a gauge. Calling `increment` or `observe` with the
same name later throws a `LogicError`.

Labels split one gauge into several, for example one per database. A label value from a request is
not allowed, because every different value makes a new series.

**Good to know:** a program cannot read the value back. The call returns nothing.

**The examples below** show a count of waiting jobs, then the error for a name that is already a
counter, then one gauge per connection pool.
