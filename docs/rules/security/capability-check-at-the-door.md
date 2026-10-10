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
diagnostic already are, so a member that goes through it inherits all three.

**Declined:**
- **A check emitted at every call site, driven by the member's row.** It costs a branch and a snapshot
  read at every call site rather than one inside the effect, misses a member reached any way other
  than a compiled call, and has to carry the scoped argument into the check at every site: more
  machinery for less coverage.
- **A guard the member-definition macro emits before every member body.** The macro cannot see the
  row, so it would look the member up on every `Core` call, a table lookup on a string member to learn
  that it needs nothing (`rule:security/capability-costs-nothing-unasked`).
- **The check written by hand as the first line of each body.** The completeness test would then have
  to prove it by reading source for a call, a test about text rather than data; putting the check
  inside the only door makes the same guarantee with no such test.
