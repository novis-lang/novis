`Core\Metrics::observe` records one measurement, such as how long a request took or how big an
upload was. The measurements go into a histogram.

A histogram does not keep each value. It counts how many values were in each range, and it keeps
their number and their sum. With these, your monitoring system can show the average and how slow the
slowest requests were. Give the value in the unit the name ends with: seconds for `_seconds`, bytes
for `_bytes`.

The first use decides that this name is a histogram. Calling `increment` or `gauge` with the same
name later throws a `LogicError`. Labels split one histogram into several, for example one per
database table.

**Good to know:** a program cannot read the histogram back. The call returns nothing.

**The examples below** show recording durations, then recording sizes in bytes, then timing queries
per table with a label.
