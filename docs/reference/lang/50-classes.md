---
id: classes
title: Classes, interfaces and objects
summary: declaring a class, its properties, methods and constants; inheritance; interfaces, default methods and `by` delegation; hooks, observers, `Stringable`, `Comparable`; what an object is and what `clone` copies
keywords: class, constructor, __construct, new, public, protected, private, static, self, parent, $this, abstract, final, extends, implements, interface, trait, delegation, by, readonly, lateinit, property hooks, get, set, PropertyObserver, Stringable, __toString, Comparable, compareTo, clone, __clone, is, object, ?->, nullsafe, __get, __set, __call, __callStatic, __invoke, __destruct, anonymous class, const, ::class, class<T>, class reference, new $cls, late static binding
---

# Declaring a class

A class is declared at file scope with `class`, and every member — property, method, constant —
writes its visibility. The constructor is a method named `constructor`; `__construct` is
refused with a diagnostic naming `constructor`. Instances are made with `new`, members are reached with `->`.

```nvs
<?nvs
class Point {
    public int $x;
    public int $y;
    private string $label = "p";

    public function constructor(int $x, int $y) {
        $this->x = $x;
        $this->y = $y;
    }

    public function describe(): string {
        return $this->label . "(" . $this->x . ", " . $this->y . ")";
    }
}

var $p = new Point(1, 2);
$p->x = 5;
echo $p->describe(), "\n";
```
```output
p(5, 2)
```

- `public` is reachable from anywhere, `protected` from the class and the classes that extend it,
  `private` from the declaring class's own bodies only. A read or call from outside that scope is
  a compile error. A `private` constructor is honoured the same way: `new` from outside the class
  is refused.
- There is no implicit `public`. A member written without a visibility is refused:

```nvs error
<?nvs
class Account {
    int $balance = 0;
}
```
```output
a property must declare
```

- A method's parameters and return type are written the way the [types](#lang-types) chapter
  describes; a method that returns nothing declares `void`.
- A wrong argument count at a call, an unknown method, or an undeclared property is a compile
  error — there is no dynamic property and no fallback.

## Constructor promotion

A visibility keyword on a constructor parameter declares a property of that name and type and
assigns the argument to it. A default on the parameter is a default for the argument. The
keyword is refused on any other method's parameter.

```nvs
<?nvs
class Person {
    public function constructor(public string $name, private int $age = 30) {}

    public function age(): int {
        return $this->age;
    }
}

var $a = new Person("Ada");
var $b = new Person("Bob", 41);
echo $a->name, " ", $a->age(), "; ", $b->name, " ", $b->age(), "\n";
```
```output
Ada 30; Bob 41
```

## A class with no constructor

A class need not declare a constructor when every property has a default. `new` on such a class
takes no arguments — `new Counter(1)` is a compile error saying the class declares no
`constructor`. A subclass that declares no constructor inherits its parent's.

```nvs
<?nvs
class Counter {
    public int $n = 0;
}

var $c = new Counter();
$c->n = $c->n + 1;
echo $c->n, "\n";
```
```output
1
```

# Properties

## Every property is assigned before the constructor returns

A property with no default must be assigned on every path out of the constructor. A path that
leaves one unassigned is a compile error, and so is declaring a property with no default in a
class that has no constructor.
<!-- src: `rule:classes/definite-property-initialization` -->

A `return` inside a constructor carries no value: a bare `return;` may leave early once every
property is assigned on that path, and `return $value;` does not compile — the object under
construction is the result and nothing else can be.
<!-- src: `rule:classes/a-constructor-return-carries-no-value` -->

```nvs error
<?nvs
class Box {
    public int $n;

    public function constructor(bool $big) {
        if ($big) {
            $this->n = 10;
        }
    }
}
```
```output
not assigned on every path
```

## Defaults

A property default is a compile-time constant of the declared type: a `bool`, `int`, `uint`,
`float` or `string` literal (optionally negated), `[]`, an enum case, or a class constant
(`self::NAME` or `Class::NAME`). `null` is one of them wherever the declared type admits it, so a
`?T` property may be defaulted rather than assigned in the constructor. Anything else is refused.

```nvs
<?nvs
enum Mode { Off, On }

class Config {
    public const int DEFAULT_SIZE = 8;
    public Mode $mode = Mode::On;
    public int $size = self::DEFAULT_SIZE;
    public array<int> $ports = [];
    public ?string $label = null;
}

var $c = new Config();
echo $c->mode as int, " ", $c->size, " ", Core\Arr::count($c->ports), " ", $c->label ?? "none", "\n";
```
```output
1 8 0 none
```

```nvs error
<?nvs
class Box {
    public string $label = null;
}
```
```output
a property default must be a
```

## `lateinit`: a property written after construction

`lateinit` on a class- or interface-typed property exempts it from the constructor rule. A read
before any write throws; a read the compiler can see is unreachable-before-write is a compile
error. `lateinit` is refused on a scalar or array property — give those a default.
<!-- src: `rule:classes/lateinit` -->

```nvs
<?nvs
class Logger {
    public function name(): string {
        return "log";
    }
}

class Widget {
    public lateinit Logger $logger;

    public function attach(Logger $l): void {
        $this->logger = $l;
    }
}

var $w = new Widget();
try {
    echo $w->logger->name(), "\n";
} catch (Throwable $e) {
    echo "threw: ", $e->message, "\n";
}
$w->attach(new Logger());
echo $w->logger->name(), "\n";
```
```output
threw: `Widget`'s property `$logger` is read before it is written
log
```

```nvs error
<?nvs
class Box {
    public lateinit int $n;

    public function constructor() {}
}
```
```output
only allowed on a class- or interface-typed property
```

## `readonly`

`readonly` promises a property is assigned exactly once, while the object is being built. The
declaring class's own `constructor` is the one place that assignment may happen — a promoted
parameter carries the modifier the same way a declaration does. A `readonly` property therefore
declares no default: a value already known at the declaration is a `const`, not a property.
<!-- src: `rule:classes/a-readonly-property-declares-no-default` -->

```nvs
<?nvs
class Id {
    public function constructor(public readonly int $value) {}
}

var $id = new Id(1);
echo $id->value, "\n";
```
```output
1
```

Every other write is refused where it is written, whichever spelling it uses, and that includes a
write from another method of the same class: by the time one runs, the object is built.

```nvs error
<?nvs
class Id {
    public function constructor(public readonly int $value) {}
}

var $id = new Id(1);
$id->value = 2;
```
```output
this write happens after construction
```

## `static` properties

A `static` property is one slot per request, owned by the class that declares it — a subclass
names the same slot. It must have an initializer, or be nullable so that it starts each request
at `null`. It is reached as `self::$name` inside the class and `Class::$name` anywhere;
`static::$name` is refused, because the slot is resolved while compiling. Every compound
assignment and `++`/`--` works on it.
<!-- src: `rule:statements/static-is-a-member-modifier` -->

```nvs
<?nvs
class Registry {
    public static int $count = 0;
    public static ?string $last;

    public static function note(string $name): void {
        self::$count = self::$count + 1;
        self::$last = $name;
    }
}

Registry::note("a");
Registry::note("b");
Registry::$count = Registry::$count + 10;
echo Registry::$count, " ", Registry::$last ?? "none", "\n";
```
```output
12 b
```

```nvs error
<?nvs
class Counter {
    public static int $n = 0;

    public static function read(): int {
        return static::$n;
    }
}
```
```output
does not resolve a static property
```

# Methods, `self`, `static` and `parent`

- `$this` is the receiver inside an instance method. An anonymous function written inside a method
  keeps the `$this` it was made on and reaches that class's `private` members.
- A `static` method is called as `Class::method()`, or `self::method()` from inside the class.
  It has no `$this`.
- `self` names the declaring class: `new self()`, `self::method()`, `self::CONST`, and as a
  return type. `static` names the class the call was made on — late static binding — for
  `new static()`, `static::method()`, `static::CONST` and the return type `static`. A body
  declared `: static` must return `$this`, `new static(...)` or a `static::` call; returning
  `new self()` or a named class there is a compile error.
- `static::` is not allowed inside an anonymous function body (`E0834`). Read the value into a
  variable before the anonymous function and use that variable.
- `parent::method()` calls the parent's version of an overridden method, and
  `parent::constructor(...)` its constructor.

```nvs
<?nvs
class Registry {
    public static function tag(): string {
        return "base";
    }

    public static function make(): static {
        return new static();
    }

    public function label(): string {
        return static::tag() . "/" . self::tag();
    }
}

class LeafRegistry extends Registry {
    public static function tag(): string {
        return "leaf";
    }
}

echo Registry::make()->label(), "\n";
echo LeafRegistry::make()->label(), "\n";
LeafRegistry $typed = LeafRegistry::make();
echo ($typed is LeafRegistry) as string, "\n";
```
```output
base/base
leaf/base
1
```

A method returning `self` or `static` chains, and an anonymous function made inside a method keeps its receiver:

```nvs
<?nvs
class Acc {
    private int $n = 0;

    public function bump(): self {
        $this->n = $this->n + 1;
        return $this;
    }

    public function adder(): callable {
        return fn(int $k): int => $this->n + $k;
    }
}

var $a = new Acc();
var $f = $a->adder();
$a->bump()->bump();
echo $f(3) as int, "\n";
```
```output
5
```

# Constants and `::class`

A class constant is `public const int NAME = …;`, and like every other binding it writes its type
(`E0246`). It is reached as `self::NAME` inside the class and `Class::NAME` anywhere. Both give the
value of the class that declares it. `static::NAME` gives the value of the class the call was made
on, so a subclass or an implementor that redeclares the constant is seen by an inherited method or
an interface default method. A redeclaration keeps the constant's type (`E0833`). `static::NAME`
works for `string`, `int`, `uint`, `bool` and `float` constants (`E0832` for an `array`), and not in
a default value or an attribute (`E0831`). `Class::class` is the
class's name as a string, and it stays a `string` — the type that holds a class itself is `class<T>`,
and the three sites that take one are below.

`::class` answers **the class the value is**. `Class::class`, `self::class` and `parent::class` name a
class the compiler resolves, so they are folded where they are written. `static::class` and
`$obj::class` are not: the first is the class the call was made on, the second the class the receiver
was actually allocated from, so both are read at run time and a variable declared as a base class
reports the subclass it holds. The operand has to carry a class — an object does; a `mixed` or a `?T`
is `E0702` until it is narrowed, and a `class<T>` converts with `as string` instead
(`rule:types/class-constant`).

```nvs
class Base {
    public static function called(): string { return static::class; }
}
class Leaf extends Base {}

echo Leaf::called(), "\n";       // Leaf — the class the call was made on
echo Base::called(), "\n";       // Base

Base $b = new Leaf();
echo $b::class, "\n";            // Leaf — the class it *is*, not the declared one
```

A constant's value is **inlined at every read** — there is no storage a read loads it from — so the
value has to have a compile-time form. A value written directly in the code has one, and so does
an `array<T>` literal of such values: each
element is placed in the declared element type, so `array<float> RATES = [1, 2.5]` holds two floats.
Because it is inlined, every read builds its own array, and writing to one is invisible to the next
read. What has no form yet is a *named* constant reaching into another — another class's `const`, an
enum case, or `Class::class` — written as the value or nested in the array; a read of one is `E0792`
at the read, while declaring it and never naming it is fine.

```nvs
<?nvs
class Limits {
    public const int MAX = 3;
    public const string NAME = "limits";
    public const array<int> STEPS = [1, 2, 3];

    public static function twice(): int {
        return self::MAX * 2;
    }
}

echo Limits::MAX, " ", Limits::NAME, " ", Limits::twice(), " ", Limits::class, "\n";
echo Limits::STEPS[0], Limits::STEPS[2], "\n";
```
```output
3 limits 6 Limits
13
```

# Inheritance

`extends` names one parent. A subclass inherits every member, may override a method, and reaches
the parent's version through `parent::`. A subclass constructor must call
`parent::constructor(...)` on every path when the parent declares a constructor; a parent with
no constructor demands nothing.

```nvs
<?nvs
class Animal {
    public function constructor(protected string $name) {}

    public function describe(): string {
        return $this->name . " makes a sound";
    }
}

class Dog extends Animal {
    public function constructor(string $name, private int $tricks) {
        parent::constructor($name);
    }

    public function describe(): string {
        return parent::describe() . ", knows " . $this->tricks . " tricks";
    }
}

Animal $a = new Dog("Rex", 3);
echo $a->describe(), "\n";
```
```output
Rex makes a sound, knows 3 tricks
```

```nvs error
<?nvs
class Base {
    public function constructor(public int $a) {}
}

class Child extends Base {
    public function constructor(public int $b) {}
}
```
```output
never calls `parent::constructor(...)`
```

## `abstract`

An `abstract` class declares `abstract` methods without a body; a concrete subclass must declare
every one of them, or it is a compile error. A concrete method on the abstract class may call an
abstract one through `$this`.

An abstract class has no instances: `new` on one is refused (`E0785`), as is `new` on an interface —
instantiate a subclass instead. `new static()` inside an abstract class is *not* refused, because
late static binding resolves it to whichever concrete subclass the call arrived through. The rule's
other half is at the declaration: a class that is not `abstract` may not leave a method without a
body (`E0786`), because a call to one has no compiled function to reach.

```nvs
<?nvs
abstract class Shape {
    public abstract function area(): int;

    public function describe(): string {
        return "area=" . $this->area();
    }
}

class Square extends Shape {
    public function constructor(public int $side) {}

    public function area(): int {
        return $this->side * $this->side;
    }
}

array<Shape> $all = [new Square(3), new Square(2)];
foreach ($all as Shape $s) {
    echo $s->describe(), ";";
}
echo "\n";
```
```output
area=9;area=4;
```

```nvs error
<?nvs
abstract class Shape {
    public abstract function area(): int;
}

class Blob extends Shape {
}
```
```output
does not declare `area`, which `Shape` requires
```

An `abstract static` method has no body either, so a call to it must reach a subclass. Inside a
method, `self::title()` and `static::title()` call the class the method was called on, so
`Blog::heading()` runs `Blog`'s `title`. A call that names the class itself is a compile error
(`E0835`): `Page::title()`, where `Page` gives `title` no body, and `self::title()` inside an anonymous
function, which calls the class the function is written in. Call the method on a class that is not
`abstract`, or call it outside the anonymous function and use the result inside it.

```nvs
<?nvs
abstract class Page {
    public abstract static function title(): string;

    public static function heading(): string {
        return "Title: " . self::title();
    }
}

final class Blog extends Page {
    public static function title(): string {
        return "Blog";
    }
}

echo Blog::heading(), "\n";
```
```output
Title: Blog
```

```nvs error
<?nvs
abstract class Page {
    public abstract static function title(): string;
}

echo Page::title(), "\n";
```
```output
is `abstract`, so it has no body to call
```

## `final`

`final` says a declaration is not specialized further, and both halves are refused where the
offending declaration is written: no class may name a `final` class as its superclass, and no class
may redeclare a method an ancestor declared `final`. A `final` method beside an ordinary one
constrains only itself — the sibling is overridden as usual.

```nvs error
<?nvs
final class Sealed {}

class Widened extends Sealed {}
```
```output
so no class extends it
```

To build on a sealed class, hold one in a property and forward to it — `implements … by $field`
writes the forwards for you.

## `is`

`$x is T` is true for the object's own class, every ancestor, and every interface any of
them implements; false for anything else, and false when `$x` is `null`. Inside the `if` it
guards, a value declared `object` or at a base type is narrowed to `T`. The right-hand side is a
class name written out, or a `class<T>` value (below); a `string` there is refused whatever it
holds.

```nvs
<?nvs
interface Marks {
    public function mark(): string;
}

class Base implements Marks {
    public function mark(): string {
        return "base";
    }
}

class Leaf extends Base {}

class Other {}

var $leaf = new Leaf();
echo ($leaf is Leaf) as string, ($leaf is Base) as string, ($leaf is Marks) as string, "|", ($leaf is Other) as string, "|\n";

object $o = new Other();
if ($o is Marks) {
    echo $o->mark(), "\n";
} else {
    echo "not marked\n";
}
```
```output
111||
not marked
```

## A class chosen at run time: `class<T>`

A `class<T>` value is a class rather than an instance of one, and three sites take it:
`new $cls(...)`, `$cls::f(...)` and `$x is $cls`. All three take that value **and nothing
else** — a `string` holding a class name is refused at every one of them, with the `as` that would
produce one named in the help — and `$obj->$name` is not on the list and never will be, because a
class reference answers *which class* and never *which member*. The type, and the `as` that is its
only source, are in [the type chapter](20-types.md).

A static call through one is virtual: the implementor's body wins where it declares one, and
`static::` inside that body sees the implementor rather than the bound. A `new` through one is typed
against **`T`'s** constructor, since that is the only signature the site can see — so the `new` is
refused, where it is written, when any implementor of `T` declares a constructor that could not take
the arguments there. Narrow the reference (`as class<Invoice>`) and instantiate that, or give the
subclass a compatible constructor.

```nvs
<?nvs
class Report {
    public function constructor(public string $title) {}

    public static function kind(): string {
        return "report";
    }

    public function render(): string {
        return static::kind() . ":" . $this->title;
    }
}

class Invoice extends Report {
    public static function kind(): string {
        return "invoice";
    }
}

string $wanted = "Invoice";
class<Report> $cls = $wanted as class<Report>;
Report $r = new $cls("March");
echo $r->render(), " ", $cls::kind(), " ", ($r is $cls) as string, "\n";
```
```output
invoice:March invoice 1
```

```nvs error
<?nvs
class Formatter {
    public function constructor(public string $prefix) {}
}

class Strict extends Formatter {
    public function constructor(string $prefix, public int $width) {
        parent::constructor($prefix);
    }
}

class<Formatter> $cls = Formatter::class as class<Formatter>;
Formatter $made = new $cls("p: ");
echo $made->prefix, "\n";
```
```output
`Strict::constructor` is not compatible with `Formatter::constructor`
```

# Interfaces

Six interfaces are declared by the compiler for every program and need no `use`; the ones with a
type parameter are implemented at a concrete type (`implements Iterable<int>`) and are the only
generic names a class may implement. `Parses` is what a class implements to be built from a piece
of text, which is what lets a route capture, a `#[Core\Query]` parameter, a command argument or a
command option be declared at that class and arrive as an object:

<!-- generated: interfaces -->

An `interface` declares methods a class must provide. It may also carry:

- **Default methods** — a `public` method with a body, inherited by every implementor and
  overridable like any other method. An implementor that overrides one reaches the default as
  `Interface::method()`; a subclass reaches its parent's version with `parent::`.
- **Private helpers** — a `private` method with a body, callable only from the interface's own
  method bodies.
- **Constants**, written with a type: `public const int MAX = 3;`. An interface constant with no
  type is not usable — write the type.
- **`static` methods** with a body, reached as `Interface::method()` or `Implementor::method()`,
  and overridable by an implementor.

An interface has no properties. `public string $path;` inside one is an error (`E0254`). A value
every implementor must supply is a method. A value that is fixed per implementor is a typed
constant: the implementor overrides it, and a default method reads it as `static::NAME`.

A class implements any number of interfaces, comma-separated, and an interface may `extends`
another. A class missing a required method is a compile error naming the method and the
interface.
<!-- src: `rule:classes/no-traits` -->

```nvs
<?nvs
interface Greets {
    public const int MAX_LEN = 20;

    public function name(): string;

    public function greet(): string {
        return $this->prefix() . $this->name() . "!";
    }

    private function prefix(): string {
        return "Hello, ";
    }

    public static function kind(): string {
        return "greeter";
    }
}

interface Counts {
    public function count(): int;
}

class Person implements Greets, Counts {
    public function constructor(private string $who) {}

    public function name(): string {
        return $this->who;
    }

    public function count(): int {
        return 1;
    }
}

class Loud extends Person {
    public function greet(): string {
        return Core\Str::upper(parent::greet());
    }
}

echo (new Person("ada"))->greet(), " ", Greets::MAX_LEN, " ", Person::kind(), "\n";
echo (new Loud("bob"))->greet(), "\n";
```
```output
Hello, ada! 20 greeter
HELLO, BOB!
```

```nvs
<?nvs
interface Named {
    public function name(): string;
}

interface Titled extends Named {
    public function title(): string;
}

class Doc implements Titled {
    public function name(): string {
        return "doc";
    }

    public function title(): string {
        return "Doc";
    }
}

Named $n = new Doc();
echo $n->name(), " ", ($n is Titled) as string, "\n";
```
```output
doc 1
```

```nvs error
<?nvs
interface Counts {
    public function count(): int;
}

class Empty implements Counts {
}
```
```output
does not declare `count`, which `Counts` requires
```

## Delegation: `implements I by $field`

`implements I by $field` forwards every method `I` requires to the object held in the property
`$field`. The field must be a declared (or promoted) property whose type is a non-nullable class
or interface that satisfies `I`. The forward is a real method of the class, so a receiver typed
as the interface reaches it too. A method the class writes itself wins over the forward. Two
interfaces may delegate to one field, and two fields may each serve a different interface in the
same `implements` list.

```nvs
<?nvs
interface Greets {
    public function greet(string $who): string;
    public function wave(): string;
}

class Polite implements Greets {
    public function greet(string $who): string {
        return "Hello, " . $who;
    }

    public function wave(): string {
        return "*wave*";
    }
}

class Desk implements Greets by $voice {
    public function constructor(private Greets $voice) {}

    public function wave(): string {
        return "*nod*";
    }
}

var $d = new Desk(new Polite());
echo $d->greet("Ada"), " ", $d->wave(), "\n";
Greets $g = $d;
echo $g->greet("Bo"), "\n";
```
```output
Hello, Ada *nod*
Hello, Bo
```

- The field may be `lateinit`; a delegated call before it is written throws the same
  read-before-write error a direct read would.
- A forward cannot express a `static` member, a variadic parameter or an `inout` parameter: each
  is a compile error at the `by` clause, and the class must declare that method itself.
- A nullable, scalar or unrelated field is refused:

```nvs error
<?nvs
interface Greets {
    public function greet(): string;
}

class Maybe implements Greets by $voice {
    private ?Greets $voice;

    public function constructor(?Greets $voice) {
        $this->voice = $voice;
    }
}
```
```output
cannot satisfy `Greets`
```

## There is no `trait`

`trait`, class-body `use T;` and `insteadof` are refused. Shared behaviour is an interface default
method; shared state is `by` delegation.

```nvs error
<?nvs
trait Greets {
    public function hi(): string {
        return "hi";
    }
}
```
```output
traits do not exist
```

# Property hooks

A property may declare a `get` and/or a `set` hook. Every read runs `get`, every write runs
`set` — including reads and writes made by the constructor, by an interpolation, and by a
subclass, which inherits the hooks. Inside its own hooks `$this->name` is the backing slot. A
`set` hook that throws is an ordinary throw from the assignment, and the slot keeps its value.
<!-- src: `rule:classes/property-observer` -->

```nvs
<?nvs
class Temperature {
    public int $celsius {
        get => $this->celsius;
        set (int $v) {
            if ($v < -273) {
                throw new LogicError("below absolute zero");
            }
            $this->celsius = $v;
        }
    }

    public int $doubled {
        get => $this->celsius * 2;
    }

    public function constructor(int $start) {
        $this->celsius = $start;
    }
}

var $t = new Temperature(20);
echo $t->celsius, " ", $t->doubled, " in a string: $t->doubled\n";
try {
    $t->celsius = -300;
} catch (LogicError $e) {
    echo "refused: ", $e->message, "\n";
}
echo $t->celsius, "\n";
```
```output
20 40 in a string: 40
refused: below absolute zero
20
```

- `get => expr;` is short for `get { return expr; }`; `set (T $v) { … }` takes the value being
  written. Either hook may stand alone.
- A property with only a `get` hook is a computed property, and **only the declaring class writes
  it**. `$this->name = …` in that class's own bodies stores into the backing slot its `get` hook
  reads, which is how such a property holds anything at all; a write from anywhere else is refused
  (`E0787`) rather than stored where no read would find it. Add a `set` hook to accept one.
- An array element cannot be written through a hooked property (`$b->rows[0] = "x"`): read the
  array into a local, write the element there, and assign the local back.

```nvs error
<?nvs
class Box {
    public array<int> $xs {
        get => $this->xs;
        set (array<int> $v) {
            $this->xs = $v;
        }
    }

    public function constructor() {
        $this->xs = [];
    }
}

var $b = new Box();
$b->xs[0] = 1;
```
```output
cannot be written through the hooked property
```

# `PropertyObserver`

A class implementing the global interface `PropertyObserver` declares
`onPropertyGet(string $name, mixed $value): void` and `onPropertySet(string $name, mixed $value): void`,
and is told of every read and write of every property it declares — hooked or not, in the
constructor or later, in a subclass too — after the value has settled. The observer cannot
change the value; a throwing `onPropertySet` fails the write. A class that does not implement
the interface pays nothing.

```nvs
<?nvs
class Account implements PropertyObserver {
    public int $balance = 0;

    public function onPropertyGet(string $name, mixed $value): void {
        echo "get ", $name, "\n";
    }

    public function onPropertySet(string $name, mixed $value): void {
        echo "set ", $name, "=", $value, "\n";
    }
}

var $a = new Account();
$a->balance = 7;
int $seen = $a->balance;
echo $seen, "\n";
```
```output
set balance=7
get balance
7
```

# `Stringable`

An object becomes a string only through the global interface `Stringable`, whose one method is
`toString(): string`. It is called by `echo`, by `.`, by interpolation and by `as string`,
dispatching on the object's runtime class. An object of a class that does not implement it is
refused at every one of those sites. `__toString` is refused as a method name.
<!-- src: `rule:classes/no-magic-methods` -->

```nvs
<?nvs
class Money implements Stringable {
    public function constructor(private int $cents) {}

    public function toString(): string {
        return ($this->cents / 100) . " EUR";
    }
}

var $m = new Money(250);
echo $m, "|", "total: " . $m, "|", "in {$m}", "|", $m as string, "\n";
```
```output
2.5 EUR|total: 2.5 EUR|in 2.5 EUR|2.5 EUR
```

```nvs error
<?nvs
class Point {
    public int $x = 1;
}

var $p = new Point();
echo "point $p\n";
```
```output
does not implement `Stringable`
```

# `Comparable`

`<`, `<=`, `>`, `>=` and `<=>` between two objects require the global interface `Comparable`,
whose one method is `compareTo(self $other): int`; the sign of its answer drives every operator,
and `<=>` answers it unchanged. Without the interface, ordering two objects is a compile error —
there is no property-by-property comparison. `==` is unaffected: it stays identity.
<!-- src: `rule:classes/comparable` -->

```nvs
<?nvs
class Version implements Comparable {
    public function constructor(public int $n) {}

    public function compareTo(self $other): int {
        return $this->n <=> $other->n;
    }
}

var $a = new Version(2);
var $b = new Version(5);
var $c = new Version(2);
echo ($a < $b) as string, ";", ($a > $b) as string, ";", ($a <= $c) as string, ";", ($a >= $c) as string, "\n";
echo $a <=> $b, ";", $b <=> $a, ";", $a <=> $c, ";", ($a == $c) as string, "|\n";
```
```output
1;;1;1
-1;1;0;|
```

```nvs error
<?nvs
class Point {
    public int $x = 1;
}

var $a = new Point();
var $b = new Point();
if ($a < $b) {
    echo "less\n";
}
```
```output
does not implement `Comparable`
```

# `Parses`

A class implementing the global interface `Parses` is built from a piece of text. Its one required
member is `public static function parse(tainted string $s): static`, which answers an instance of the
called class or throws. The text is `tainted` because the text at every site that builds one arrived
from outside the process, and the object it answers carries no qualifier at all — `tainted` is a
property of `string` and `bytes` and never of a class, so a field keeping the text keeps it
`tainted`.
<!-- src: `rule:security/tainted-qualifier` -->

That contract is what a binding site asks for rather than naming one class: a route capture, a
`#[Core\Query]` parameter, a command argument and a command option are each declared at any class
carrying it, and a parameter typed at a class without it is a compile error naming `Parses` as the
fix. A path capture is the one site that reads two ways. The router converts the types it reads
itself — `int`, `uint`, `decimal`, `Core\Uuid` and a closed set — while it matches, so a segment
those refuse is no match and ends in a `404`; a capture at any other `Parses` class matches on shape
and runs `parse` where the match crosses into the program, so a segment that class refuses is a
`400`.
<!-- src: `rule:security/route-capture-is-laundered-by-its-type` -->

```nvs
<?nvs
class Slug implements Parses {
    public function constructor(public tainted string $text) {}

    public static function parse(tainted string $s): static {
        if ($s != Core\Str::lower($s)) {
            throw new ParseError("a slug is written in lower case");
        }
        return new static($s);
    }
}

class Posts {
    #[Core\Route(path: "/posts/{slug}", method: Core\Http\Method::Get, name: "Posts::show")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function show(Slug $slug, #[Core\Query] Slug $tag): string {
        return "one post";
    }
}

echo Slug::parse("hello-world")->text, "\n";
try {
    echo Slug::parse("Hello-World")->text, "\n";
} catch (ParseError $refusal) {
    echo $refusal->message, "\n";
}
```
```output
hello-world
a slug is written in lower case
```

```nvs error
<?nvs
class Tag {
    public string $text = "";
}

class Posts {
    #[Core\Route(path: "/posts/{tag}", method: Core\Http\Method::Get, name: "Posts::show")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function show(Tag $tag): string {
        return "one post";
    }
}
```
```output
or a class implementing `Parses`
```

# Objects are handles

A variable, a property, an array element or a parameter holding an object holds a reference to
it: assigning it, passing it, or storing it never copies the object, and a write through any
handle is seen through every other. `==` on two objects is identity — true only for the same
object. `clone` makes a new object of the same class (a subclass clone keeps its class) with a
copy of every property: an array property is copied, an object property is shared, and no
method runs. `__clone` does not exist.
<!-- src: `rule:classes/two-copy-depths` -->

```nvs
<?nvs
class Tag {
    public function constructor(public string $text) {}
}

class Note {
    public array<string> $lines = [];

    public function constructor(public Tag $tag) {}
}

class Edit {
    public static function retitle(Tag $t): void {
        $t->text = "changed";
    }
}

var $a = new Note(new Tag("red"));
$a->lines[] = "one";
var $same = $a;
var $copy = clone $a;
$copy->lines[] = "two";
Edit::retitle($copy->tag);
echo ($a == $same) as string, "|", ($a == $copy) as string, "|", Core\Arr::count($a->lines), Core\Arr::count($copy->lines), "|", $a->tag->text, "\n";
```
```output
1||12|changed
```

# Nullable objects and `?->`

`?Foo` holds a `Foo` or `null`. `->` on a value that may be `null` is a compile error; write
`?->`, which answers `null` without evaluating the member or its arguments, or test the value
first — inside `if ($x != null)` it is no longer nullable.

```nvs
<?nvs
class Box {
    public function constructor(public string $label) {}

    public function inner(): ?Box {
        return null;
    }
}

?Box $some = new Box("here");
?Box $none = null;
echo $some?->label ?? "gone", "|", $none?->label ?? "gone", "|", $some?->inner()?->label ?? "gone", "\n";
if ($some != null) {
    echo $some->label, "\n";
}
```
```output
here|gone|gone
here
```

```nvs error
<?nvs
class Box {
    public int $n = 1;
}

?Box $none = null;
echo $none->n, "\n";
```
```output
so `->` cannot reach a member of it
```

# `object`: the top of every class type

`object` holds any object and names no class. A method call through it is a compile error until
`is` or `as` narrows it. A property read through it compiles and is resolved at run
time, throwing when the object has no such property.

```nvs
<?nvs
class Tag {
    public function constructor(public string $text) {}

    public function shout(): string {
        return Core\Str::upper($this->text);
    }
}

class Plain {}

class Pick {
    public static function any(bool $tag): object {
        if ($tag) {
            return new Tag("x");
        }
        return new Plain();
    }
}

object $o = Pick::any(true);
echo $o->text, "\n";
if ($o is Tag) {
    echo $o->shout(), "\n";
}
Tag $t = $o as Tag;
echo $t->shout(), "\n";
```
```output
x
X
X
```

`Core\Program::implementing<I>()` builds one instance of every non-abstract class implementing
`I` that the program's autoload roots can reach — the [programs](#lang-programs) chapter covers
the roots, the attributes chapter the metadata usually read off them.

# What a class cannot declare

- **Magic methods.** No identifier may start with `_`, so `__get`, `__set`, `__call`,
  `__callStatic`, `__invoke`, `__clone`, `__destruct`, `__isset`, `__unset`, `__debugInfo`,
  `__set_state` and `__toString` are all refused where written. There is no property
  interception beyond hooks and `PropertyObserver`, no callable objects (a `callable` comes from an anonymous function or a method reference),
  and no destructor.
- **Anonymous classes.** `new class { … }` does not exist; declare a named class.
- **Nested classes.** A `class` inside a class body is refused; every class is declared at file
  scope.

```nvs error
<?nvs
class Legacy {
    public function __get(string $name): mixed {
        return 1;
    }
}
```
```output
method names must be camelCase
```

```nvs error
<?nvs
class Outer {
    public class Inner {
    }
}
```
```output
expected a class member
```
