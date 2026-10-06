No `Core` member mutates an argument and none takes a reference. There is no `inout $out`, no
out-parameter and no in-place variant of anything; the result is the return value.

Copy-on-write makes this free rather than expensive: an argument whose refcount is 1 is mutated in place by
the implementation, so the copy is paid only when another holder can still see the original. What a
second, by-reference spelling would buy is the aliasing rules this removes, and it would cost a second name
for one operation (`rule:core-api/one-name-one-signature`).

The cost is real and is paid at every mutation site: `$a = Arr::sort($a);` is three characters longer than
`sort($a);` and reads as a copy even though it is not one. A program that genuinely wants by-reference
argument passing has `inout` (`rule:statements/inout-is-the-by-reference-spelling`), which no `Core` member
uses.
