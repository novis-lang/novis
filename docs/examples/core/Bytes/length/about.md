Counts the bytes in a `bytes` value.

The result is the number of bytes. For a value with nothing in it the result is `0`. This is not the
number of characters. A text written in UTF-8 uses one byte for `a` and two bytes for `é`. Counting
the bytes of such a text gives a larger number than counting its characters. Use `Core\Str::length`
when you want the characters a person sees.

Counting is immediate. The size is stored with the value, so the method does not read the bytes. A
value of one hundred megabytes is counted as fast as a value of ten bytes.

**Good to know:** a size limit on an upload, a request body or a stored value is a limit on bytes.
This is the method those checks use.
