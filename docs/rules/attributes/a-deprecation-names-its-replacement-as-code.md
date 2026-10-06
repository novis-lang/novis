`#[Core\Deprecated(since:, note:, replace:)]` marks a declaration as retired, and its `replace` is one
Novis expression whose placeholders are the declaration's own parameters and `$this`.

```nvs
class Api {
    #[Core\Deprecated(since: "2.0", note: "`find` also takes a limit.", replace: "$this->find($id, limit: $limit)")]
    public function findById(int $id, int $limit = 10): ?User { return $this->find($id, limit: $limit); }

    public function find(int $id, int $limit): ?User { … }
}

// W1003, with a fix that writes: $user = $api->find($request->id(), limit: 10);
$user = $api->findById($request->id());
```

**The attribute.** It is on the closed `Core` roster of `rule:attributes/attach-sites-and-forms`. Its
fields are `since`, `note` and `replace`, each a string and each optional, and `construct`, a string
allowed on a class only; any other field is refused. It attaches to a class, an interface, an enum, an
enum case, a method, a constructor, a property, a class constant and a parameter.

**The template is checked where it is declared.** `replace` is one expression, or one type name on a
class, an interface or an enum, compiled in the declaration's scope with its parameters and, on an
instance member, `$this` bound. There is no `{0}`, no `$1` and no other template syntax. It is refused
where it is written, each with its own `E08xx` code, when it does not parse, does not compile, has a type
not assignable to the member's (a parameter's template is a named argument that fits the parameter),
names anything less visible than the member, or names anything deprecated — so no fix ever produces a
second warning. A class's replacement declares every public member the class declares with an
assignable signature, and `construct` is a template over the constructor's parameters for a `new`.

**Every use is `W1003`, never an error.** A call, a `new`, a property read or write, a class constant,
an enum case, a passed deprecated parameter and a type position naming a deprecated class each warn. An
override or implementation of a deprecated member warns at the override, with no fix. Nothing warns
inside a deprecated declaration or inside a member of a deprecated class. The message names the member,
then `since` when written, then `note`, then the replacement as written at that use. Only `nvs check
--deny deprecated` (`rule:tooling/a-todo-is-a-comment-the-tools-list`) makes one fail a build.

**The fix fills the template in.** A parameter becomes its argument's source text, matched by position
or name; one left out becomes its default as written at the declaration; `$this` becomes the receiver as
written; a static member's class becomes the class as the use wrote it. A substitution is parenthesized
only where its precedence is lower than its place. A name is resolved at the declaration and written as
the shortest name that resolves to it at the use, with an import added as `E0303`'s fix adds one, and
never an alias (`rule:ide/no-refactoring-introduces-an-alias`).

**Side effects keep their order and count.** An argument or receiver is pure when it holds no call,
`new`, assignment, increment or decrement. A pure one is copied wherever the template uses it, and so is
an impure one used exactly once and in the order the use wrote it. Every other impure one moves to a
`var` line in front of the statement, in evaluation order, under a fresh name made from its parameter's.
Where that line would change when the code runs — the right of `&&`, `||`, `??` or `?:`, a loop's
condition or step, an arrow function, a `match` arm, a default value, a property initializer — and for
a `...$args` spread into a template parameter, there is no fix, and the warning's help shows the
template. Every fix that is offered is `safe`.

A construct the compiler refuses outright is not a deprecation and stays refused. A type alias
carries no attributes and cannot be deprecated.
