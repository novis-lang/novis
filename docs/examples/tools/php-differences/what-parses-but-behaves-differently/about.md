Code that compiles in both PHP and Novis and gives a different result in Novis.

`==` on two strings compares the text, so `"01" == "1"` is `false`. PHP returns `true` here. Two
arrays are equal when they have the same elements in the same order. Two objects are equal only
when they are the same object. An integer that gets too large throws `ArithmeticError`. `017` is
the decimal number 17, and octal is written `0o17`. `9.9 as int` throws an error, so write the
rounding yourself. `Core\Str::length("héllo")` is `5`, because a string counts characters and not
bytes. Array keys are always strings, so `$a[1]` is `$a["1"]`. `decimal` is a built-in type for
money.

`require` runs a file every time the program reaches it. A regular expression with a lookaround or
a backreference has a step limit, and it throws an error at that limit. `Core\Password` chooses the
hashing algorithm, and its `verify` method still reads a bcrypt hash that PHP stored.
