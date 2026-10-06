`static` is a class-member modifier and a class-relative type. Static methods, static properties,
`static::`, `static::$prop`, `new static()` and `: static` all bind late, to the called class, and
`new static()` compiled as `new self()` would return the wrong class rather than fail to compile.
`static::class` is the called class's own name,
read at run time off the descriptor the frame already holds.

`static::NAME` is late-bound the same way, and it is the one class-constant read that is not inlined:
the frame's called class is asked for `NAME` when the read runs, so a default method or an inherited
body reads the implementor's override where `self::NAME` reads the declaring class's value. Three things
follow from that read being a run-time one. A redeclaration of an inherited constant keeps a type
assignable to the ancestor's, in every class and interface, because the read is typed where the
constant is declared and answers whichever class the call was made on (`E0833`). A `static::` constant
is refused in a constant expression — a default, a payload — which is folded once with no call to bind
it to (`E0831`). A `static::` read of a constant with no scalar value — an `array`, an object, `null`
— is refused, because the called class's table carries `string`, `int`, `uint`, `bool` and `float`
and nothing else (`E0832`); `self::` and the class name still inline such a constant.

An anonymous function's body is a frame of its own and carries no called class, so `static::` in any
form inside one is refused (`E0834`). `self::m()` and `parent::m()` inside one call the class the
anonymous function is written in,
as a written class name would: a static method reached that way reads that class as `static`, whichever
subclass the enclosing method was called on. An instance method reached that way is called on the
anonymous function's `$this`, which the call captures, and reads `$this`'s class as `static` as it
does everywhere.

A site that sets the called class leaves no late binding to reach an override, so a call from one to an
`abstract static` method is refused where it is written (`E0835`) unless the class it names, or a class
or interface above it, declares the method with a body. That is `Page::title()` on the abstract class
that declares `title`, and `self::title()` inside an anonymous function in `Page`'s methods. `self::`, `static::` and
`parent::` in a method body forward the called class and are not refused, so an inherited static method
calling `self::title()` runs the subclass's body.

A `static` member declares its type like every other member — `public static int $n = 0;` — and a body
declaring `static` as its return type may not return the declaring class, since a subclass call site is
promised its own.

The keyword therefore has one meaning per position: a modifier before a member, a type or a scope in an
expression. Nothing about it depends on whether it appears inside a function body, because inside a
function body it is not a declaration at all — see `rule:statements/no-function-static-and-no-global`.
