A `type` alias is also a member of a class, interface or enum body, taking no visibility modifier,
reached as `Owner::Name` from anywhere and as a bare `Name` inside its owner's own body, and never
inherited.

```nvs
final class Order {
    type Meta = {total: decimal, note?: string};

    public function meta(): Meta { … }          // the short form, inside the owner
}

function show(Order::Meta $m): void { … }       // the qualified form, anywhere
```

Everything `rule:types/type-alias` says about an alias holds here unchanged: it is transparent in
both directions, it is erased before codegen, a cycle is a diagnostic, and it may not name one bare
class-shaped atom (`rule:types/alias-is-never-a-bare-class`). The member is accepted in a class, an
interface and an enum body alike — an interface's alias is not a contract an implementor satisfies,
and an enum's alias has nothing to do with its cases.

- **No visibility, ever.** A modifier run or an attribute group written in front of a body's `type`
  is `E0133`. Visibility restricts reaching a name a running program has, and an alias has none; a
  `private` alias would hide the name while leaving the type it expands to writable by anyone.
- **No inheritance.** `Sub::Name`, where only an ancestor of `Sub` declares `Name`, is `E0405`
  naming the owner that does. A class constant is inherited because a subclass genuinely has one; an
  alias is an entry on nothing, and inheriting it would give one type as many names as its owner has
  descendants — what `rule:statements/nothing-gets-a-second-name` closes.
- **Two spellings and no third.** `self::Name` and `static::Name` in type position are not spellings
  of this member and are `E0135` where they are written, as `parent::Name` is
  (`rule:types/grammar`). An alias is resolved before there is a receiver, so `static::` could only
  ever mean the lexical class, which the bare `Name` already says.
- **A name is one thing.** A body's alias sharing a name with a class constant, an enum case or an
  enum the same body declares (`rule:enums/class-scoped-enum`) is `E0304` at the later of the two
  declarations. `Owner::Name` in type position is therefore read as an alias, then an owned enum, then
  an enum case, then a class constant, and no program that compiles depends on that order — it exists
  so the diagnostic for a name that resolves to nothing can say what was looked for.

The member exists so a type that describes one class's data is named by its owner, as that class's
constants and methods are, and not by an owner prefix written by hand into a file-scope name
(`OrderMeta`, `InvoiceMeta`). `nvs meta --json` lists it inside its owner's entry, and the file-scope
roster holds only file-scope aliases. Its declaration is keyed by the owner's `QName` and the member
name, never by a namespace path: `Ns\Order\Meta` is also the spelling of a class `Meta` in namespace
`Ns\Order`, and the two must not share a key.

- **Memory:** nothing per request, because nothing about the member survives the checker and the
  compiled program is the one the file-scope form produces. At compile time it costs one alias-table
  entry per declaration, the same entry the file-scope form pays for.

**Declined:**
- **Aliases at file scope only.** Every program would pay for it with owner prefixes spelled by hand,
  to save one member kind in the parser.
- **An owner-scoped alias that is a distinct type from its expansion.** That is a newtype, and an alias
  is transparent (`rule:types/type-alias`).
- **A parametric `type Rows<T> = …` in a body.** It is out of scope at both sites while user-defined
  generics are (`rule:types/type-alias`).
