The root every error in Novis descends from, and the one a program's own errors extend.

Writing `catch (Throwable $e)` means *anything at all*: the two halves of the tree, every narrower
name under them, and whatever the application declared itself. Every one of them carries the same
four things — the `message`, the `location` it was thrown from, the `backtrace` of frames it passed
through, and the `previous` error it was raised in answer to.

There is no `Exception` and no `Error`. A class of your own says `extends Throwable`, which puts it
beside the built-in halves rather than inside either, so a library catching mistakes or catching the
world's refusals never takes it by accident.

**Good to know:** the widest clause goes last. A `catch` list is read top to bottom and the first
matching arm wins, so a `Throwable` arm above a narrower one would take everything.

**The examples below** show an error of your own, reading what a caught one carries, and a request
handler whose last clause turns anything unexpected into one reply.
