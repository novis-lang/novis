`fields()` returns the extra fields of a log record, as an array. In a test, `Core\Test::logs()`
returns one `Core\Test\LogRecord` for each record, and `fields()` is one of its methods.

The array contains the `$fields` that your program gave to `Core\Log::write`, in the same order. A
string, a number, a `bool` and `null` keep their value. The array is empty when the record has no
fields.

**Good to know:** the fields are returned the way the log wrote them. JSON has no type for some
values, such as `bytes`. Such a value is the same as in a JSON log line. For `bytes`, that is an
array with one key, `$bytes`, and the bytes as hex digits.

**The examples below** show a slow database query with its duration, the value of each type of
field, and an upload request that logs the size of the file.
