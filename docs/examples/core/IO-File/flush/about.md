Sends everything that was written through a handle to the operating system.

`$file->flush()` works on a `Core\IO\File` that `Core\IO::open` returned. In Novis, `write`
already sends the bytes to the operating system before it returns. So another program or a
second handle can read the bytes at once, and `flush` does not change what they see. You can
still call `flush` after a group of writes, and your program keeps working the same way.

`flush` does not wait until the bytes are stored on the disk. After a power failure, the last
writes can still be lost.

A closed handle throws a `RuntimeError`.

This replaces PHP's `fflush`.
