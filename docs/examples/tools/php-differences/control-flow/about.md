How blocks, `require`, `try` and `yield` differ from PHP.

Blocks use braces only, so `endif` and `endforeach` do not exist. `goto` and
`declare(strict_types=1)` do not exist, and every program is strict. `die` is written `exit`.
`require` replaces `include`, `include_once` and `require_once`. It throws an error when the file is
missing, and it runs the file every time.

A `catch` clause names one class. For two classes, write two clauses, each with its own variable.
The variable ends with its clause. The class `Exception` does not exist. Every error is a
`Throwable`, and your own error class extends `RuntimeError`. The message is the property
`$e->message`. `yield` gives a value without a key, and `list($a, $b)` is written
`[int $a, int $b]`.

**Good to know:** a `match` with no matching arm throws an error, so add a `default` arm. A `switch`
continues into the next case without `break`, as in PHP.
