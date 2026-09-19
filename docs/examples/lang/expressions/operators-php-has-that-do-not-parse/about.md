PHP has a handful of operators that Novis does not, and writing one is a compile error that names
what to write instead.

`&&` and `||` are the only two logical connectives, so `and` and `or` are gone, and `xor` becomes
`!=` between two `bool` values. `==` and `!=` never convert either side, so there is nothing left
for `===`, `!==` and `<>` to tell apart and none of the three compiles. A cast such as `(int)$x` is
written `$x as int`. `@` has nothing to suppress, because a failure is thrown and you catch it.
There are no references (`&$x`), no variable variables (`$$name`) and no backticks. Adding two
arrays with `+` is `Core\Arr::underlay`, reading one character with `$s[0]` is `Core\Str::at`, and a
string has no `++`.

**Good to know:** every one of these errors reaches you when you compile, never while the program
runs.

**The examples below** show the three replacements you need most often: `&&` and `||` in a
condition, `==` where PHP wrote `===`, and `Core\Arr::underlay` and `Core\Str::at` where PHP used
`+` and `$s[0]`.
