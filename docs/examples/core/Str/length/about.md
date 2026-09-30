Counts the characters in a string, the way a person would count them.

An emoji, an accented letter or a flag each count as one character, even when the computer stores
them as several bytes. Every `Core\Str` method counts the same way, so a position you get from one
method is correct for every other one.

**Good to know:** this is not the size of the string in memory. If you need bytes, for a file size
or a network limit, use `Core\Bytes::length`.
