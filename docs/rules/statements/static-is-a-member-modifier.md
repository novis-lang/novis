`static` is a class-member modifier and a class-relative type. Static methods, `static::`, `new
static()` and `: static` all bind late, to the called class, and `new static()` compiled as `new
self()` would return the wrong class rather than fail to compile. A static method called through an
instance, `$base->m()`, names the object's run-time class, as `$this->m()` does: a `Child` held in a
`Base` variable runs `Child::m`, which reads `Child` as `static`. A callable taken with `static::m`
keeps the frame's called class and runs that class's `m`. A static property does not:
its storage is fixed where the access is written, so `static::$prop` is refused (`E0499`). For a
subclass that redeclares the property it would silently name the parent's storage, while a reader
expects the called class's. `self::$prop` and a written class name say which storage is meant.
A static property is never reached through an instance: `$p->x` and `$p?->x` are `E0869`, whose
message names `P::$x`, because an instance has no slot for it.
`static::class` is the called class's own name,
read at run time off the descriptor the frame already holds.

A `static` anywhere in a parameter type or a return type is the class the call names, at
`Category::find(...)` and at `$category->find(...)` alike: `property<static>`, `array<static>`,
`?static` and `: static` are all bound at the call. A base class that declares
`sortBy(property<static> $field)` once then checks each subclass's own properties at each call, with
no override per subclass. A type that also writes `self` or the declaring class's own name is not
bound, and neither is a `static` among a written callable's parameters; both keep the declaring class.
Inside the body the checker does not know which subclass called it, so a `property<static>` is the
property's name as a `string`, which the body may print, compare or convert back with `as
property<self>`, checked at run time. A key of `static` read on `$this` would be sound, but the body
types `$this` as the declaring class and cannot tell it from any other value of that class.
A `propertyPath<static>` is the path's text in the same way, and a `string<…static…>` is a `string`.
To reach `Core\Str::parts` or `Core\Reflect::steps` (`rule:core-classes/str-parts-and-reflect-steps`)
the body converts the text with `as propertyPath<self>` or `as string<propertyPath<self>>`, checked
at run time against the declaring class. A path through a property only a subclass declares throws
there: `steps` reads the class a path starts at off the argument's type, and the body has no type
naming the subclass.

`static::NAME` is late-bound the same way, and it is the one class-constant read that is not inlined:
the frame's called class is asked for `NAME` when the read runs, so a default method or an inherited
body reads the implementor's override where `self::NAME` reads the declaring class's value. Three things
follow from that read being a run-time one. A redeclaration of an inherited constant keeps a type
assignable to the ancestor's, in every class and interface, because the read is typed where the
constant is declared and answers whichever class the call was made on (`E0833`). The read converts
the called class's value to that type: a subclass's `int` read where `int|string` is declared is that
union holding the `int`. Nothing converts an `int` to a `float` there, so `int` over `float` is
`E0833` too, as it is for an override's return type. A `static::` constant
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
does everywhere. For `parent::m()` the class the call is made on is still the one the function is
written in: the parent is only where the method is looked up. PHP forwards the enclosing method's
called class into the anonymous function instead, so there a static method reached this way sees the
subclass. `self::` already means the written class for constants, `::class` and `self::m`, and
this reading stores nothing new.

A site that sets the called class leaves no late binding to reach an override, so a call from one to an
`abstract static` method is refused where it is written (`E0835`) unless the class it names, or a class
or interface above it, declares the method with a body. That is `Page::title()` on the abstract class
that declares `title`, and `self::title()` inside an anonymous function in `Page`'s methods. `self::`, `static::` and
`parent::` in a method body forward the called class and are not refused, so an inherited static method
calling `self::title()` runs the subclass's body.

A `static` member declares its type like every other member — `public static int $n = 0;` — and a body
declaring `static`, `?static` or `array<static>` as its return type may not return the declaring
class, since a subclass call site is promised its own (`E0741`). What it may return is `$this`, `new
static(...)`, `null`, a call on `static::`, `self::`, `parent::` or `$this` whose own return binds
`static`, a `? :` of two of these, or an array literal of them. A local variable is none of these, so
an `array<static>` built up in a loop is refused, and declaring `array<self>` is the way to write it.

The keyword therefore has one meaning per position: a modifier before a member, a type or a scope in an
expression. Nothing about it depends on whether it appears inside a function body, because inside a
function body it is not a declaration at all — see `rule:statements/no-function-static-and-no-global`.

Late static binding stays because fluent builders, record-style models and static factories are
written with it.

- **Memory:** only code that writes `static::` on a constant pays for the late read. Each read scans
  the called class's constant table, which is the size its declarations wrote, and builds the name as
  a fresh string released when the read returns. A `string` constant costs one allocation per read,
  charged to the isolate, as `static::class` does: the process-wide table's strings are not shared
  across isolates. `self::NAME` and `Class::NAME` still inline and cost nothing at run time. A
  `self::m()` or `parent::m()` call inside an anonymous function sets a class fixed at the call site
  and costs nothing, and only one that reaches an instance method captures `$this`. Binding `static`
  in a parameter or a return is the checker's, once per call it resolves, and costs nothing at run time.

**Declined:**
- **Refusing `static::NAME`.** It would be sound, but it drops a meaning PHP programs rely on: a
  default method that reads the implementor's constant.
- **Choosing the constant per called class at compile time.** The checker does not hold the set of
  classes a body can be called on, since a class in an autoload root nobody has named yet is still a
  caller.
- **Reading an `array` constant late.** The table stores a folded scalar, so an array would have to be
  rebuilt from its constant form on every read, a second constant builder beside the compiler's.
  `self::` still inlines it.
- **Carrying the enclosing method's called class into an anonymous function.** It is PHP's meaning
  and would admit `static::` there too, but the function object has no slot that holds a class, so it
  is a change to the layout of every anonymous function. It is the better answer if that slot is ever
  added.
- **Refusing `self::m()` and `parent::m()` inside an anonymous function, as `static::` is.** Every
  anonymous function that calls a private static helper would have to name its class in full.
- **Writing the class into each subclass's override of a method taking `property<static>`.** It is the
  boilerplate late binding exists to remove, and a missed override checks against the base class's
  properties.
- **Binding `static` in a parameter at run time.** The call site already names the class, so the check
  belongs where the call is written.
- **A `static` type kept apart from the declaring class inside the body.** It would let a body read a
  `property<static>` on `$this`, and every consumer of a type would have to learn a second meaning of
  a class. The body's `string` and `E0741`'s list cover the programs that need it.
- **Dropping late static binding along with the storage meanings.** `new static()` compiled as `new
  self()` returns the wrong class, and no annotation a program could add keeps the original behaviour.
