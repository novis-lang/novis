The check is **not** a line in a member's body that an author remembers to write, **not** at the call
site in emitted code, and **not** in a dispatcher. It is inside the function that performs the effect
— the one that opens the file, or dials the socket.

A `Core` member cannot reach the operating system another way, because the standard library **may not
name the spellings that perform an effect**, and a test reads the crate's own sources and fails on
one. So "the author forgot the check" is not a failure mode that exists: forgetting it means calling
the raw spelling directly, and that does not get past the test. The list is of *spellings*, not of
modules, because an address parser and a process abort live beside the doors without being ones.

Every other placement leaves the check *beside* the effect, where omitting it is a silent hole that
reviews are expected to catch. This one puts it *in* the effect, where omitting it means not
performing the effect. The friction is the point: the door is where the check, the path rule and the
diagnostic already are.
