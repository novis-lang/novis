Some operators do not exist in Novis. Writing one is a compile error, and the error names what to
write instead.

`&&` and `||` are the only two logical operators that join conditions. `and` and `or` do not exist,
and `xor` is `!=` between two `bool` values. `==` and `!=` never convert either side, so `===`,
`!==` and `<>` do not compile. A cast such as `(int)$x` is written `$x as int`. `@` does not exist,
because a failure is thrown and you catch it. There are no references (`&$x`), no variable variables
(`$$name`) and no backticks. To add two arrays, use `Core\Arr::underlay`. To read one character, use
`Core\Str::at`. A string has no `++`.

**Good to know:** every one of these errors appears when you compile, never while the program runs.

**The examples below** show the replacements you need most often: `&&` and `||` in a condition,
`==` for equality, and `Core\Arr::underlay` and `Core\Str::at`.
