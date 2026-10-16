A read or an element write that finds a `lateinit` property uninitialized, on an object whose class
implements `LateInit`, calls `initProperty(string $name): void` with the property's name and then
reads the slot again. If the property is still uninitialized, the read throws a `LogicError`:
"`initProperty` returned, but `$author` is still not initialized." Without `LateInit` the read throws
`rule:classes/an-unwritten-property-read-throws`'s error. A property already initialized, by a write
or by an earlier call, calls nothing, and `isinit` never calls it
(`rule:classes/isinit-reads-nothing`).

`initProperty` runs once per property per object while it succeeds. A throw out of it leaves the
property uninitialized and propagates from the read, so the next read calls it again. A read of the
same property of the same object, in the same task, while its own `initProperty` runs throws a
`LogicError`: "`$author` is read while `initProperty` initializes it." The class is decided at run
time from the object's own class, so a compiled read and a read through `mixed` or `Core\Reflect`
behave the same.

**Across tasks.** A task shares its request's objects (`rule:concurrency/one-scheduler`), so a read
from another task can find the call running. That read parks until the call ends
(`rule:concurrency/the-parking-contract`). After a return it reads the slot again and never calls
`initProperty` a second time. After a throw or a cancellation the property is still uninitialized,
and the parked read looks again: it calls `initProperty` itself, or parks on a call a third task
started first. Coordinating a batch across *different* objects is the program's own job, with a
`Core\Task\Channel`. An isolate shares no object, so it never sees another isolate's calls.

**Plain writes.** While an `initProperty` call runs in a task, a write in that task that initializes
an uninitialized `lateinit` property is plain, on any object and through any write form: no
`beforePropertySet` runs and no `onPropertySet` is told
(`rule:classes/property-observer-pipeline`). One call can therefore load a
whole row into several properties, as a data mapper does. Every other write inside `initProperty` —
to a property that already has a value, or a second write to the same one — is ordinary.

The compiled read keeps the one tag compare of `rule:classes/an-unwritten-property-read-throws`, and
only its uninitialized branch reaches the runtime. An observed `lateinit` write tests the tag before it
stores, and asks whether an `initProperty` call is running only when the slot was uninitialized.

- **Memory:** per `initProperty` call while it runs, one two-word entry in the task's own table and
  one in the request's shared table, keyed by object and slot, plus one entry per read parked on it.
  The shared table is one small allocation per request that calls `initProperty` or starts a task.
  Nothing per property or per object.

**Declined:**
- **Every write inside `initProperty` is plain.** Any validation an observer makes could then be
  skipped by writing inside an `initProperty`, and security ranks first.
- **A method per property** (`initAuthor`). One method keeps a batch load in one place, and the name
  argument already says which property was read.
- **One interface for `LateInit`, `PropertyObserver` and a guard.** A class pays only for the contract
  it implements.
