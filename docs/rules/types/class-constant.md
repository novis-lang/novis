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
charged to the isolate that asked.
