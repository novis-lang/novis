`Core\Metrics::increment` adds to a counter: a number that only goes up, such as how many orders
were placed or how many logins failed.

Each call adds one. The option `by` adds a different amount, and it can never be negative. When your
monitoring system collects the metrics, it sees the total. The name is written in lower case
letters, digits and `_`, and by custom a counter's name ends in `_total`. The first use decides that
this name is a counter. Calling `observe` or `gauge` with the same name later throws a `LogicError`.

Labels split one counter into several, for example one per payment status. A label value from a
request is not allowed, because every different value makes a new series.

**Good to know:** a program cannot read the total back. The call returns nothing.

**The examples below** show counting one event, then adding more than one with `by`, then counting
payments by their result.
