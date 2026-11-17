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

**The error** says first what is not allowed, then where it is allowed, then the note, then the
source: "`Core\Router::urlSigned` cannot be used here. It is allowed only in
`Framework\Core\Signing`. Use `Framework\Core\Url::signed`. It adds the framework's key rotation.
This rule is in `modules/Framework/package.toml`, line 4." The editor shows the same text at the use.

**Cost:** nothing at run time and nothing per request, apart from one table lookup per reflective act
on a member. At compile time, one read of each table and one hash-set lookup per resolved use when a
table exists. **Memory:** the tables are compile-time input, held once per compiled program, and
nothing of them is held per request. `crates/nvs-config/src/restrict.rs` reads them.

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
