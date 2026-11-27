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
`Core\ObjectMap<string, static>`, `?static` and `: static` are all bound at the call. A body that
promises one of these returns `$this`, `new static(...)`, `null`, a call on `self`, `static`,
`parent` or `$this` whose own return binds `static`, an array literal or a `? :` of those, a
fresh `Core` collection with no constructor arguments, which is empty as `[]` is, a variable or a
`readonly` path narrowed by `is static`, or a local declared with a type that binds `static` (`E0741`). A base class that declares
`sortBy(property<static> $field)` once then checks each subclass's own properties at each call, with
no override per subclass. A type that also writes `self` or the declaring class's own name is not
bound, and keeps the declaring class.

A `static` written directly among the parameters of a callable that is a parameter's own type binds
too: `callable(static)`, `callable(static, int)`, `?callable(static)` and `callable(static)|null`
take a function for the class the call names, so `Order::check(fn (Order $o) => …)` checks. Every
deeper position keeps the declaring class: an `array<callable(static)>`, a shape field, a callable
among a callable's parameters, and a written callable in a return type. Inside the body the parameter
is a plain `callable`, so passing it on, storing it or capturing it keeps a plain `callable`'s
run-time check. A call `$rule(...)` written in the method's own body, outside any anonymous function,
and on a parameter the body never assigns, is proven: each argument at a `static` position must be
one E0741's list accepts (`$this`, `new static(...)`, `null` where the position is nullable, or a
call on `self`, `static`, `parent` or `$this` whose return binds `static`). A proven call returns the
signature's type and pays no run-time check. Any other argument there is `E1015`. A function whose
parameter is a class strictly below the one a user method's callable slot takes is still `E0401`,
and its help names `callable(static)`, or the class to call the method on.
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

`static::NAME` is late-bound the same way, and it is the one class-constant read that is not inlined
wherever a class below the declaring one declares `NAME` again (where none does, every called class
reads one value, and `rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`
inlines it): the frame's called class is asked for `NAME` when the read runs, so a default method or an inherited
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
form inside one is refused (`E0834`), and so is every `is` that writes `static`, in a `final` class
too. A local declared there with a type that writes `static` is not recorded. `self::m()` and `parent::m()` inside one call the class the
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
`static`, a `? :` of two of these, or an array literal of them.

**`$o is static` tests the called class at run time**, in a static and in an instance method alike: it
walks `$o`'s class against the descriptor the frame holds, the walk `$o is $cls` pays, so in `Model`
called as `Order::findAll()` it is `true` for an `Order` and `false` for a `Customer`. `$o is ?static`
also passes `null`, and `static|null` is the same type. The true edge keeps the subject's narrowed type
the declaring class's and adds one fact: the value is the called class. A read of it while that
narrowing lives is on E0741's list, for a variable and a `readonly` path, and every write, loop and
merge that drops the narrowing drops the fact with it. A negated guard, `if (!($o is static)) {
continue; }`, proves the rest of the block, as every narrowing does. Only `always_holds` is skipped:
a subject disjoint from the declaring class is disjoint from every subclass, so `is static` on it
still folds to `false`. A `static` deeper in a tested type, `$xs is array<static>` or `$o is
static|int`, is refused (`E1021`): the lowered type cannot tell it from the declaring class, so the
test would quietly ask about the wrong class. Test each element with `is static` instead.

**A local declared with a type that binds `static`**, `?static $found = null;` or `array<static> $out
= [];`, is the called class at every read, because every write to it is held to E0741's list: the
initializer, an assignment, a compound write, an append `$out[] = $o`, an element write and a nested
one. A value off the list is `E1022`, with a label on the value and one on the declaration. Its type
inside the body stays the declaring class's, so it reads, prints and passes on as one. An `inout` pass
of it is `E1022` too, because the callee's parameter is typed as the declaring class and may write any
value of it, and so are a `foreach` over it with an `inout` value and a `foreach` that reuses its name
as the value. A by-value `foreach` over it and removing an element are allowed, and a property write
through it changes the object and not the local. A `property<static>` local stays a key and is not
recorded. A finder that keeps loaded objects in a base class and returns only the called class's is
then written as a loop that tests each with `is static` and appends it to an `array<static>`.

**In a `final` class, `static` is exactly that class**, because no subclass can be the class a call
names. So a body there may return any value its declared type accepts with `static` read as the class:
a local variable, `new Point(...)`, a record a `parse` loaded from a store. And an override in a
`final` class may write the class's own name or `self` wherever the parent or the interface writes
`static` (`E0404`), as `final class Order implements Parses` may declare `parse(...): Order`. A call
site reads the same type either way, since the only class it can name is that one. A class that is not
`final` keeps the list above.

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
  A bound callable parameter costs nothing at run time and nothing per request; at check time it
  keeps one small record per bound parameter for one method body. A proven call does less work than
  a plain `callable` call, since it skips the per-call parameter check. `$o is static` in a class
  that is not `final` costs one descriptor walk per test, the walk `$o is $cls` pays, where it was a
  constant before, and no memory. A narrowed read and a `static` local cost the checker one span per
  read and one small record per local for one body, and nothing at run time.

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
- **Keeping the declaring class in `callable(static)`.** It sends the user to a plain `callable`,
  which adds a run-time check to every call and returns `mixed`.
- **Taking `callable(Order)` where `callable(Model)` is declared, with a run-time check.** A typed
  callable would then no longer promise that every `Model` works. Plain `callable` already says that.
- **Binding `static` in an array of callables, a shape field or a nested callable.** An element read
  out of the array is no longer the parameter the scope keeps a record for, so no call through it can
  be proven. Default until a program needs it.
- **Binding `static` in a callable return type.** E0741 checks a bound return for the called class,
  and a function is not an instance. The return keeps the declaring class.
- **Proving a call through a `callable(static)` parameter inside an anonymous function.** `self::`
  there names the written class, so the proof would need a second list. The call is checked at run
  time.
- **A run-time checked `$o as static`.** `as static` and `as ?static` stay `E0473`
  (`rule:types/conversion`). `is` already narrows a class with no throw path, and a conversion would
  be a second way to write the same test.
- **Folding `$o is static` to the declaring class's test.** It answers `true` for a sibling subclass's
  object, so a finder returned the wrong class with no error.
- **Accepting an `inout` pass of a `static` local.** The callee's parameter is the declaring class,
  so it may write a sibling's object into the local, and nothing checks the write.
- **Dropping late static binding along with the storage meanings.** `new static()` compiled as `new
  self()` returns the wrong class, and no annotation a program could add keeps the original behaviour.
