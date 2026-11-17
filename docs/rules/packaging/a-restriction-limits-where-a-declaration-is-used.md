A `[[restrict]]` entry in a `package.toml` names one declaration and the places that may use it, and a
use anywhere else is a **compile error** that says where the declaration is allowed, why, and which
file and line made the rule.

```toml
[[restrict]]
use     = 'Core\Router::urlSigned'
allow   = ['Framework\Core\Signing\*']
note    = "Use `Framework\Core\Url::signed`. It adds the framework's key rotation."
replace = 'Framework\Core\Url::signed($route, $params)'
```

**The entry.** `use` is required and names a class, interface or enum (`'Ns\Name'`), a method,
property, class constant or enum case (`'Ns\Name::member'`), or a namespace: `'Ns'` for its own
declarations and `'Ns\*'` for every depth below, as `rule:security/longest-prefix-wins` writes a
subtree. Novis has no top-level function, so a method is the only callable a `use` names. `allow`
is required and lists the places that may use it. An entry matches a use when it names the use's
enclosing namespace (exact, or a `\*` subtree), its enclosing class, or its enclosing
`Class::method`. `allow = []` allows no place. `note` is optional Markdown, shown in the error and on
hover, and is text an end user reads. `replace` is optional, one Novis expression with the target's
parameters and `$this` as its placeholders, exactly as `#[Core\Deprecated]`'s `replace` is
(`rule:attributes/a-deprecation-names-its-replacement-as-code`), and only on a method target. An
unknown field is an error
(`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`).

**Where it is read.** The application's `package.toml`, the one the grant table is read from, and
every package's own, the one `rule:packaging/a-package-file-stays-under-its-declared-prefix` places
a package's files under. A hand-placed folder with a named `package.toml` counts exactly as a fetched
package does. A package's table holds once one of its files is compiled into the program. An
`autoload` line alone loads nothing, so a package the program never uses restricts nothing, and an
entry inside a package's own folder reads its `package.toml` as the application's file, matched by
namespace alone.

**What is checked where it is written** (`E0665`, at the entry's line): a `use` or an `allow` entry
that is not a declaration name, a `\*` anywhere but at the end, a field `nvs` does not know, a
`replace` on a target that is not a method or a function, and, in a package's file, an `allow` entry
that does not lie under the package's own name. A `Core` target that names no `Core` declaration is
`E0665` too, since `Core` is always known. Any other target that names no declaration of the program
is `W1031` in the application's file and silent in a package's, where it often names code the
current program does not load. A `use` that names a method which overrides or implements another one
is `E0665` with the help "restrict `Parent::method` instead". A `replace` that does not compile is
`E0666`.

**The replacement is compiled where it is written.** Before any body is checked, each `replace` is
inferred in a copy of its `package.toml` in which it is the only code, so an error inside it points
at its bytes in the file. Its scope is the target method's parameters and, on an instance method,
`$this`, in the namespace of the file the table is in: the package's name, or the global namespace
in the application's file. It is outside every class. It is `E0666` when it does not parse, does not
compile, returns a type that does not fit the method's, holds an escape sequence (its text must be
the text between its quotes), or uses a declaration its own entry restricts. Only the declarations
the program loads are known: a name in it that the program does not load is checked when a program
that loads it compiles, such as the one the fix writes. `E0664` at a use carries the replacement
filled in as its fix, through the same filling as a deprecation's
(`rule:attributes/a-deprecation-names-its-replacement-as-code`), and a parameter's default is not
read, so a left-out parameter the template uses leaves the help and no fix.

**What counts as a use.** A call (static, instance, `parent::`), a method reference
(`Name::method`), `new`, a property read or write, a class constant, an enum case, and a
reflective act on the member. A type position, an `is` test and `catch` are not uses, so allowed
code can pass a restricted object through typed signatures. A restriction on a method holds for
every override and implementation of it, so a call through a subclass or an interface meets the same
check. An entry on a class holds every use of its members too, and an entry on a namespace every
use of its classes and their members, so `$object->method()` on a restricted class is refused
outside the allowed places. A use inside the restricted declaration itself, its own class or its own
namespace, is always allowed, so restricted code can call itself. A use outside every allowed place
is `E0664`.

**It only narrows.** A use must pass every restriction on the declaration, from the application's
file and from every loaded package. Nothing lifts one: the application cannot remove, widen or
override a package's entry, and a package's entry holds for the application's code and every other
package's. A package's `allow` lies under its own name, and a use matches it only when the using file
is inside that package by layout, so a module file that writes `namespace Framework\Core\Signing;`
is still refused. The application's own entries match by namespace alone, since the application has
no package folder. A package that adds a restriction, or allows fewer places, makes a breaking change
(`rule:packaging/only-a-major-may-break-the-api`).

**When it runs.** Each body's check ends with a pass over the expression table entries the body
recorded, the typed tree's record of what each call, `new`, property, constant and case resolved
to. It is the same pass that warns about a deprecated use (`crates/nvs-types/src/restrict.rs`), so
no side list is kept and every use kind the deprecation warning sees is held here too. Each entry
looks up a hash map of restricted declarations, and a method use also walks its ancestors. With no
`[[restrict]]` anywhere the index is `None` and the pass returns at once. The tables fold into
every unit's cache key beside the grants (`rule:config/the-extension-set-is-in-every-unit-key`), so an
edited entry is an ordinary cache miss, and a program with no table keeps the key it had.

**A reflective act is held at run time, by a list made at compile time.** `Core\Reflect\ClassInfo`'s
`call`, `get`, `set`, `constant` and `construct` name their member in a string, so the checker
cannot see which one. When a body holds such a call, the same pass lists every act on a restricted
member that the call's place does not allow, each with `E0664`'s text, and the call-site constant
those members already take carries the list (`nvs_runtime::source::encode_restricted`). The member
throws a `LogicError` with that text when its act is on the list, the class a refused reflective
`call` of a `private` method throws. A member is listed under the class the act is made on, so an
inherited one is held under each class that inherits it, by the entries a use of it there holds.
A callable reference to one of these members, `$info->call(...)`, runs wherever it is later called,
so its list names every restricted act, allowed anywhere or not.

**A call or property access through a `mixed` or `object` receiver is held the same way.** Its
class is known only when it runs, so the checker lists, for that one name and that kind of act, every
restricted member its place does not allow. The compiler emits a check of the receiver's class
before the access (`Helper::RestrictedErasedAct`), and it throws the same `LogicError`. A name no
entry restricts gets no list and no check.

**A keyed access, `$object->$key`, is a use of every property its key may name.** The key is a
`property<T>`, which ranges over `T`'s public properties, so the access does not compile where any
of them is restricted and not allowed, whichever name the key holds when it runs. It is a compile
error, never a run-time check, so it costs nothing per request. Declined: a check of the key's value
when it runs. It would cost a branch per access and hold a list per site, and fail only when the
line runs, for a form that a class with a restricted property rarely needs.

**The error** says first what is not allowed, then where it is allowed, then the note, then the
source: "`Core\Router::urlSigned` cannot be used here. It is allowed only in
`Framework\Core\Signing`. Use `Framework\Core\Url::signed`. It adds the framework's key rotation.
This rule is in `modules/Framework/package.toml`, line 4."

**The editor** reads the same tables, from the program's `package.toml` and each loaded package's,
and checks with them and with no grant key, so it shows `E0664` and never `E0658`. The error's
message carries where the use is allowed and the note, and a related location links the entry's
line. The fix is the entry's `replace`. A hover on a restricted declaration names where it is
allowed, the note, and the entry's file relative to the program's directory and its line. A
completion item the cursor's namespace and class are not allowed to use sorts last, with that text
in its `detail`; the method around the cursor is not read there. Every `package.toml` is watched,
and an edit to one an open document's analysis read checks that document again
(`crates/nvs-lsp/src/document.rs` `analyse_through`).

**Cost:** nothing per request. A reflective act reads one header word of its call-site constant, and
scans that site's list only in a program with a table. At compile time, one read of each table and
one hash-set lookup per resolved use when a table exists, and, on the first reflective call, one walk
of the classes an entry names. **Memory:** the tables are compile-time input, held once per compiled
program, and nothing of them is held per request. Each reflective call site in a program with a
table carries its list in the unit's data, and so does each erased access to a restricted name, which
pays one helper call and one scan of that list each time it runs. `crates/nvs-config/src/restrict.rs` reads the tables.

**Declined:**
- **A map form, like `[grants]`** (`'Core\Router::urlSigned' = [...]`). It has no room for `note`
  and `replace`, and an error without its reason is what the table exists to avoid.
- **An attribute on the allowed class.** It holds only when that class is compiled, and lets any
  pasted file claim a `Core` member.
- **A table in `nvs.toml`.** That file is the operator's and scoped to a deployment, and a
  restriction is the code author's statement about their own code.
- **A runtime form through `Config::set`.** It costs a branch per call, fails only when the line
  runs, and the editor cannot see it.
- **A warning instead of an error.** A warning is what a framework author is trying to get past.
- **One table of the unit's restrictions, matched at run time against the calling place.** A
  package's entry matches only a file inside the package by layout, which the run time cannot see,
  so the match would be a second copy of the checker's, kept in step by hand. The per-site list is
  the checker's own answer, made once.
