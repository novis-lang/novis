- **No ORM.** Data access is prepared statements plus derived codecs — a data mapper, where a row
  becomes a declared class and nothing is lazy. Active record is not merely absent, it is
  unrepresentable: it needs a property-read fallback, properties that are not definitely initialized,
  and a shape decided at run time, and all three are already refused elsewhere.
- **No runtime service container, and no facades.** Wiring is constructor injection resolved **while
  compiling**: `rule:programs/implementing` finds the one implementation of an interface, and a missing
  or ambiguous binding is a compile error naming the interface and the constructor parameter, rather
  than a container exception in production. A container that resolves a class by string name is the
  mechanism behind facades, and nothing it would need still exists.
- **No plugin auto-discovery beyond what exists.** `rule:programs/implementing`'s scan is the only
  discovery; there is no scan of a vendor directory for service providers.
- **No configuration cache, no route cache, no autoload dump.** Every one of those exists in an
  interpreted framework to move compile-time work off the request path, and every one of them is work Novis
  already does while compiling.
- **No second way to do anything the language does.** The framework ships no collections of its own, no
  date type, no string helpers and no error hierarchy.
