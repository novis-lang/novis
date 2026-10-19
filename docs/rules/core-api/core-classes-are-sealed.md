No `extends` or `implements` clause may name a class under `Core`, and the link pass refuses each one
as `E0329` before any type is checked. That covers a class's `extends` and `implements` and an
interface's `extends`, a name written in full or imported with `use`, every registry class, and the
classes of the exception tree that sit under `Core`: `Core\Db\DbError`, `Core\Db\RolledBack`,
`Core\Ldap\LdapError`, `Core\Cli\NotInteractive`, `Core\Test\Failure`, `Core\DeprecatedError` and the
finish marker `Core\Script\Finished`.

What a program's class may name under `Core` is an interface written for it to implement: a contract
such as `Core\Router\Gate`, and the interfaces the derive attributes stand for, `Core\Json\Codec` and
`Core\Db\Codec`. A `Core` interface only `Core` classes implement, such as `Core\Db\Queryable`, is
`E0328` instead. A program's own error class extends one of the global error classes, `Throwable`,
`RuntimeError`, `LogicError` and the rest of the tree, none of which is under `Core`.

A `Core` instance member is native code that reads its receiver's slots by index, against the layout
its own class declares. A program's subclass has a layout of its own, so the member would read the
program's properties as its own: a string's pointer read as an integer, or a slot past the end of the
object. `implements` reaches the same read through the checker, which accepts an implementing class
wherever the `Core` class is expected, and an interface that extends a `Core` class passes that on.
This is a memory-safety question, so it is settled where the program is written rather than left to
each member. The exception classes under `Core` follow the same rule, so there is no list of safe
`Core` parents to remember. A program that wants a narrower error catches the `Core` one and throws
its own `RuntimeError` subclass with the caught one as `previous`.

`nvs_stdlib::instance::receiver` checks the class too: a receiver whose descriptor is not the member's
own class is the same fatal error a receiver that is not an object is. That catches any path the link
pass does not see, such as a `mixed` value a member reads as its own class. `Core\Html::later` splices
text only from a `Core\Html\Markup` its callable returned, and an ordering of a `Core` object against an
object of another class throws the ordinary "no natural order" error before any `compareTo` is called.
The check is a binary search of the `Core` descriptors by address and one name comparison per call on a
`Core` instance member. It costs no memory per call.

**Declined:**
- **A marker that opens chosen `Core` classes to subclassing.** No `Core` class is built to be a
  parent: its members read slots, and a subclass would need a second layout contract that every
  member keeps. A class of the program's own with a property that holds the `Core` value covers the
  same need.
