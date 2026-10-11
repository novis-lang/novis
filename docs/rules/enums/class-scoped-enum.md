An `enum` is also a member of a class or interface body, taking no visibility modifier, reached as
`Owner::Name` and `Owner::Name::Case` from anywhere and as a bare `Name` and `Name::Case` inside its
owner's own body, and never inherited.

```nvs
namespace Shop;

final class Order {
    enum Status { Open, Paid, Cancelled }

    public Status $status = Status::Open;              // the short form, inside the owner

    public function pay(): void { $this->status = Status::Paid; }
}

final class Receipt {
    public static function label(Order::Status $s): string {   // the qualified form, anywhere
        return $s == Order::Status::Paid ? "paid" : "unpaid";
    }
}
```

Everything else about an enum holds unchanged: its body (`rule:enums/declaration`), its one backing
type, its closed integer cases (`rule:enums/closed-integer-type`), its zero-byte representation, and
every binding site it types (`rule:statements/an-enum-name-is-a-type-everywhere`). The member
only changes where the declaration is written and how its name is spelled. A case is still an integer
inlined where it is written, so `Order::Status::Paid` compiles to the same constant a file-scope
`Status::Paid` does. In type position, `Order::Status::Paid` is the case's own type
(`rule:types/enum-case-type`).

- **Two places, and no third.** An enum is declared at file scope or in a class or interface body. In
  an enum body it is `E0220`, whose help names the class body. In a method body, a hook, a closure or a
  block it stays `E0233` (`rule:classes/a-type-is-declared-at-file-scope`). A class or an interface in
  a class body stays refused: it is one `E0233` where it is written.
- **No visibility, ever.** A modifier run in front of a body's `enum` is `E0133`. Its cases leave the
  class in every return value and every property, so a `private` enum would hide the name and not the
  values. An attribute group may stand in front of it, as in front of a file-scope enum.
- **No inheritance.** `Sub::Status`, where only an ancestor of `Sub` declares `Status`, is `E0405`
  in a type and `E0309` in a value, each naming the class that does. A subclass that inherited the name would give one enum as many names as
  its owner has descendants (`rule:statements/nothing-gets-a-second-name`).
- **Two spellings and no third.** `self::Status` and `static::Status` are `E0135` where they are
  written, as they are for a class-scoped alias (`rule:types/class-scoped-alias`). An enum is resolved
  before there is a receiver, so `static::` could only mean the lexical class, which the bare `Status`
  already says.
- **A name is one thing.** An owner's enum, constant and alias never share a name: the later of the
  two declarations is `E0304`.
- **No import of a member.** `use Shop\Order::Status;` is a diagnostic where it is written. A program
  imports the owner and writes `Order::Status`. A member import would give one enum a second short
  name.

**The name a program sees** is the owner's qualified name, `::`, and the member's name:
`Shop\Order::Status`. `Core\Reflect\EnumInfo::name`, a diagnostic, `nvs meta` and the error a route
capture throws all print it. `Core\Reflect\EnumInfo::of(Order::Status::class)` reads it back. The
declaration is keyed by the owner's `QName` and the member name, never by a namespace path, for the
reason `rule:types/class-scoped-alias` gives: `Shop\Order\Status` is also the spelling of a class
`Status` in namespace `Shop\Order`, and the two are two types with two keys.

The member exists so an enum that describes one class's state is named by its owner, as that class's
constants, aliases and methods are, and not by an owner prefix written by hand into a file-scope name
(`OrderStatus`, `InvoiceStatus`). Where an enum goes is `rule:enums/an-enum-lives-in-the-class-that-uses-it`.

What it trades. Performance: nothing at run time, because a case is still an inlined integer and the
name is resolved before any code runs. Usability: a class's enums are read with it, and a name stops
carrying its owner's word. Simplicity: a second place to declare an enum and a three-part value form,
both resolved by the checker.

- **Memory:** nothing per request, because the compiled program is the one the file-scope form
  produces. At compile time it costs one enum-table entry per declaration, the same entry a
  file-scope enum pays for, and one runtime name string per enum, as before.

**Declined:**
- **Enums at file scope only.** Every program would pay for it with owner prefixes spelled by hand,
  to save one member kind in the parser.
- **`use Shop\Order::Status;`.** It gives one enum a second short name
  (`rule:statements/nothing-gets-a-second-name`), and importing the owner already costs one segment.
- **A `private` or `protected` enum.** The cases are values that leave the class, so a modifier hides
  a name and nothing else.
- **Nested classes and interfaces alongside.** A class has members, layout and dispatch of its own,
  and none of the static tables has a reading for one inside another. An enum is a closed list of
  integers and has none of that.
- **An enum in an enum body.** An enum body is a list of cases (`rule:enums/no-class-machinery`), and
  a case list owning another case list says nothing a namespace does not.
- **The old file-scope `Core` names kept beside the new ones.** That is a second name for each
  enum, and there is no release that a program could have been written against.
