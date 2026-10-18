Each value that cannot be joined has its own error:

- a `bytes` value, an array, an enum case or a call that returns `void` is the error `E0707`.
  Write `$data as string` for `bytes`, `Core\Json::encode($list)` for an array and `$case as int`
  for an enum case.
- an object whose class does not implement `Stringable` is the error `E0412`.
- a `Core\Html\Markup` value is the error `E0710`. Join two of them with `+`, which gives a new
  `Markup`.

A `mixed` value is converted when the program runs. A `mixed` number joins like a number. A `mixed`
array throws a `RuntimeError` with the message "cannot convert an array to `string`".

`+` on two strings is the error `E0716`. `+`, `-`, `*` and `/` run before `.`, so `"n=" . $n + 1`
with `$n` equal to `2` gives `n=3`.

A `float` loses its `.0` when it is joined: `10.0` gives `10`. A `decimal` keeps every digit it has,
so `4.50` gives `4.50`. A `uint` gives all its digits, up to `18446744073709551615`.

`.=` writes a `string`, so the variable must be able to contain one. `$n .= "x"` on an `int` is the
error `E0401`. On a `?string` that is `null`, `.=` starts from an empty string.

When one side is `tainted` or `secret`, the result is `tainted` or `secret` too. `$s .= $input` on a
plain `string` `$s` is the error `E0401` when `$input` is `tainted`. `echo` of a joined `secret`
value is the error `E0790`.
