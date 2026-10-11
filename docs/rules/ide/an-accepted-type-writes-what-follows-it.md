Accepting a type from a completion list writes `Name::` inside an expression and `Name()` after `new`, and
the name alone everywhere else. Inside an expression a type's name is a receiver and nothing else, so the
item asks the client to open the list again and the static half of that class is on screen without a
keystroke. After `new` the name is a call: the cursor is left between the parentheses, with signature help
open, where a constructor the class declares or inherits takes a parameter, and after them where none
does. `self`, `static` and `parent` are receivers and calls on the same terms, and `parent` writes its `::`
at the start of a statement too, where it opens nothing else.

**The start of a statement writes the name alone**, because `User $u = …` and `User::create()` both start
there and nothing before the name tells them apart. So does every place a type is written — after
`extends`, `implements`, `is`, `as` and `#[`, in a parameter list, in a `catch`, after the `:` of a return
type — and so does a position the tokens do not give away, such as the right of a `<`. A `::` nobody
wanted is deleted by hand and a missing one is two keystrokes, so every doubt is the bare name. A type
alias is never a receiver.

Which of these a cursor is at is read off the lexer's tokens before the name, because a written type is
no node and the name being typed is usually what stops the statement around it from parsing. One token
decides most of it. A `(` and a `,` are read with the bracket they belong to — a `function`, a `fn` or a
`catch` in front of it makes it a list of types — and a `:` opens a return type only after the `)` of such
a list. A `?`, a `|` and a `&` continue a type where one was being written.

**After `new`, only a name `new` compiles on is offered**: no interface, no enum, no alias, no `abstract`
class, and of `Core` the classes the registry names a constructor for. **What follows the cursor is not
written twice** — no `::` in front of one, no `()` in front of a `(`.

The two commands, `editor.action.triggerSuggest` and `editor.action.triggerParameterHints`, are an
editor's and not this server's. A client lists the ones it runs in `capabilities.experimental.commands`
and is sent no other, because a client answers an id it does not know with an error on every accepted
item. A client without snippet support gets `Name()` as plain text. A client that lists no command gets
`Name::` and `Name()` as text and no list, and opens it by typing the member's first letter. There is no
setting that turns any of this off. What it costs is a developer who accepts a class and types `::` from
habit, who writes `Name::::`: no editor types over inserted text. A setting is the answer if that proves
to be a real cost, rather than fewer positions that write `::`.

**Memory:** the workspace index keeps two words on each class declaration, whether it is `abstract` and
how many parameters its own `constructor` declares, for as long as the session runs. The `new` list is
filtered and shaped by them.

**Declined:**
- **`::` everywhere but after `new`.** It is wrong in every written type and at every typed local, such as
  `function f(Foo:: $x)`.
- **Two items per class at the start of a statement**, `User` and `User::`. It doubles the type rows of the
  longest list the server sends.
- **`new Name();` with the semicolon.** It is right only where `new` starts a statement, and the tokens
  cannot tell that from `$x = new Name()` once the `=` is behind the cursor. `new` is usually inside a
  larger expression.
- **A guess from the rest of the line** to decide the semicolon. It is wrong while the first argument of an
  unfinished call is being typed.
- **Sending the command to every client, or adding it in the extension's middleware.** The first sends an
  unknown command id to clients that answer it with an error; the second puts the behaviour where no Rust
  test holds it.
