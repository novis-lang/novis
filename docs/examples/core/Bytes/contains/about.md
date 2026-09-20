Tells you whether one `bytes` value occurs anywhere inside another.

The result is `true` when the bytes you are looking for appear somewhere in the value you are
searching, and `false` when they do not. The search matches bytes exactly, one after the other. There
is no option to ignore the difference between upper case and lower case, because a `bytes` value
carries no language and no character set. Use `Core\Bytes::indexOf` instead when you also need the
position.

**Good to know:** an empty value occurs in everything, so searching for one always returns `true`.
