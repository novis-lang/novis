A namespace matching no grant line holds what the application holds — the operator's ceiling. Denying
by default here would refuse to compile every program written before the table existed and force every
application to grant itself, which is ceremony charged to the common case.

That default is only safe because **nothing a package manager fetched is ever unmatched**: fetching
writes a grant line for every package in the resolved graph, direct and transitive, **including an
empty one**. The permissive default therefore applies only to code a human wrote or pasted, which is
the case where it is the right answer.

Hand-vendored code is the residue, and it is reported rather than closed: an audit warns on a
namespace whose autoload root lies under a directory the grant table never mentions. A warning is the
correct strength, because the same shape describes a legitimate second source tree of the
application's own.

**Not on disk.** There is no fetch step and no audit that writes or checks these lines.
