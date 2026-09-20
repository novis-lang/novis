Copies a part of a `bytes` value into a new value.

The part starts at the position you give, counted in bytes, and the first byte is at position `0`. A
negative position counts from the end of the value, so `-4` starts at the fourth byte from the end.
The length says how many bytes to take. A negative length stops that many bytes short of the end.
When you leave the length out, the part runs to the end of the value.

The value you pass stays as it is. A position after the end of the value returns a value with
nothing in it, and so does a length of `0`.

**Good to know:** the positions count bytes, not characters. Use `Core\Str::slice` for text that a
person reads, where one character can be several bytes.
