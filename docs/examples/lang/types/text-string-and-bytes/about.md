Text is a `string`; anything that is not text yet is `bytes`. They are separate types, and a value
crosses from one to the other only where you say so.

A `string` always holds valid text, and it is measured in characters the way a person counts them: an
accented letter, an emoji or a flag is one character, however many bytes it takes to store. There is
no `$name[0]` — `Core\Str::at` hands you one character and `Core\Str::slice` a run of them, so a
position you get from one member is safe to hand to another.

`bytes` is for a file you have read, a reply from a socket, a hash — data nobody has yet claimed is
text. It is counted and indexed in bytes, the only unit it has.

**Good to know:** turning a string into bytes always works, but turning bytes into a string checks
the data first and stops with an error when it is not valid text, so a stray byte never slips into
your text unnoticed.
