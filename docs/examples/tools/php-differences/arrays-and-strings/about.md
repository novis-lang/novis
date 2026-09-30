How the syntax of arrays and strings differs from PHP. Most of it is the same.

An array variable is declared with its element type before it is assigned, as
`array<int> $a = [1, 2];`. `var` does not work with an array literal, because it cannot find the
element type. A `foreach` loop does not take an array literal directly. Assign the literal to a
typed variable first. In destructuring with keys, each variable gets its type, as
`["a" => int $x] = $arr;`.

`array(1, 2)` is accepted, and `[1, 2]` is the usual way to write it. Interpolation in a string
works as in PHP, and heredoc and nowdoc strings work too. `Core\Debug::dump` replaces `print_r`,
`var_dump` and `var_export`.

**Good to know:** `Core\Debug::dump` writes to standard error and never to the output of the
program.
