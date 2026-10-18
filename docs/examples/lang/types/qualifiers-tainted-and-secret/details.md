You can write `tainted`, `secret` or both, in the order `secret tainted string`. They go only before
`string` and `bytes`. `tainted int` and `tainted secret string` are syntax errors (`E0102`). You can
use them on a parameter, a local variable, a property and a return type.

A tainted string stays tainted through `.` and interpolation. It also stays tainted through every
`Core` method that returns text built from it, such as `Core\Str::upper`, `trim` and `split`. A method that
returns a count or a `bool`, such as `Core\Str::length` or `contains`, returns a plain value. A
parameter that is an instruction does not take a tainted string. That is a regular expression, a
`Core\Str::format` template, a `Core\Bytes::pack` format and a date pattern. Neither does a `Core`
parameter of type `string` that is not listed as taking one. Each of these is `E0401`, and so is
`string $plain = $input;` for a tainted `$input`.

`as int`, `as uint`, `as float`, `as bool` and `as` an enum give a plain value. `as string` and
`as bytes` keep `tainted` and `secret`, so `string $s = $input as string;` is `E0401`. Two methods
give a plain string that is safe for one use: `Core\Regex::quote` for a pattern and
`Core\Uri::encodeComponent` for a link.

A secret string stays secret through `.` and interpolation, separately from `tainted`. These do not
compile with a secret value. `echo` and `print` are `E0790`, also for `echo "Bearer {$token}"`.
An error message is `E0422`. `Core\Debug::dump` and `Core\Debug::render` are `E0724`, and
`Core\Json::encode` is `E0791`. Every `Core` parameter of type `string` or `bytes` is `E0401`. That
includes the parameters that take a tainted string, so `Core\Str::length($token)` does not compile.

`==` between two secrets, or between a secret and a plain string, compiles and gives a plain `bool`.
A declared `array<secret string>` keeps `secret`, so `Core\Json::encode` of it is `E0791`. An array
written in the code is an `array<mixed>`, and it has no `secret` elements. So
`Core\Json::encode([$token])` compiles and prints the value.

`Core\Secret::reveal($value, $reason)` gives a plain string, and `Core\Secret::revealBytes` gives
plain `bytes`. It removes only `secret`. For a `secret tainted string`, the result is a
`tainted string`.
