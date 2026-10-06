A short name resolves through the imports and then the enclosing namespace, and stops. If neither has it,
the name does not resolve, and there is no third place to look.

A name declared at the root — a user class in the global namespace, or the reserved exception tree
(`Throwable`, `LogicError`, `TimeoutError` and their siblings) — is reached from inside a namespace by
importing it:

```nvs
namespace App;
use Throwable;

try { ... } catch (Throwable $e) { ... }
```

A single-segment `use` is not a special form; it is the ordinary absolute path to a name whose path happens
to be one segment long. The reserved tree gets no auto-import: a name that resolves without appearing
anywhere in the file is a name a reader cannot trace, and one closed roster's worth of exception is a
second resolution rule to hold alongside the first. Code at the root namespace is unaffected, because its
enclosing namespace *is* the root.

PHP's fallback existed so a function or constant call could find a built-in from inside a namespace. With
no free functions and no free constants, nothing is left that it was for.
