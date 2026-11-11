The graph copy is a recursive, cycle-safe traversal of a value's reachable structure producing a
result that shares no mutable heap state with its source. Shared substructure stays shared — two
properties pointing at one nested object still point at one object on the other side — and a cycle
terminates instead of recursing. It never runs a constructor, and no hook fires: the shape that
crosses is the class's own declared properties, every time.

It refuses what has no meaning on the other side, naming the offending value and its path. An `inout`
binding aliases a specific frame, and the checker refuses it at the copy site. Six refusals are facts
about an object's class:

- a callable's class, which captures a heap and a scope;
- a generator's state class, which is a suspended frame;
- a class holding a host handle, whose key indexes the sending side's table;
- every other class the compiler made, known by a `$` in its name, except an anonymous object's
  `$shape{…}` class, which is data;
- a class with a `secret` property, named by its first one (`rule:security/secret-crosses-no-boundary`);
- a class with no instances, `abstract` or an interface, whose name in a payload is forged, since no
  instance of it was ever encoded.

One function, `ClassDesc::refusal`, answers those six, and it takes the door that asks: a live copy,
a copy through bytes, or a reflective `construct` (`rule:security/reflection-enforces-visibility`).
Every door that makes or copies an instance by name calls it, so the doors cannot come to disagree
about a class. The bytes door also refuses a `Core` class that does not state the type of each slot,
because `decode` checks every slot it fills against a stated type
(`rule:classes/serialize-is-a-closed-format`). The live door admits it: it copies an object a `Core`
member made, so its slots already hold their types.

| Class | `decode` | Live crossing | `construct` | `forClass` |
|---|---|---|---|---|
| A class the program declares, the error tree included | made | crosses | made | described |
| The same with a `secret` property | refused | refused | made | described |
| An `abstract` class or an interface | refused | has no instance | refused | described |
| An anonymous object's class, `$shape{…}` | made | crosses | refused | `null` |
| A function value's class, a generator's state class | refused | refused | refused | `null` |
| Every other class the compiler made | refused | refused | refused | `null` |
| A `Core` class that holds a host handle | refused | refused | refused | `null` |
| A `Core` class that states its slot types | made, each slot checked | crosses | refused | `null` |
| Every other `Core` class | refused | crosses | refused | `null` |

Both copy directions call it: the walk for every object it copies or encodes, and the decoder for
every class a payload names. `encode` asks the bytes door, so it refuses what `decode` refuses, with
the same sentence. Declared types make most of that a compile-time refusal at the copy
site; the run-time check is what a `mixed` carrying one of them needs. An object whose class the
receiving side cannot resolve is refused by name, never stubbed.

One walk serves both carriers — arena-to-arena at the isolate boundary, and bytes through
`Core\Serialize` — so a rule added to it reaches both or neither. Two implementations that agree today
is the failure that costs.

The crossings are these, and each calls a public function of the graph module:

| Crossing | Carrier | Lands in |
|---|---|---|
| `Core\Serialize::encode` and `decode` | bytes | the program that decodes |
| the memory and the shared cache tier | bytes | the program that reads the entry |
| the session record | bytes | the request that starts the session |
| `spawn` in and out, on one core | live | the child, then the parent |
| `spawn` placed on another core | bytes | the child, then the parent |
| `Core\Socket::upgrade` and `Core\Sse::upgrade` | live | the connection |
| the test fixtures | live, or bytes across cores | the test |
| `Core\Topic::publish` | live on one core, bytes across cores | each subscriber |

Every door that lands a value takes the receiving side
(`rule:security/isolate-values-cross-by-copy`), and a test reads every shipped call of one and fails
on a call that names none. `Core\Queue::push` is not a crossing: it writes `$args` as JSON, which
names no class.
