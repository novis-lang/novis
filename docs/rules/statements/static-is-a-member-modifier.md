`static` is a class-member modifier and a class-relative type. Static methods, static properties,
`static::`, `static::$prop`, `new static()` and `: static` all carry their PHP meanings unchanged: late
static binding is load-bearing in the OO code Novis converts, and `new static()` compiled as `new self()`
would return the wrong class rather than fail to compile. `static::class` is the called class's own name,
read at run time off the descriptor the frame already holds.

A `static` member declares its type like every other member — `public static int $n = 0;` — and a body
declaring `static` as its return type may not return the declaring class, since a subclass call site is
promised its own.

The keyword therefore has one meaning per position: a modifier before a member, a type or a scope in an
expression. Nothing about it depends on whether it appears inside a function body, because inside a
function body it is not a declaration at all — see `rule:statements/no-function-static-and-no-global`.
