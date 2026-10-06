The leading separator is refused in all three positions a name could carry one, rather than stripped:

```nvs
\App\Models\User::find(1);   // refused
use \App\Models\User;        // refused
namespace \App;              // refused
```

`E0240` names the spelling without it: *a name is already absolute; write `App\Models\User` without the
leading `\`.*

Refusing rather than repairing is the reading ambiguous input gets everywhere: a spelling the compiler
silently accepts is a spelling that spreads. A qualified name is read from the root anyway
(`rule:statements/a-qualified-name-is-absolute`), so the token has no work left to do, and accepting it
would restore exactly the two-spellings-one-meaning state removed here with the confusion moved from
resolution into style. The formatter does not strip it either — a construct the parser rejects never
reaches the formatter.
