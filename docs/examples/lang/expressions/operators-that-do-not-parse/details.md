Each form has its own error code:

- `and`, `or` and `xor` are the error `E0226`.
- `===` and `!==` are the error `E0232`, and `<>` is the error `E0241`.
- A cast such as `(int)$x` or `(string)$x` is the error `E0225`.
- `@` before an expression is the error `E0236`. Catch the error with `try` and `catch` instead.
- `$a = &$b` is the error `E0701`.
- `$$name` and `${$name}` are the error `E0202`.
- `$a + $b` on two arrays is the error `E0467`.
- `$s[0]` on a string is the error `E0482`.
- `$s++` on a string is the error `E0474`.
- `eval(…)` is the error `E0201`. Put the code in a file and `require` it.

A backtick and `&$x` in an expression give a general syntax error. That error does not name what
to write instead. To run another program, use `Core\Process`. To let a method change a variable,
use an `inout` parameter.

After `===`, `!==` or `<>`, the compiler reads the line as if `==` or `!=` were written. So it also
reports every other error in the file in the same run.
