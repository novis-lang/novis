Takes an exclusive lock on an open file. Only one handle can have the lock at a time.

`$file->lock()` works on a `Core\IO\File` that `Core\IO::open` returned. When no other handle
has the lock, `lock` takes it and returns. When another handle already has it, `lock` does not
wait. It throws an `IOError` at once. Your program can then stop, or try again later.

There is no `unlock` method. The lock is released when the handle is closed, with `close` or at
the end of the request.

Use a lock when two programs, or two requests, must not change the same file at the same time.
On some systems a lock does not stop a handle without the lock from reading or writing the file.
So every program that changes the file should call `lock` first.

A closed handle throws a `RuntimeError`.
