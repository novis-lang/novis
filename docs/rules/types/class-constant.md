`::class` answers the class the value **is**. Three sides are folded because the compiler already
knows the answer — `Foo::class`, `self::class` and `parent::class` resolve against the file's
namespace and imports and are inlined at every use site, which is what makes `Bogus::class` an `E0303`
rather than a string: there is no later place for the typo to be caught.

The two run-time sides read the name off a class descriptor:

| written | answers |
|---|---|
| `static::class` | the class the call was made on — the frame's late-static-binding class |
| `$obj::class` | the class the receiver was allocated from, one load off the object |

Neither is foldable, and the reason is the same for both: the static type is an upper bound, not the
answer. `User $u = new Admin(); echo $u::class;` prints `App\Admin`, and an inherited `static::class`
prints the subclass. Folding either would produce a string that is *silently* wrong rather than
absent.

The operand must carry a class statically. An object does, and a `class<T>` does — for which the name
is a conversion rather than a member read (`rule:types/class-reference`), answering the **descriptor's**
class rather than the `T` it was checked against, so `$name as class<Animal> as string` is the name it
started from. A `mixed` or a `?T` is **refused** (`E0702`): narrow it — an `is` test, or a `!= null`
one (`rule:types/narrowing`) — or ask reflection, whose whole purpose is the erased receiver. Accepting
`$m::class` on any operand would fail at run time on one that is not an object, and would put a tag test and a throw behind a spelling that reads like a member read.
The narrowing that lifts the refusal is the one `->` already requires of the same receiver.

Per evaluation the run-time form spends one load, one call and one string allocation for the name,
charged to the isolate that asked. A program that reads a name in a hot loop moves the read out of
the loop, as it would any other allocation.

`static::class` is what replaces PHP's `get_called_class`, and `$obj::class` replaces `get_class`.
Reflection stays the answer for every other question about a class. It is the wrong tool for a name:
it allocates a description of the class's whole member surface, and a program that wants a name to
log, to key a cache or to compare would pay for all of it.

- **Memory:** each run-time read allocates one string for the name in the isolate that asked. Nothing
  is cached per class, so no name string is shared between isolates. The folded sides spend nothing.

**Declined:**
- **`$obj::class` folded where the receiver's declared type is known.** It is wrong for a variable
  declared as a base class that holds a subclass, which is the case the feature exists for. A rule
  that is correct for some receivers and silently wrong for others is worse than either total rule.
- **Both run-time sides routed to reflection.** A static method has no instance to pass, and a
  program that wants one string would pay for a description of every member.
- **A `mixed` receiver accepted with a run-time tag test.** It puts a throw behind a syntax that reads
  as total, and the narrowing it needs is the one `->` already requires.
- **One cached name string per class descriptor.** The descriptor is shared by every isolate, so a
  shared string would put a reference count on the request-isolation boundary to save one allocation.
