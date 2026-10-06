---
id: enums
title: Enums
summary: `enum` declares a closed set of named integers — how cases get their values, how a case converts to and from its integer, and what an enum does not have
keywords: enum, case, backed enum, BackedEnum, UnitEnum, int enum, uint enum, string enum, ->name, ->value, cases(), from(), tryFrom(), as int, as E, enum match, enum switch, enum case type, closed set, Core\Order
---

# Declaring an enum

`enum` declares a new named integer type with a fixed set of cases. A case is a compile-time
constant of that type, never an object. The body is a comma list of `Name` or `Name = integer`
— there is no `case` keyword and no `;`. Case names are `PascalCase`.
<!-- src: `rule:enums/closed-integer-type` -->

```nvs
<?nvs
enum Rank { Bronze, Silver, Gold }
enum Signal: int { Stop = 10, Go, Wait = 30, After }
enum Dir { Back = -2, Left, Still, Right }
enum Mask: uint { None = 0, All = 18446744073709551615 }

echo Rank::Bronze as int, ",", Rank::Gold as int, "\n";
echo Signal::Stop as int, ",", Signal::Go as int, ",", Signal::Wait as int, ",", Signal::After as int, "\n";
echo Dir::Back as int, ",", Dir::Left as int, ",", Dir::Still as int, ",", Dir::Right as int, "\n";
echo Mask::All as uint, "\n";
```
```output
0,2
10,11,30,31
-2,-1,0,1
18446744073709551615
```

- The first case with no value is `0`; every later case with no value is the previous case's
  value plus one, whether that previous value was written or counted. A value may be negative or
  zero. Two cases may share a value.
- `enum E: int` and `enum E: uint` name the backing type; with no `: T` it is `int`. Nothing
  else backs an enum — `enum E: string` is refused, and so is a case with a non-integer value.
- An enum body holds cases and nothing else: a method, a constant or an `implements` clause is
  refused. Put behaviour on a class that takes the enum.

```nvs error
<?nvs
enum Suit: string { Hearts = "h", Spades = "s" }
```
```output
cannot be backed by `string`
```

```nvs error
<?nvs
enum Suit { case Hearts; case Spades; }
```
```output
is not written with `case`
```

```nvs error
<?nvs
enum Suit {
    Hearts,
    Spades,
    const int COUNT = 2;
}
```
```output
an enum declares only cases
```

# A case is its integer

`E::Case as int` (or `as uint`, for a `uint`-backed enum) is the case's value, and is the only
conversion out of an enum: `as string`, `echo E::Case` and interpolation are refused — convert
to the backing type first, `($e as int) as string`. A `uint`-backed case converts with `as uint`
only; `as int` on it is refused, and the same the other way round.

```nvs error
<?nvs
enum Rank { Bronze, Silver }
echo Rank::Silver as string, "\n";
```
```output
cannot be converted to `string`
```

# From an integer back to a case

`$n as E` converts an integer to the case that names it and throws a `RuntimeError` naming
every case when none does. `$n as ?E` answers `null` instead of throwing. The operand is
converted to the backing type first, so a string of digits reaches a case through `as int`.

```nvs
<?nvs
enum Mode { Read, Write, Exec = 4 }

class Gate {
    public static function of(int $n): string {
        try {
            Mode $m = $n as Mode;
            return "case " . ($m as int);
        } catch (RuntimeError $e) {
            return "threw: " . $e->message;
        }
    }
}

echo Gate::of(1), "\n", Gate::of(2), "\n";
?Mode $maybe = 9 as ?Mode;
echo $maybe == null ? "none" : "some", "\n";
Mode $parsed = ("4" as int) as Mode;
echo $parsed == Mode::Exec ? "exec" : "other", "\n";
```
```output
case 1
threw: `2` is not one of `Mode::Read`, `Mode::Write`, `Mode::Exec`
none
exec
```

# Comparing cases

`==` and `!=` compare two cases of one enum by value. A case against a plain integer, or a case
of one enum against a case of another, is a compile error — the types are disjoint. Ordering
(`<`, `>`, `<=>`) and arithmetic go through `as int`.

```nvs
<?nvs
enum Rank { Bronze, Silver, Gold }

Rank $held = Rank::Silver;
echo ($held == Rank::Silver) as string, "|", ($held != Rank::Gold) as string, "|", ($held == Rank::Gold) as string, "|\n";
if (($held as int) < (Rank::Gold as int)) {
    echo "below gold\n";
}
```
```output
1|1||
below gold
```

```nvs error
<?nvs
enum Rank { Bronze, Silver }
echo (Rank::Bronze == 0) as string, "\n";
```
```output
are disjoint
```

# `match` and `switch` over an enum

Both compare the subject with each case label in source order. A `match` with no matching arm
and no `default` throws, as over any subject; a `switch` falls through without `break`.

```nvs
<?nvs
enum Mode { Read, Write, Exec }

class Name {
    public static function of(Mode $m): string {
        return match ($m) {
            Mode::Read => "read",
            Mode::Write => "write",
            default => "other",
        };
    }
}

echo Name::of(Mode::Write), " ", Name::of(Mode::Exec), "\n";

Mode $m = Mode::Exec;
switch ($m) {
    case Mode::Read:
        echo "sw=read\n";
        break;
    default:
        echo "sw=other\n";
}

try {
    string $u = match ($m) { Mode::Read => "read" };
} catch (Throwable $t) {
    echo "threw: ", $t->message, "\n";
}
```
```output
write other
sw=other
threw: no `match` arm matched the subject
```

# An enum as a type

An enum's name is a type everywhere a type is written: a property, a parameter, a return type,
an `array<E>` element, a `foreach` binding, a generator's `Iterator<E>`. A case is not an array
key — index with `$e as int`.

```nvs
<?nvs
enum Suit { Clubs, Hearts }

class Card {
    public function constructor(public Suit $suit, public int $rank) {}
}

class Deck {
    public static function suits(): Iterator<Suit> {
        yield Suit::Hearts;
        yield Suit::Clubs;
    }
}

array<Card> $hand = [new Card(Suit::Hearts, 10), new Card(Suit::Clubs, 2)];
foreach ($hand as Card $c) {
    echo $c->suit as int, ":", $c->rank, ";";
}
echo "\n";

array<string> $names = [];
$names[Suit::Clubs as int] = "clubs";
$names[Suit::Hearts as int] = "hearts";
foreach (Deck::suits() as Suit $s) {
    echo $names[$s as int], ";";
}
echo "\n";
```
```output
1:10;0:2;
hearts;clubs;
```

# A union of cases is a narrower type

`E::A|E::B` written as a type accepts only those cases: a case written in the code outside the set, or a
value typed as the whole enum, is a compile error. A whole-enum value enters the set through
`as E::A|E::B`, which throws when the value is not one of them. A single `==` narrows its subject to
that one case's type; two comparisons joined by `||` do not narrow to the pair, so a set of two is
entered by the conversion alone.
<!-- src: `rule:types/single-value-types` -->

```nvs
<?nvs
enum Mode { Read, Write, Exec }

class Open {
    public static function file(Mode::Read|Mode::Write $m): string {
        return "opened " . ($m as int);
    }
}

echo Open::file(Mode::Read), "\n";
Mode $m = Mode::Write;
echo Open::file($m as Mode::Read|Mode::Write), "\n";
Mode $x = Mode::Exec;
try {
    echo Open::file($x as Mode::Read|Mode::Write), "\n";
} catch (RuntimeError $e) {
    echo "threw: ", $e->message, "\n";
}
```
```output
opened 0
opened 1
threw: `2` is not one of `Mode::Read`, `Mode::Write`
```

```nvs error
<?nvs
enum Mode { Read, Write, Exec }

class Open {
    public static function file(Mode::Read|Mode::Write $m): int {
        return $m as int;
    }
}

echo Open::file(Mode::Exec), "\n";
```
```output
expected `Mode::Read|Mode::Write`, found `Mode`
```

# What an enum does not have

A case has no properties and an enum has no methods. So `E::A->value`, `E::A->name`,
`E::from($n)`, `E::tryFrom($n)` and `E::cases()` are compile errors. An enum body has only
cases: a method, a constant or an `implements` clause does not compile. There is no `BackedEnum`
or `UnitEnum`, because every enum is backed by an integer. Write these instead:

- The integer of a case is `E::A as int`, or `E::A as uint`.
- `$n as E` gives the case for an integer, and throws `RuntimeError` when no case matches.
  `$n as ?E` gives `null` when no case matches.
- The name of a case is a `match` in a class of your own.
- The list of every case is a `static` method that returns `array<E>`.
- Behaviour that belongs to an enum goes in a class that takes the enum.

```nvs
<?nvs
enum Level { Debug = 10, Info = 20, Warn = 30 }

class Levels {
    public static function name(Level $l): string {
        return match ($l) {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
        };
    }

    public static function all(): array<Level> {
        return [Level::Debug, Level::Info, Level::Warn];
    }

    public static function parse(string $s): ?Level {
        foreach (Levels::all() as Level $l) {
            if (Levels::name($l) == $s) {
                return $l;
            }
        }
        return null;
    }
}

echo Levels::name(Level::Info), " ", Core\Arr::count(Levels::all()), " ", (Levels::parse("warn") ?? Level::Debug) as int, "\n";
```
```output
info 3 30
```

```nvs error
<?nvs
enum Level { Debug, Info }
echo Level::Info->name, "\n";
```
```output
has no property named `name`
```

```nvs error
<?nvs
enum Level { Debug, Info }
echo Core\Arr::count(Level::cases()), "\n";
```
```output
has no method named `cases`
```

# `Core` enums

The enums under `Core` — `Core\Order`, `Core\RoundMode`, `Core\Weekday` and the rest, each
listed in Part B — are ordinary enums: a case is named `Core\Order::Desc`, passed where a
member asks for it, compared with `==`, and converted with `as int`.

```nvs
<?nvs
array<int> $xs = [3, 1, 2];
foreach (Core\Arr::sort($xs, {order: Core\Order::Desc}) as int $v) {
    echo $v;
}
echo " ", Core\Order::Desc as int, "\n";
```
```output
321 1
```
